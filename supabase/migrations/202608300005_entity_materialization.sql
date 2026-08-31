-- MasterChef Phase 2 completion: entity materialization
-- AFTER INSERT trigger on sync_operations fans accepted operations out into
-- business tables. Idempotent by stable operation/entity ids. Every branch is
-- exception-guarded: a malformed payload never rejects the sync operation
-- itself; it is skipped and remains auditable in sync_operations.

create or replace function public.sync_safe_uuid(t text)
returns uuid language plpgsql immutable as $$
begin
  if t is null or length(trim(t)) < 32 then return null; end if;
  return t::uuid;
exception when others then return null;
end $$;

create or replace function public.sync_safe_bigint(v jsonb)
returns bigint language plpgsql immutable as $$
begin
  if v is null or jsonb_typeof(v) <> 'number' then return null; end if;
  return (v#>>'{}')::bigint;
exception when others then return null;
end $$;

create or replace function public.sync_safe_ts(v jsonb)
returns timestamptz language plpgsql immutable as $$
begin
  if v is null then return null; end if;
  if jsonb_typeof(v) = 'number' then return to_timestamp((v#>>'{}')::bigint); end if;
  return (v#>>'{}')::timestamptz;
exception when others then return null;
end $$;

create or replace function public.materialize_sync_operation()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
declare
  p jsonb := new.payload;
begin
  -- ORDERS (restaurant app order lifecycle)
  if new.entity_type = 'ORDER' then
    begin
      insert into public.restaurant_orders
        (id, order_number, order_type, table_id, customer_name, customer_phone,
         waiter_id, status, subtotal_minor, discount_minor, total_minor, currency, notes,
         created_at, updated_at)
      values
        (public.sync_safe_uuid(p->>'id'),
         coalesce(public.sync_safe_bigint(p->'order_number'), 0),
         coalesce(p->>'order_type', 'TAKEAWAY'),
         public.sync_safe_uuid(p->>'table_id'),
         p->>'customer_name', p->>'customer_phone',
         public.sync_safe_uuid(p->>'waiter_id'),
         coalesce(p->>'status', 'DRAFT'),
         coalesce(public.sync_safe_bigint(p->'subtotal_minor'), 0),
         coalesce(public.sync_safe_bigint(p->'discount_minor'), 0),
         coalesce(public.sync_safe_bigint(p->'total_minor'), 0),
         coalesce(p->>'currency', 'UGX'), p->>'notes',
         coalesce(public.sync_safe_ts(p->'created_at'), now()),
         coalesce(public.sync_safe_ts(p->'updated_at'), now()))
      on conflict (id) do update set
        status = excluded.status,
        subtotal_minor = excluded.subtotal_minor,
        discount_minor = excluded.discount_minor,
        total_minor = excluded.total_minor,
        notes = excluded.notes,
        updated_at = excluded.updated_at;
    exception when others then null;
    end;

  elsif new.entity_type = 'ORDER_ITEM' then
    begin
      insert into public.restaurant_order_items
        (id, order_id, menu_item_id, name_snapshot, unit_price_minor, quantity,
         total_minor, station, status, notes, created_at, updated_at)
      values
        (public.sync_safe_uuid(p->>'id'),
         public.sync_safe_uuid(p->>'order_id'),
         public.sync_safe_uuid(p->>'menu_item_id'),
         coalesce(p->>'name_snapshot', 'unknown'),
         coalesce(public.sync_safe_bigint(p->'unit_price_minor'), 0),
         greatest(coalesce(public.sync_safe_bigint(p->'quantity'), 1), 1),
         coalesce(public.sync_safe_bigint(p->'total_minor'), 0),
         coalesce(p->>'station', 'KITCHEN'),
         case coalesce(p->>'status', 'ORDERED')
           when 'PENDING' then 'ORDERED'
           when 'SENT_TO_KITCHEN' then 'ORDERED'
           when 'IN_PREPARATION' then 'PREPARING'
           else coalesce(p->>'status', 'ORDERED')
         end,
         p->>'notes',
         coalesce(public.sync_safe_ts(p->'created_at'), now()),
         coalesce(public.sync_safe_ts(p->'updated_at'), now()))
      on conflict (id) do update set
        status = excluded.status,
        total_minor = excluded.total_minor,
        updated_at = excluded.updated_at;
    exception when others then null;
    end;

  -- WORKFORCE (epoch-second BIGINT schema, TEXT ids, payload_hash required)
  elsif new.entity_type = 'ATTENDANCE' then
    begin
      insert into public.workforce_attendance
        (id, worker_id, device_id, clock_in_at, clock_out_at, break_minutes, status, notes,
         payload_hash, created_at, updated_at)
      values
        (coalesce(p->>'id', new.id::text),
         coalesce(p->>'worker_id', 'unknown'),
         coalesce(p->>'device_id', new.device_id::text),
         coalesce(public.sync_safe_bigint(p->'clock_in_at'), extract(epoch from now())::bigint),
         public.sync_safe_bigint(p->'clock_out_at'),
         coalesce((p->>'break_minutes')::int, 0),
         coalesce(p->>'status', 'OPEN'),
         p->>'notes', new.payload_hash,
         coalesce(public.sync_safe_bigint(p->'created_at'), extract(epoch from now())::bigint),
         coalesce(public.sync_safe_bigint(p->'updated_at'), extract(epoch from now())::bigint))
      on conflict (id) do update set
        clock_out_at = excluded.clock_out_at,
        break_minutes = excluded.break_minutes,
        status = excluded.status,
        notes = excluded.notes,
        updated_at = excluded.updated_at;
    exception when others then null;
    end;

  elsif new.entity_type = 'SHIFT' then
    begin
      insert into public.workforce_shifts
        (id, worker_id, role, starts_at, ends_at, location, created_by, payload_hash, created_at, updated_at)
      values
        (coalesce(p->>'id', new.id::text),
         coalesce(p->>'worker_id', 'unknown'),
         coalesce(p->>'role', 'WAITER'),
         coalesce(public.sync_safe_bigint(p->'starts_at'), 0),
         coalesce(public.sync_safe_bigint(p->'ends_at'), 0),
         coalesce(p->>'location', 'MAIN'),
         coalesce(p->>'created_by', 'unknown'), new.payload_hash,
         coalesce(public.sync_safe_bigint(p->'created_at'), extract(epoch from now())::bigint),
         coalesce(public.sync_safe_bigint(p->'updated_at'), extract(epoch from now())::bigint))
      on conflict (id) do nothing;
    exception when others then null;
    end;

  -- STOCK (requires uuid item ids; name-only payloads skip via exception guard)
  elsif lower(new.entity_type) = 'stock_movement' then
    begin
      insert into public.stock_movements
        (id, item_id, movement_type, quantity, unit, from_location, to_location,
         unit_cost_minor, source_id, reason, actor_id, device_id, created_at)
      values
        (public.sync_safe_uuid(p->>'id'),
         public.sync_safe_uuid(p->>'item_id'),
         coalesce(new.operation, 'CREATE'),
         coalesce((p->>'quantity')::numeric, 0),
         coalesce(p->>'unit', 'PCS'),
         p->>'location', p->>'to_location',
         public.sync_safe_bigint(p->'unit_cost_minor'),
         public.sync_safe_uuid(p->>'source_id'),
         coalesce(p->>'reason', p->>'supplier_reference'),
         public.sync_safe_uuid(p->>'actor_id'),
         new.device_id::text,
         coalesce(public.sync_safe_ts(p->'created_at'), now()))
      on conflict (id) do nothing;
    exception when others then null;
    end;

  -- EXPENSES (actor attribution required by NOT NULL created_by)
  elsif lower(new.entity_type) = 'expense' then
    begin
      insert into public.expenses
        (id, category, amount_minor, currency, status, description, reference,
         created_by, created_at, updated_at)
      values
        (public.sync_safe_uuid(p->>'id'),
         coalesce(p->>'category', 'OTHER'),
         coalesce(public.sync_safe_bigint(p->'amount_minor'), 0),
         coalesce(p->>'currency', 'UGX'),
         case coalesce(p->>'status', 'SUBMITTED')
           when 'PENDING_APPROVAL' then 'SUBMITTED'
           else coalesce(p->>'status', 'SUBMITTED')
         end,
         coalesce(p->>'description', 'synced expense'),
         p->>'reference',
         public.sync_safe_uuid(p->>'actor_id'),
         coalesce(public.sync_safe_ts(p->'created_at'), now()),
         now())
      on conflict (id) do nothing;
    exception when others then null;
    end;

  -- EQUIPMENT events
  elsif lower(new.entity_type) = 'equipment_event' then
    begin
      insert into public.equipment_events
        (id, event_type, actor_id, reason, created_at)
      values
        (public.sync_safe_uuid(p->>'id'),
         coalesce(p->>'condition', 'NOTE'),
         public.sync_safe_uuid(p->>'actor_id'),
         concat_ws(' — ', p->>'asset_tag', p->>'description', p->>'notes'),
         coalesce(public.sync_safe_ts(p->'created_at'), now()))
      on conflict (id) do nothing;
    exception when others then null;
    end;
  end if;

  return new;
end $$;

drop trigger if exists sync_operations_materialize on public.sync_operations;
create trigger sync_operations_materialize
  after insert on public.sync_operations
  for each row execute function public.materialize_sync_operation();

-- Entities intentionally not yet materialized (no cloud table exists):
-- payment, cash_session, refund, waste_event — they remain fully auditable in
-- sync_operations until their tables ship in a later release.
