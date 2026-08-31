-- MasterChef Phase 10: CEO operational KPI aggregation
-- Read-only rollup over synced cloud truth. Callable only by the service role
-- (edge functions); no direct client access.
create or replace function public.ceo_daily_kpis()
returns jsonb
language sql
security definer
set search_path = public
stable
as $$
  with day_start as (select date_trunc('day', now()) as ts),
  ep as (
    select extract(epoch from (select ts from day_start))::bigint as start_epoch,
           extract(epoch from (select ts from day_start) + interval '1 day')::bigint as end_epoch
  ),
  sales as (
    select coalesce(sum(total_minor),0)::bigint as sales_minor, count(*)::bigint as order_count
    from restaurant_orders, day_start
    where created_at >= day_start.ts and status <> 'CANCELLED'
  ),
  delayed as (
    select count(*)::bigint as c from restaurant_orders
    where status in ('SUBMITTED','PREPARING') and created_at < now() - interval '20 minutes'
  ),
  exp as (
    select coalesce(sum(amount_minor),0)::bigint as expense_minor
    from expenses, day_start
    where created_at >= day_start.ts and status in ('SUBMITTED','APPROVED')
  ),
  present as (
    select count(distinct worker_id)::bigint as c from workforce_attendance where status = 'OPEN'
  ),
  scheduled as (
    select count(distinct worker_id)::bigint as c from workforce_shifts, ep
    where starts_at < ep.end_epoch and ends_at >= ep.start_epoch and coalesce(status,'') <> 'CANCELLED'
  ),
  low as (
    select i.name, i.unit, i.reorder_level,
      coalesce(sum(case when sm.to_location is not null then sm.quantity else 0 end),0)
      - coalesce(sum(case when sm.from_location is not null then sm.quantity else 0 end),0) as on_hand
    from inventory_items i
    left join stock_movements sm on sm.item_id = i.id
    where i.active
    group by i.id, i.name, i.unit, i.reorder_level
    having coalesce(sum(case when sm.to_location is not null then sm.quantity else 0 end),0)
         - coalesce(sum(case when sm.from_location is not null then sm.quantity else 0 end),0) <= i.reorder_level
  )
  select jsonb_build_object(
    'generated_at', now(),
    'sales_today_minor', (select sales_minor from sales),
    'orders_today', (select order_count from sales),
    'delayed_orders', (select c from delayed),
    'expenses_today_minor', (select expense_minor from exp),
    'workers_present', (select c from present),
    'workers_scheduled_today', (select c from scheduled),
    'low_stock_items', coalesce((select jsonb_agg(jsonb_build_object('item',name,'on_hand',on_hand,'reorder_level',reorder_level,'unit',unit)) from low), '[]'::jsonb)
  );
$$;

revoke all on function public.ceo_daily_kpis() from public;
revoke all on function public.ceo_daily_kpis() from anon, authenticated;
grant execute on function public.ceo_daily_kpis() to service_role;
