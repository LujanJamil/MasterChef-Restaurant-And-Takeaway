create extension if not exists pgcrypto;

create table if not exists public.sync_devices (
  device_id uuid primary key,
  location_id uuid,
  name text not null,
  status text not null default 'ACTIVE' check (status in ('ACTIVE','SUSPENDED','REVOKED')),
  last_seen_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.sync_device_credentials (
  device_id uuid primary key references public.sync_devices(device_id) on delete cascade,
  token_hash text not null,
  created_at timestamptz not null default now(),
  rotated_at timestamptz
);

create table if not exists public.sync_operations (
  id uuid primary key,
  device_id uuid not null references public.sync_devices(device_id),
  entity_type text not null,
  entity_id uuid not null,
  operation text not null check (operation in ('CREATE','UPDATE','DELETE','ADJUST')),
  payload jsonb not null,
  payload_hash text not null,
  sequence bigint not null,
  client_created_at timestamptz not null,
  received_at timestamptz not null default now(),
  applied_at timestamptz,
  status text not null default 'RECEIVED' check (status in ('RECEIVED','APPLIED','CONFLICT','REJECTED')),
  error_code text,
  unique(device_id, sequence),
  unique(device_id, id)
);

create index if not exists idx_sync_operations_device_sequence on public.sync_operations(device_id, sequence);
create index if not exists idx_sync_operations_status on public.sync_operations(status, received_at);
create index if not exists idx_sync_operations_entity on public.sync_operations(entity_type, entity_id, sequence);

create table if not exists public.sync_cursors (
  device_id uuid primary key references public.sync_devices(device_id),
  last_received_sequence bigint not null default 0,
  last_pushed_sequence bigint not null default 0,
  updated_at timestamptz not null default now()
);

create table if not exists public.sync_conflicts (
  id uuid primary key default gen_random_uuid(),
  operation_id uuid not null references public.sync_operations(id),
  entity_type text not null,
  entity_id uuid not null,
  local_payload jsonb,
  remote_payload jsonb,
  detected_at timestamptz not null default now(),
  resolved_at timestamptz,
  resolution text
);

create table if not exists public.sync_audit (
  id uuid primary key default gen_random_uuid(),
  operation_id uuid,
  device_id uuid,
  action text not null,
  details jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

alter table public.sync_devices enable row level security;
alter table public.sync_device_credentials enable row level security;
alter table public.sync_operations enable row level security;
alter table public.sync_cursors enable row level security;
alter table public.sync_conflicts enable row level security;
alter table public.sync_audit enable row level security;

-- No direct client policies are granted. The Edge Function is the trusted boundary.
-- The service-role key is server-only and must never ship in a desktop application.

create or replace function public.record_sync_operation(
  p_id uuid,
  p_device_id uuid,
  p_entity_type text,
  p_entity_id uuid,
  p_operation text,
  p_payload jsonb,
  p_payload_hash text,
  p_sequence bigint,
  p_client_created_at timestamptz
) returns public.sync_operations
language plpgsql
security definer
set search_path = public
as $$
declare r public.sync_operations;
       expected_hash text;
       current_seq bigint;
begin
  if length(p_entity_type) < 1 or length(p_entity_type) > 100 then raise exception 'INVALID_ENTITY_TYPE'; end if;
  if p_operation not in ('CREATE','UPDATE','DELETE','ADJUST') then raise exception 'INVALID_OPERATION'; end if;
  expected_hash := encode(digest(convert_to(p_payload::text,'UTF8'),'sha256'),'base64');
  if expected_hash <> p_payload_hash then raise exception 'PAYLOAD_HASH_MISMATCH'; end if;

  insert into public.sync_devices(device_id,name) values(p_device_id,'device-'||left(p_device_id::text,8)) on conflict(device_id) do update set last_seen_at=now(),updated_at=now();
  insert into public.sync_cursors(device_id) values(p_device_id) on conflict(device_id) do nothing;
  select last_received_sequence into current_seq from public.sync_cursors where device_id=p_device_id for update;

  if p_sequence <= current_seq then
    select * into r from public.sync_operations where device_id=p_device_id and sequence=p_sequence;
    if r.id is not null and r.payload_hash = p_payload_hash then return r; end if;
    raise exception 'SEQUENCE_REUSE';
  end if;
  if p_sequence <> current_seq + 1 then raise exception 'SEQUENCE_GAP'; end if;

  insert into public.sync_operations(id,device_id,entity_type,entity_id,operation,payload,payload_hash,sequence,client_created_at,status)
  values(p_id,p_device_id,p_entity_type,p_entity_id,p_operation,p_payload,p_payload_hash,p_sequence,p_client_created_at,'RECEIVED') returning * into r;
  update public.sync_cursors set last_received_sequence=p_sequence,updated_at=now() where device_id=p_device_id;
  insert into public.sync_audit(operation_id,device_id,action,details) values(p_id,p_device_id,'SYNC_RECEIVED',jsonb_build_object('entity_type',p_entity_type,'entity_id',p_entity_id,'sequence',p_sequence));
  return r;
end;
$$;
