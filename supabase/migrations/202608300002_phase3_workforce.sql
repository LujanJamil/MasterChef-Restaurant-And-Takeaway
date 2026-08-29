-- Phase 3 Workforce — free-tier compatible tables + ingest RPC
-- RLS disabled; access only via Edge Function with service role.

CREATE TABLE IF NOT EXISTS workforce_attendance (
  id TEXT PRIMARY KEY,
  worker_id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  clock_in_at BIGINT NOT NULL,
  clock_out_at BIGINT,
  break_minutes INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL CHECK (status IN ('OPEN','CLOSED','ADJUSTED')),
  notes TEXT,
  payload_hash TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wf_att_worker ON workforce_attendance (worker_id, clock_in_at DESC);

CREATE TABLE IF NOT EXISTS workforce_shifts (
  id TEXT PRIMARY KEY,
  worker_id TEXT NOT NULL,
  role TEXT NOT NULL,
  starts_at BIGINT NOT NULL,
  ends_at BIGINT NOT NULL,
  location TEXT NOT NULL,
  created_by TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  updated_at BIGINT NOT NULL,
  CHECK (ends_at > starts_at)
);

CREATE INDEX IF NOT EXISTS idx_wf_shift_worker ON workforce_shifts (worker_id, starts_at);

CREATE TABLE IF NOT EXISTS workforce_pay (
  id TEXT PRIMARY KEY,
  worker_id TEXT NOT NULL,
  period_start BIGINT NOT NULL,
  period_end BIGINT NOT NULL,
  hours_worked DOUBLE PRECISION NOT NULL,
  rate_minor BIGINT NOT NULL,
  amount_minor BIGINT NOT NULL,
  currency TEXT NOT NULL DEFAULT 'UGX',
  status TEXT NOT NULL CHECK (status IN ('DRAFT','APPROVED','PAID')),
  created_by TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  updated_at BIGINT NOT NULL,
  CHECK (period_end > period_start),
  UNIQUE (worker_id, period_start, period_end)
);

CREATE INDEX IF NOT EXISTS idx_wf_pay_worker ON workforce_pay (worker_id, period_start);

-- Idempotent ingest for workforce entities (used by Edge Function after sync_operations).
CREATE OR REPLACE FUNCTION record_workforce_entity(
  p_entity_type TEXT,
  p_entity_id TEXT,
  p_payload JSONB,
  p_payload_hash TEXT,
  p_device_id TEXT
) RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
  v_hash TEXT;
BEGIN
  IF p_entity_type IS NULL OR p_entity_id IS NULL OR p_payload IS NULL OR p_payload_hash IS NULL THEN
    RAISE EXCEPTION 'invalid arguments';
  END IF;

  v_hash := encode(digest(p_payload::text, 'sha256'), 'base64');
  IF v_hash <> p_payload_hash THEN
    RAISE EXCEPTION 'payload hash mismatch';
  END IF;

  IF p_entity_type = 'ATTENDANCE' THEN
    INSERT INTO workforce_attendance (
      id, worker_id, device_id, clock_in_at, clock_out_at, break_minutes, status, notes,
      payload_hash, created_at, updated_at
    ) VALUES (
      p_entity_id,
      p_payload->>'worker_id',
      COALESCE(p_payload->>'device_id', p_device_id),
      (p_payload->>'clock_in_at')::BIGINT,
      NULLIF(p_payload->>'clock_out_at', '')::BIGINT,
      COALESCE((p_payload->>'break_minutes')::INTEGER, 0),
      COALESCE(p_payload->>'status', 'CLOSED'),
      p_payload->>'notes',
      p_payload_hash,
      COALESCE((p_payload->>'created_at')::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
      COALESCE((p_payload->>'updated_at')::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT)
    )
    ON CONFLICT (id) DO UPDATE SET
      clock_out_at = EXCLUDED.clock_out_at,
      break_minutes = EXCLUDED.break_minutes,
      status = EXCLUDED.status,
      notes = EXCLUDED.notes,
      payload_hash = EXCLUDED.payload_hash,
      updated_at = EXCLUDED.updated_at;
    RETURN TRUE;
  ELSIF p_entity_type = 'SHIFT' THEN
    INSERT INTO workforce_shifts (
      id, worker_id, role, starts_at, ends_at, location, created_by, payload_hash, created_at, updated_at
    ) VALUES (
      p_entity_id,
      p_payload->>'worker_id',
      p_payload->>'role',
      (p_payload->>'starts_at')::BIGINT,
      (p_payload->>'ends_at')::BIGINT,
      COALESCE(p_payload->>'location', 'Najjera II Central Market'),
      COALESCE(p_payload->>'created_by', p_device_id),
      p_payload_hash,
      COALESCE((p_payload->>'created_at')::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
      COALESCE((p_payload->>'updated_at')::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT)
    )
    ON CONFLICT (id) DO UPDATE SET
      role = EXCLUDED.role,
      starts_at = EXCLUDED.starts_at,
      ends_at = EXCLUDED.ends_at,
      location = EXCLUDED.location,
      payload_hash = EXCLUDED.payload_hash,
      updated_at = EXCLUDED.updated_at;
    RETURN TRUE;
  ELSIF p_entity_type = 'PAY' THEN
    INSERT INTO workforce_pay (
      id, worker_id, period_start, period_end, hours_worked, rate_minor, amount_minor, currency, status,
      created_by, payload_hash, created_at, updated_at
    ) VALUES (
      p_entity_id,
      p_payload->>'worker_id',
      (p_payload->>'period_start')::BIGINT,
      (p_payload->>'period_end')::BIGINT,
      (p_payload->>'hours_worked')::DOUBLE PRECISION,
      (p_payload->>'rate_minor')::BIGINT,
      (p_payload->>'amount_minor')::BIGINT,
      COALESCE(p_payload->>'currency', 'UGX'),
      COALESCE(p_payload->>'status', 'DRAFT'),
      COALESCE(p_payload->>'created_by', p_device_id),
      p_payload_hash,
      COALESCE((p_payload->>'created_at')::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT),
      COALESCE((p_payload->>'updated_at')::BIGINT, EXTRACT(EPOCH FROM NOW())::BIGINT)
    )
    ON CONFLICT (id) DO UPDATE SET
      hours_worked = EXCLUDED.hours_worked,
      rate_minor = EXCLUDED.rate_minor,
      amount_minor = EXCLUDED.amount_minor,
      status = EXCLUDED.status,
      payload_hash = EXCLUDED.payload_hash,
      updated_at = EXCLUDED.updated_at;
    RETURN TRUE;
  ELSE
    RAISE EXCEPTION 'unsupported entity_type %', p_entity_type;
  END IF;
END;
$$;
