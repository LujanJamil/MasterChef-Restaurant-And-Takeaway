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

## Architecture

Tauri 2 + React + TypeScript desktop applications with a native Rust security boundary and persistent local SQLite identity/security storage. Phase 2 will extend the local data architecture to restaurant transactions and connect it to PostgreSQL/cloud synchronization.

## Security baseline

Phase 1 includes Argon2id password hashing, individual identities, role-based authorization, per-user permission overrides, CEO authority, session expiration/idle timeout, failed-login lockout, password-change session invalidation, audit events, device identity, restrictive CSP, prototype freezing and Tauri command-surface restrictions.

See `SECURITY.md` for the threat model and security rules.

## Roadmap

1. Identity & Security Core — current
2. Offline Data & Sync Core
3. Workforce
4. Menu & Orders
5. Kitchen
6. Payments & Cash
7. Inventory, Store & Suppliers
8. Expenses & Equipment
9. Delivery
10. CEO Command Center
11. Production Hardening

## Current phase

**Phase 1 — Identity & Security Core**

Implementation branch: `phase-1-identity-security` · Pull request: #1.

Hosted GitHub Actions verification is currently blocked by synthetic `BuildFailed` / `startup_failure` runs with zero jobs in this repository. The verification workflow remains committed and must pass once the GitHub Actions workflow-registration/platform issue clears.
