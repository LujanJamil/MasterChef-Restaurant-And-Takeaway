-- MasterChef V1 Completion Layer
-- Najjera II Central Market
-- Makes menu -> recipe -> stock -> cost -> sale -> payment -> reconciliation -> reporting auditable.
create extension if not exists pgcrypto;

create table if not exists public.recipe_headers (
 id uuid primary key default gen_random_uuid(),
 menu_item_id uuid not null unique references public.menu_items(id),
 version integer not null default 1 check(version > 0),
 yield_quantity numeric(18,3) not null default 1 check(yield_quantity > 0),
 yield_unit text not null default 'PCS',
 active boolean not null default true,
 effective_from timestamptz not null default now(),
 effective_to timestamptz,
 created_by uuid not null,
 created_at timestamptz not null default now(),
 check(effective_to is null or effective_to > effective_from)
);
create table if not exists public.recipe_lines (
 id uuid primary key default gen_random_uuid(),
 recipe_id uuid not null references public.recipe_headers(id) on delete cascade,
 inventory_item_id uuid not null references public.inventory_items(id),
 quantity numeric(18,6) not null check(quantity > 0),
 unit text not null,
 waste_percent numeric(8,4) not null default 0 check(waste_percent >= 0 and waste_percent <= 100),
 created_at timestamptz not null default now(),
 unique(recipe_id,inventory_item_id)
);
create index if not exists recipe_lines_item on public.recipe_lines(inventory_item_id);

create table if not exists public.inventory_cost_layers (
 id uuid primary key default gen_random_uuid(),
 item_id uuid not null references public.inventory_items(id),
 quantity_remaining numeric(18,6) not null check(quantity_remaining >= 0),
 unit_cost_minor bigint not null check(unit_cost_minor >= 0),
 currency char(3) not null default 'UGX',
 source_movement_id uuid references public.stock_movements(id),
 created_at timestamptz not null default now()
);
create index if not exists inventory_cost_layers_item on public.inventory_cost_layers(item_id,created_at);

create table if not exists public.product_cost_snapshots (
 id uuid primary key default gen_random_uuid(),
 menu_item_id uuid not null references public.menu_items(id),
 recipe_id uuid references public.recipe_headers(id),
 ingredient_cost_minor bigint not null check(ingredient_cost_minor >= 0),
 waste_cost_minor bigint not null default 0 check(waste_cost_minor >= 0),
 total_cost_minor bigint not null check(total_cost_minor >= 0),
 selling_price_minor bigint not null check(selling_price_minor >= 0),
 gross_margin_minor bigint not null,
 margin_percent numeric(10,4) not null,
 calculated_at timestamptz not null default now(),
 unique(menu_item_id,calculated_at)
);
create index if not exists product_cost_snapshots_item_time on public.product_cost_snapshots(menu_item_id,calculated_at desc);

create table if not exists public.order_payments (
 id uuid primary key default gen_random_uuid(),
 order_id uuid not null references public.restaurant_orders(id),
 method text not null check(method in ('CASH','MTN_MOBILE_MONEY','AIRTEL_MONEY','OTHER')),
 amount_minor bigint not null check(amount_minor > 0),
 reference text,
 status text not null default 'RECORDED' check(status in ('RECORDED','VERIFIED','VOIDED','REFUNDED')),
 received_by uuid not null,
 verified_by uuid,
 created_at timestamptz not null default now(),
 updated_at timestamptz not null default now()
);
create index if not exists order_payments_order on public.order_payments(order_id,created_at);
create index if not exists order_payments_method_time on public.order_payments(method,created_at);

create table if not exists public.cash_reconciliations (
 id uuid primary key default gen_random_uuid(),
 business_date date not null,
 shift_label text not null,
 expected_cash_minor bigint not null check(expected_cash_minor >= 0),
 counted_cash_minor bigint not null check(counted_cash_minor >= 0),
 variance_minor bigint generated always as (counted_cash_minor-expected_cash_minor) stored,
 counted_by uuid not null,
 approved_by uuid,
 status text not null default 'OPEN' check(status in ('OPEN','SUBMITTED','APPROVED','REJECTED')),
 notes text,
 created_at timestamptz not null default now(),
 updated_at timestamptz not null default now()
);
create unique index if not exists cash_reconciliation_shift on public.cash_reconciliations(business_date,shift_label);

