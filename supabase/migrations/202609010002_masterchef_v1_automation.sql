-- V1 automation: completion of a sale consumes its recipe ingredients exactly once.
create table if not exists public.order_stock_consumption (
 id uuid primary key default gen_random_uuid(),
 order_id uuid not null references public.restaurant_orders(id) on delete cascade,
 order_item_id uuid not null references public.restaurant_order_items(id) on delete cascade,
 recipe_id uuid not null references public.recipe_headers(id),
 quantity numeric(18,6) not null check(quantity > 0),
 created_at timestamptz not null default now(),
 unique(order_item_id,recipe_id)
);
create index if not exists order_stock_consumption_order on public.order_stock_consumption(order_id);

create or replace function public.consume_order_recipe_stock() returns trigger language plpgsql as $$
declare it record; rl record; consume_qty numeric(18,6); movement_id uuid;
begin
 if new.status='COMPLETED' and old.status is distinct from 'COMPLETED' then
   for it in select oi.*, rh.id recipe_id, rh.yield_quantity from public.restaurant_order_items oi join public.recipe_headers rh on rh.menu_item_id=oi.menu_item_id and rh.active=true where oi.order_id=new.id and oi.status <> 'CANCELLED' loop
     for rl in select * from public.recipe_lines where recipe_id=it.recipe_id loop
       consume_qty := (rl.quantity / it.yield_quantity) * it.quantity * (1 + rl.waste_percent/100);
       if not exists(select 1 from public.order_stock_consumption osc where osc.order_item_id=it.id and osc.recipe_id=it.recipe_id) then
         insert into public.stock_movements(item_id,movement_type,quantity,unit,from_location,to_location,source_id,reason,actor_id,device_id)
         values(rl.inventory_item_id,'ISSUE',consume_qty,rl.unit,'STORE','PRODUCTION',it.id,'Automatic recipe consumption for completed order',new.waiter_id,'cloud-order-completion') returning id into movement_id;
         insert into public.order_stock_consumption(order_id,order_item_id,recipe_id,quantity) values(new.id,it.id,it.recipe_id,consume_qty);
       end if;
     end loop;
   end loop;
 end if;
 return new;
end $$;

drop trigger if exists trg_consume_order_recipe_stock on public.restaurant_orders;
create trigger trg_consume_order_recipe_stock after update of status on public.restaurant_orders for each row execute function public.consume_order_recipe_stock();

create or replace function public.calculate_menu_item_cost(p_menu_item uuid) returns bigint language sql stable as $$
 select coalesce(sum(round((rl.quantity/rh.yield_quantity)*(1+rl.waste_percent/100)*coalesce((select sp.unit_cost_minor from public.supplier_prices sp where sp.item_id=rl.inventory_item_id and (sp.effective_to is null or sp.effective_to>now()) order by sp.effective_from desc limit 1),0))),0)::bigint
 from public.recipe_headers rh join public.recipe_lines rl on rl.recipe_id=rh.id where rh.menu_item_id=p_menu_item and rh.active=true;
$$;

create or replace function public.refresh_product_cost(p_menu_item uuid,p_actor uuid) returns void language plpgsql as $$
declare rid uuid; c bigint; price bigint; margin bigint; pct numeric(10,4);
begin
 select id into rid from public.recipe_headers where menu_item_id=p_menu_item and active=true order by version desc limit 1;
 c:=public.calculate_menu_item_cost(p_menu_item);
 select current_price_minor into price from public.menu_items where id=p_menu_item;
 margin:=coalesce(price,0)-c; pct:=case when coalesce(price,0)=0 then 0 else round((margin::numeric/price::numeric)*100,4) end;
 insert into public.product_cost_snapshots(menu_item_id,recipe_id,ingredient_cost_minor,waste_cost_minor,total_cost_minor,selling_price_minor,gross_margin_minor,margin_percent)
 values(p_menu_item,rid,c,0,c,coalesce(price,0),margin,pct);
end $$;

-- Low-stock alert generation. It derives current quantity from the immutable movement ledger.
create or replace function public.generate_low_stock_notifications() returns integer language plpgsql as $$
declare r record; n integer:=0;
begin
 for r in select ii.id,ii.name,ii.reorder_level,coalesce(sum(case when sm.movement_type in ('OPENING','PURCHASE_RECEIPT','RETURN','COUNT_ADJUSTMENT') then sm.quantity when sm.movement_type in ('ISSUE','WASTAGE','DAMAGE','EXPIRY') then -sm.quantity else 0 end),0) qty from public.inventory_items ii left join public.stock_movements sm on sm.item_id=ii.id where ii.active=true group by ii.id,ii.name,ii.reorder_level having coalesce(sum(case when sm.movement_type in ('OPENING','PURCHASE_RECEIPT','RETURN','COUNT_ADJUSTMENT') then sm.quantity when sm.movement_type in ('ISSUE','WASTAGE','DAMAGE','EXPIRY') then -sm.quantity else 0 end),0) <= ii.reorder_level loop
   if not exists(select 1 from public.operational_notifications onf where onf.entity_type='INVENTORY_ITEM' and onf.entity_id=r.id and onf.resolved_at is null) then
     insert into public.operational_notifications(severity,category,title,message,entity_type,entity_id) values('WARNING','INVENTORY','Low stock: '||r.name,'Current quantity '||r.qty||' is at or below reorder level '||r.reorder_level,'INVENTORY_ITEM',r.id); n:=n+1;
   end if;
 end loop; return n;
end $$;
