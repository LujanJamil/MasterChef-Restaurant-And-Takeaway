-- MasterChef Release B: Inventory, Suppliers, Expenses & Equipment
-- Najjera II Central Market

create extension if not exists pgcrypto;

create table if not exists public.inventory_items (
 id uuid primary key default gen_random_uuid(), sku text not null unique, name text not null,
 unit text not null check (unit in ('PCS','KG','G','L','ML','BOX','PACK','BOTTLE','PLATE')),
 category text not null, reorder_level numeric(18,3) not null default 0 check (reorder_level >= 0),
 active boolean not null default true, created_at timestamptz not null default now(), updated_at timestamptz not null default now()
);
create table if not exists public.suppliers (
 id uuid primary key default gen_random_uuid(), name text not null, phone text, address text, active boolean not null default true,
 created_at timestamptz not null default now(), updated_at timestamptz not null default now()
);
create table if not exists public.supplier_prices (
 id uuid primary key default gen_random_uuid(), supplier_id uuid not null references public.suppliers(id), item_id uuid not null references public.inventory_items(id),
 unit_cost_minor bigint not null check (unit_cost_minor >= 0), currency char(3) not null default 'UGX',
 effective_from timestamptz not null default now(), effective_to timestamptz
);
create index if not exists supplier_prices_item on public.supplier_prices(item_id,effective_from desc);

create table if not exists public.stock_movements (
 id uuid primary key default gen_random_uuid(), item_id uuid not null references public.inventory_items(id),
 movement_type text not null check (movement_type in ('OPENING','PURCHASE_RECEIPT','ISSUE','RETURN','TRANSFER','WASTAGE','DAMAGE','EXPIRY','COUNT_ADJUSTMENT','REVERSAL')),
 quantity numeric(18,3) not null check (quantity > 0), unit text not null,
 from_location text, to_location text, unit_cost_minor bigint check (unit_cost_minor is null or unit_cost_minor >= 0),
 source_id uuid, reason text, actor_id uuid not null, device_id text not null, created_at timestamptz not null default now()
);
create index if not exists stock_movements_item_time on public.stock_movements(item_id,created_at desc);
create index if not exists stock_movements_locations on public.stock_movements(from_location,to_location,created_at desc);

create table if not exists public.stock_counts (
 id uuid primary key default gen_random_uuid(), location text not null, status text not null default 'OPEN' check(status in ('OPEN','FINALIZED')),
 counted_by uuid not null, created_at timestamptz not null default now(), finalized_at timestamptz
);
create table if not exists public.stock_count_lines (
 id uuid primary key default gen_random_uuid(), count_id uuid not null references public.stock_counts(id) on delete cascade,
 item_id uuid not null references public.inventory_items(id), expected_quantity numeric(18,3) not null,
 counted_quantity numeric(18,3) not null check(counted_quantity >= 0), variance numeric(18,3) generated always as (counted_quantity-expected_quantity) stored,
 unique(count_id,item_id)
);

create table if not exists public.expenses (
 id uuid primary key default gen_random_uuid(), category text not null, amount_minor bigint not null check(amount_minor > 0), currency char(3) not null default 'UGX',
 payment_method text not null check(payment_method in ('CASH','MTN_MOMO','AIRTEL_MONEY','OTHER')),
 status text not null default 'DRAFT' check(status in ('DRAFT','SUBMITTED','APPROVED','REJECTED','REVERSED')),
 description text not null, reference text, created_by uuid not null, approved_by uuid, created_at timestamptz not null default now(), updated_at timestamptz not null default now()
);
create index if not exists expenses_status_time on public.expenses(status,created_at desc);

create table if not exists public.equipment (
 id uuid primary key default gen_random_uuid(), asset_tag text not null unique, name text not null, serial_number text,
 area text not null, assigned_worker_id uuid, condition text not null default 'NEW' check(condition in ('NEW','GOOD','FAIR','DAMAGED','LOST','UNDER_REPAIR','RETIRED')),
 created_at timestamptz not null default now(), updated_at timestamptz not null default now()
);
create table if not exists public.equipment_events (
 id uuid primary key default gen_random_uuid(), equipment_id uuid not null references public.equipment(id),
 event_type text not null check(event_type in ('ASSIGNED','UNASSIGNED','DAMAGE_REPORTED','LOST_REPORTED','REPAIR_STARTED','REPAIR_COMPLETED','REPLACED','RETIRED')),
 actor_id uuid not null, worker_id uuid, reason text, created_at timestamptz not null default now()
);

alter table public.inventory_items enable row level security;
alter table public.suppliers enable row level security;
alter table public.supplier_prices enable row level security;
alter table public.stock_movements enable row level security;
alter table public.stock_counts enable row level security;
alter table public.stock_count_lines enable row level security;
alter table public.expenses enable row level security;
alter table public.equipment enable row level security;
alter table public.equipment_events enable row level security;

-- Service/native authorization remains the primary command boundary. These policies prevent anonymous public access.
create policy inventory_authenticated_read on public.inventory_items for select to authenticated using (true);
create policy suppliers_authenticated_read on public.suppliers for select to authenticated using (true);
create policy supplier_prices_authenticated_read on public.supplier_prices for select to authenticated using (true);
create policy stock_authenticated_read on public.stock_movements for select to authenticated using (true);
create policy counts_authenticated_read on public.stock_counts for select to authenticated using (true);
create policy count_lines_authenticated_read on public.stock_count_lines for select to authenticated using (true);
create policy expenses_authenticated_read on public.expenses for select to authenticated using (true);
create policy equipment_authenticated_read on public.equipment for select to authenticated using (true);
create policy equipment_events_authenticated_read on public.equipment_events for select to authenticated using (true);