create table if not exists public.stock_reconciliations (
 id uuid primary key default gen_random_uuid(),
 business_date date not null,
 item_id uuid not null references public.inventory_items(id),
 expected_quantity numeric(18,6) not null,
 counted_quantity numeric(18,6) not null check(counted_quantity >= 0),
 variance_quantity numeric(18,6) generated always as (counted_quantity-expected_quantity) stored,
 counted_by uuid not null,
 status text not null default 'OPEN' check(status in ('OPEN','SUBMITTED','APPROVED','REJECTED')),
 reason text,
 created_at timestamptz not null default now()
);
create index if not exists stock_reconciliation_date on public.stock_reconciliations(business_date,item_id);

create table if not exists public.operational_notifications (
 id uuid primary key default gen_random_uuid(),
 severity text not null check(severity in ('INFO','WARNING','CRITICAL')),
 category text not null,
 title text not null,
 message text not null,
 entity_type text,
 entity_id uuid,
 acknowledged_by uuid,
 acknowledged_at timestamptz,
 resolved_by uuid,
 resolved_at timestamptz,
 created_at timestamptz not null default now()
);
create index if not exists operational_notifications_open on public.operational_notifications(resolved_at,created_at desc);

create table if not exists public.backup_runs (
 id uuid primary key default gen_random_uuid(),
 backup_type text not null check(backup_type in ('LOCAL','CLOUD','FULL')),
 status text not null check(status in ('STARTED','COMPLETED','FAILED','VERIFIED')),
 artifact_reference text,
 checksum_sha256 text,
 started_at timestamptz not null default now(),
 completed_at timestamptz,
 verified_at timestamptz,
 initiated_by uuid,
 error_message text
);
create index if not exists backup_runs_time on public.backup_runs(started_at desc);

-- Authoritative operational reporting views. They intentionally aggregate persisted records only.
create or replace view public.ceo_payment_reconciliation as
select current_date as business_date,
 coalesce(sum(case when method='CASH' and status in ('RECORDED','VERIFIED') then amount_minor else 0 end),0) as cash_minor,
 coalesce(sum(case when method='MTN_MOBILE_MONEY' and status in ('RECORDED','VERIFIED') then amount_minor else 0 end),0) as mtn_minor,
 coalesce(sum(case when method='AIRTEL_MONEY' and status in ('RECORDED','VERIFIED') then amount_minor else 0 end),0) as airtel_minor,
 coalesce(sum(case when method='OTHER' and status in ('RECORDED','VERIFIED') then amount_minor else 0 end),0) as other_minor,
 coalesce(sum(case when status in ('RECORDED','VERIFIED') then amount_minor else 0 end),0) as total_collected_minor
from public.order_payments where created_at::date=current_date;

create or replace view public.ceo_product_profitability as
select m.id menu_item_id,m.name,m.current_price_minor,
 coalesce((select pcs.total_cost_minor from public.product_cost_snapshots pcs where pcs.menu_item_id=m.id order by pcs.calculated_at desc limit 1),0) as cost_minor,
 m.current_price_minor-coalesce((select pcs.total_cost_minor from public.product_cost_snapshots pcs where pcs.menu_item_id=m.id order by pcs.calculated_at desc limit 1),0) as gross_margin_minor
from public.menu_items m where m.active=true;

create or replace view public.ceo_daily_report as
select current_date business_date,
 coalesce((select sum(total_minor) from public.restaurant_orders where status='COMPLETED' and created_at::date=current_date),0) sales_minor,
 coalesce((select count(*) from public.restaurant_orders where created_at::date=current_date),0) orders_count,
 coalesce((select sum(amount_minor) from public.order_payments where status in ('RECORDED','VERIFIED') and created_at::date=current_date),0) collected_minor,
 coalesce((select sum(amount_minor) from public.expenses where status='APPROVED' and created_at::date=current_date),0) expenses_minor,
 coalesce((select count(*) from public.deliveries where status='DELIVERED' and created_at::date=current_date),0) deliveries_completed,
 now() generated_at;

alter table public.recipe_headers enable row level security;
alter table public.recipe_lines enable row level security;
alter table public.inventory_cost_layers enable row level security;
alter table public.product_cost_snapshots enable row level security;
alter table public.order_payments enable row level security;
alter table public.cash_reconciliations enable row level security;
alter table public.stock_reconciliations enable row level security;
alter table public.operational_notifications enable row level security;
alter table public.backup_runs enable row level security;
revoke all on public.recipe_headers,public.recipe_lines,public.inventory_cost_layers,public.product_cost_snapshots,public.order_payments,public.cash_reconciliations,public.stock_reconciliations,public.operational_notifications,public.backup_runs from anon,authenticated;
revoke all on public.ceo_payment_reconciliation,public.ceo_product_profitability,public.ceo_daily_report from anon,authenticated;
