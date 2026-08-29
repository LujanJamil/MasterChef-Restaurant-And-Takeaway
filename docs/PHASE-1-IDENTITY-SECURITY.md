# Phase 1 — Identity & Security Core

## Objective
Build the security foundation before financial, stock or workforce transactions are introduced.

## Implemented in this phase
- Two Tauri desktop application shells: CEO and Restaurant.
- Shared TypeScript domain package.
- Capability-based permissions package.
- CEO authority/override rule.
- Zod validation package.
- Audit event contract.
- Individual worker identity model.
- Correct restaurant location constant: Najjera II Central Market.

## Security rules
1. Every worker receives an individual identity.
2. Permissions are capability-based, not only role-based.
3. CEO is the ultimate authority and can override configured restrictions.
4. CEO overrides must be auditable.
5. Critical records will not be silently deleted.
6. Authentication persistence and secure local credential storage are deliberately connected to the Phase 2 local database rather than pretending the Phase 1 demo login is production authentication.

## Next phase
Phase 2 will add SQLite local persistence, device identity, sync queue, cloud PostgreSQL persistence, idempotency, conflict handling and local/cloud backup foundations.
