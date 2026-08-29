# MasterChef Phase 2 — Offline Data & Sync Core

## Objective

Make the restaurant application operational during internet outages while preserving a durable, auditable path to cloud synchronization when connectivity returns.

## Guarantees

- Local writes complete without requiring internet access.
- Every operation has a UUID, device identity, entity identity and monotonic per-device sequence.
- Payload integrity is protected by SHA-256 hashing.
- Queue processing is idempotent.
- Retries use bounded exponential backoff.
- In-flight, failed and conflict states are explicit.
- Remote ingestion rejects payload tampering and sequence reuse.
- Cloud ingestion is transactional around the per-device sequence cursor.
- Service-role credentials never enter a desktop application.
- Critical sync events are auditable.

## Local lifecycle

LOCAL WRITE -> ENQUEUE -> PENDING -> IN_FLIGHT -> SYNCED

Failures move an operation to FAILED with a retry time. A semantic collision is represented as CONFLICT and requires an explicit resolution rather than silent overwriting.

## Cloud lifecycle

AUTHENTICATED DEVICE -> VALIDATE HASH -> VALIDATE SEQUENCE -> STORE OPERATION -> ADVANCE CURSOR -> AUDIT

The cloud database is not exposed directly to the desktop clients. The Edge Function is the trusted sync boundary.

## Security

Device credentials are stored as hashes in the cloud. The service-role key is Edge Function only. Row Level Security is enabled on sync tables with no direct client policy. Device credentials must be provisioned and rotated through an authenticated administrative workflow before a production device is allowed to synchronize.

## Verification

Phase 2 is complete only when clean install, typecheck, lint, build, Rust checks/tests, sync tests, duplicate replay, payload tampering, sequence validation, retry/backoff, conflict handling, migration, offline operation, reconnect/replay and backup/restore tests all pass locally and in CI against a dedicated MasterChef cloud environment.
