-- Phase 4: Menu & Orders
create extension if not exists pgcrypto;

create table if not exists public.menu_categories (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  active boolean not null default true,
  sort_order integer not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create unique index if not exists menu_categories_name_ci on public.menu_categories (lower(name));

create table if not exists public.menu_items (
  id uuid primary key default gen_random_uuid(),
  category_id uuid not null references public.menu_categories(id),
  name text not null,
  description text,
  station text not null,
  active boolean not null default true,
  dine_in boolean not null default true,
  takeaway boolean not null default true,
  delivery boolean not null default false,
  current_price_minor bigint not null check (current_price_minor >= 0),
  currency text not null default 'UGX' check (length(currency)=3),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);
create index if not exists menu_items_category_active on public.menu_items(category_id,active);

create table if not exists public.menu_prices (
  id uuid primary key default gen_random_uuid(),
  menu_item_id uuid not null references public.menu_items(id),
  price_minor bigint not null check (price_minor >= 0),
  currency text not null check (length(currency)=3),
  effective_from timestamptz not null,
  effective_to timestamptz,
  created_by uuid not null,
  created_at timestamptz not null default now(),
  check (effective_to is null or effective_to > effective_from)
);
create index if not exists menu_prices_item_effective on public.menu_prices(menu_item_id,effective_from);

create table if not exists public.restaurant_tables (
  id uuid primary key default gen_random_uuid(),
  name text not null unique,
  active boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.restaurant_orders (
  id uuid primary key default gen_random_uuid(),
  order_number bigint not null unique,
  order_type text not null check (order_type in ('DINE_IN','TAKEAWAY','DELIVERY')),
  table_id uuid references public.restaurant_tables(id),
  customer_name text,
  customer_phone text,
  waiter_id uuid not null,
  status text not null check (status in ('DRAFT','SUBMITTED','PREPARING','READY','SERVED','COMPLETED','CANCELLED')),
  subtotal_minor bigint not null default 0 check (subtotal_minor >= 0),
  discount_minor bigint not null default 0 check (discount_minor >= 0),
  total_minor bigint not null default 0 check (total_minor >= 0),
  currency text not null default 'UGX' check (length(currency)=3),
  notes text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (total_minor = subtotal_minor - discount_minor),
  check (order_type <> 'DINE_IN' or table_id is not null)
);
create index if not exists restaurant_orders_status on public.restaurant_orders(status,updated_at desc);

create table if not exists public.restaurant_order_items (
  id uuid primary key default gen_random_uuid(),
  order_id uuid not null references public.restaurant_orders(id) on delete cascade,
  menu_item_id uuid not null references public.menu_items(id),
  name_snapshot text not null,
  unit_price_minor bigint not null check (unit_price_minor >= 0),
  quantity bigint not null check (quantity > 0),
  total_minor bigint not null check (total_minor >= 0),
  station text not null,
  status text not null check (status in ('ORDERED','PREPARING','READY','SERVED','CANCELLED')),
  notes text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (total_minor = unit_price_minor * quantity)
);
create index if not exists restaurant_order_items_order on public.restaurant_order_items(order_id,created_at);

create table if not exists public.restaurant_order_events (
  id uuid primary key default gen_random_uuid(),
  order_id uuid not null references public.restaurant_orders(id) on delete cascade,
  item_id uuid,
  actor_id uuid not null,
  event_type text not null,
  reason text,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);
create index if not exists restaurant_order_events_order on public.restaurant_order_events(order_id,created_at);

alter table public.menu_categories enable row level security;
alter table public.menu_items enable row level security;
alter table public.menu_prices enable row level security;
alter table public.restaurant_tables enable row level security;
alter table public.restaurant_orders enable row level security;
alter table public.restaurant_order_items enable row level security;
alter table public.restaurant_order_events enable row level security;

-- Access is intentionally deny-by-default until the application's authenticated
-- sync boundary establishes the correct tenant/device/role claims.
revoke all on public.menu_categories from anon, authenticated;
revoke all on public.menu_items from anon, authenticated;
revoke all on public.menu_prices from anon, authenticated;
revoke all on public.restaurant_tables from anon, authenticated;
revoke all on public.restaurant_orders from anon, authenticated;
revoke all on public.restaurant_order_items from anon, authenticated;
revoke all on public.restaurant_order_events from anon, authenticated;
