create extension if not exists pgcrypto;

create table if not exists public.menu_categories (
 id uuid primary key default gen_random_uuid(), name text not null, kind text not null check (kind in ('FOOD','SNACKS','DRINKS','ICE_CREAM','CHAPATI','BREAD','OTHER')), sort_order integer not null default 0 check (sort_order >= 0), active boolean not null default true, created_by uuid not null, created_at timestamptz not null default now(), updated_at timestamptz not null default now()
);
create unique index if not exists menu_categories_name_uq on public.menu_categories(lower(name));

create table if not exists public.menu_items (
 id uuid primary key default gen_random_uuid(), category_id uuid not null references public.menu_categories(id), name text not null, description text, sku text not null unique, station text not null check (station in ('KITCHEN','DISPLAY','FRIDGE','ICE_CREAM','CHAPATI','LOCAL_FOOD','OTHER')), active boolean not null default true, available boolean not null default true, allow_takeaway boolean not null default true, allow_dine_in boolean not null default true, allow_delivery boolean not null default false, created_by uuid not null, created_at timestamptz not null default now(), updated_at timestamptz not null default now()
);
create index if not exists menu_items_category_idx on public.menu_items(category_id);

create table if not exists public.menu_prices (
 id uuid primary key default gen_random_uuid(), menu_item_id uuid not null references public.menu_items(id), amount_minor bigint not null check (amount_minor >= 0), currency char(3) not null default 'UGX', effective_from timestamptz not null, effective_to timestamptz, status text not null check (status in ('ACTIVE','SCHEDULED','EXPIRED','CANCELLED')), changed_by uuid not null, created_at timestamptz not null default now(), check (effective_to is null or effective_to > effective_from)
);
create index if not exists menu_prices_lookup_idx on public.menu_prices(menu_item_id,effective_from,effective_to);

create table if not exists public.restaurant_tables (
 id uuid primary key default gen_random_uuid(), label text not null unique, active boolean not null default true
);

create table if not exists public.orders (
 id uuid primary key default gen_random_uuid(), order_number text not null unique, order_type text not null check (order_type in ('DINE_IN','TAKEAWAY','DELIVERY')), status text not null check (status in ('OPEN','SUBMITTED','IN_PREPARATION','READY','SERVED','PACKED','COMPLETED','CANCELLED')), table_id uuid references public.restaurant_tables(id), customer_name text, customer_phone text, waiter_id uuid, created_by uuid not null, device_id uuid not null, notes text, subtotal_minor bigint not null default 0 check (subtotal_minor >= 0), discount_minor bigint not null default 0 check (discount_minor >= 0), total_minor bigint not null default 0 check (total_minor >= 0), currency char(3) not null default 'UGX', version bigint not null default 1, created_at timestamptz not null default now(), updated_at timestamptz not null default now(), check ((order_type = 'DINE_IN' and table_id is not null) or (order_type <> 'DINE_IN' and table_id is null)), check (total_minor = subtotal_minor - discount_minor)
);
create index if not exists orders_status_idx on public.orders(status,created_at);
create index if not exists orders_waiter_idx on public.orders(waiter_id,created_at);

create table if not exists public.order_items (
 id uuid primary key default gen_random_uuid(), order_id uuid not null references public.orders(id) on delete restrict, menu_item_id uuid not null references public.menu_items(id), item_name_snapshot text not null, unit_price_minor bigint not null check (unit_price_minor >= 0), quantity integer not null check (quantity >= 0), discount_minor bigint not null default 0 check (discount_minor >= 0), line_total_minor bigint not null check (line_total_minor >= 0), station text not null, status text not null check (status in ('PENDING','SENT_TO_KITCHEN','IN_PREPARATION','READY','SERVED','CANCELLED')), notes text, created_by uuid not null, created_at timestamptz not null default now(), updated_at timestamptz not null default now(), check (line_total_minor = greatest(0,unit_price_minor * quantity - least(discount_minor,unit_price_minor * quantity)))
);
create index if not exists order_items_order_idx on public.order_items(order_id);
create index if not exists order_items_station_idx on public.order_items(station,status);

create table if not exists public.order_item_cancellations (
 id uuid primary key default gen_random_uuid(), order_item_id uuid not null references public.order_items(id) on delete restrict, quantity integer not null check (quantity > 0), reason text not null check (reason in ('CUSTOMER_REQUEST','OUT_OF_STOCK','WRONG_ITEM','DUPLICATE','MANAGEMENT','OTHER')), note text, authorized_by uuid, cancelled_by uuid not null, created_at timestamptz not null default now()
);

create table if not exists public.order_events (
 id uuid primary key default gen_random_uuid(), order_id uuid not null references public.orders(id) on delete restrict, order_item_id uuid references public.order_items(id) on delete restrict, event_type text not null check (event_type in ('CREATED','ITEM_ADDED','ITEM_UPDATED','ITEM_CANCELLED','STATUS_CHANGED','DISCOUNT_APPLIED','NOTE_CHANGED')), actor_id uuid not null, device_id uuid not null, occurred_at timestamptz not null default now(), payload jsonb not null
);
create index if not exists order_events_order_idx on public.order_events(order_id,occurred_at);

alter table public.menu_categories enable row level security;
alter table public.menu_items enable row level security;
alter table public.menu_prices enable row level security;
alter table public.restaurant_tables enable row level security;
alter table public.orders enable row level security;
alter table public.order_items enable row level security;
alter table public.order_item_cancellations enable row level security;
alter table public.order_events enable row level security;

-- Direct client access is denied by default. The authenticated sync boundary/service role performs writes.
revoke all on public.menu_categories, public.menu_items, public.menu_prices, public.restaurant_tables, public.orders, public.order_items, public.order_item_cancellations, public.order_events from anon, authenticated;
