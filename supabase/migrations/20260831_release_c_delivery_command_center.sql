-- Release C: delivery, reconciliation and CEO operational command center
create extension if not exists pgcrypto;

create table if not exists public.deliveries (
 id uuid primary key default gen_random_uuid(), order_id uuid not null unique references public.restaurant_orders(id),
 rider_id uuid, status text not null default 'PENDING' check(status in ('PENDING','ASSIGNED','PICKED_UP','OUT_FOR_DELIVERY','DELIVERED','FAILED','CANCELLED','RETURNED')),
 address text not null, customer_name text, customer_phone text,
 collection_method text check(collection_method in ('CASH','MTN_MOBILE_MONEY','AIRTEL_MONEY','PREPAID')),
 amount_due_minor bigint not null check(amount_due_minor >= 0), amount_collected_minor bigint not null default 0 check(amount_collected_minor >= 0),
 proof_reference text, failure_reason text, created_at timestamptz not null default now(), updated_at timestamptz not null default now(),
 check(amount_collected_minor <= amount_due_minor)
);
create index if not exists deliveries_status_time on public.deliveries(status,updated_at desc);
create index if not exists deliveries_rider_status on public.deliveries(rider_id,status);

create table if not exists public.delivery_events (
 id uuid primary key default gen_random_uuid(), delivery_id uuid not null references public.deliveries(id) on delete cascade,
 event_type text not null, actor_id uuid not null, metadata jsonb not null default '{}'::jsonb, created_at timestamptz not null default now()
);
create index if not exists delivery_events_delivery_time on public.delivery_events(delivery_id,created_at);

create table if not exists public.delivery_collections (
 id uuid primary key default gen_random_uuid(), delivery_id uuid not null references public.deliveries(id),
 method text not null check(method in ('CASH','MTN_MOBILE_MONEY','AIRTEL_MONEY','PREPAID')),
 amount_minor bigint not null check(amount_minor > 0), reference text, collected_by uuid not null, reconciled boolean not null default false,
 created_at timestamptz not null default now()
);
create index if not exists delivery_collections_reconciliation on public.delivery_collections(reconciled,created_at);

alter table public.deliveries enable row level security;
alter table public.delivery_events enable row level security;
alter table public.delivery_collections enable row level security;
revoke all on public.deliveries from anon, authenticated;
revoke all on public.delivery_events from anon, authenticated;
revoke all on public.delivery_collections from anon, authenticated;

-- Authoritative CEO KPI snapshot. Values are derived from persisted operational tables;
-- no UI-side arithmetic is treated as the source of truth.
create or replace view public.ceo_daily_kpis as
select
 current_date as business_date,
 coalesce((select sum(total_minor) from public.restaurant_orders where status='COMPLETED' and created_at::date=current_date),0) as sales_minor,
 coalesce((select count(*) from public.restaurant_orders where created_at::date=current_date),0) as orders_count,
 coalesce((select count(*) from public.deliveries where created_at::date=current_date and status='DELIVERED'),0) as deliveries_completed,
 coalesce((select sum(amount_minor) from public.expenses where status='APPROVED' and created_at::date=current_date),0) as approved_expenses_minor,
 coalesce((select sum(quantity) from public.stock_movements where movement_type in ('WASTAGE','DAMAGE','EXPIRY') and created_at::date=current_date),0) as loss_quantity,
 now() as generated_at;

revoke all on public.ceo_daily_kpis from anon, authenticated;
