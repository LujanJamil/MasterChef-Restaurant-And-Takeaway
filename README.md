# MasterChef Restaurant & Takeaway

MasterChef is an application-first, offline-first restaurant operating system for the Master Restaurant with Master Chefs at **Najjera II Central Market**.

## Applications

- `apps/ceo` — dedicated CEO desktop application
- `apps/restaurant` — dedicated restaurant operations desktop application
- `packages/domain` — shared business types and rules
- `packages/permissions` — capability-based authorization contracts
- `packages/audit` — audit event contracts
- `packages/validation` — shared runtime validation
- `packages/native-core` — native Rust authentication, authorization, sessions and security audit persistence
- `packages/sync-core` — offline-first transaction queue with integrity, sequencing, retry and conflict handling
- `packages/workforce-core` — offline-first attendance, shifts and payroll SQLite core

## Architecture

Tauri 2 + React + TypeScript desktop applications with a native Rust security boundary, persistent local SQLite identity/security storage, offline-first sync queue to Supabase, and workforce (attendance / shifts / pay) local store.

## Security baseline

Phase 1 includes Argon2id password hashing, individual identities, role-based authorization, per-user permission overrides, CEO authority, session expiration/idle timeout, failed-login lockout, password-change session invalidation, audit events, device identity, restrictive CSP, prototype freezing and Tauri command-surface restrictions.

Phase 2 adds SHA-256 payload integrity (standard base64), monotonic per-device sequencing, idempotent replay, bounded exponential backoff, explicit conflict states, and a free-tier logical backup foundation. Service-role credentials never enter the desktop applications.

Phase 3 adds workforce attendance (single open session per worker), shift scheduling and pay records with local-first writes that enqueue into the Phase 2 sync queue.

See `SECURITY.md` for the threat model and security rules.

## Roadmap

1. Identity & Security Core — **complete**
2. Offline Data & Sync Core — **complete**
3. Workforce — **complete**
4. Menu & Orders
5. Kitchen
6. Payments & Cash
7. Inventory, Store & Suppliers
8. Expenses & Equipment
9. Delivery
10. CEO Command Center
11. Production Hardening

## Current phase

**Phase 3 — Workforce — COMPLETE**

See:

- `docs/PHASE-3-STATUS.md`
- `docs/PHASE-3-CHECKLIST.md`

Also still relevant:

- `docs/PHASE-2-STATUS.md`
- `docs/PHASE-2-CHECKLIST.md`
- `docs/PHASE-2-OFFLINE-SYNC.md`
- `docs/PHASE-2-BACKUP.md`

Next: Phase 4 — Menu & Orders.
