# MasterChef Restaurant & Takeaway

MasterChef is an application-first, offline-first restaurant operating system for the Master Restaurant with Master Chefs at **Najjera II Central Market**.

## Applications

- `apps/ceo` — dedicated CEO desktop application
- `apps/restaurant` — dedicated restaurant operations desktop application
- `packages/domain` — shared business types and rules
- `packages/permissions` — capability-based authorization
- `packages/audit` — audit event contracts
- `packages/validation` — shared runtime validation

## Architecture

Tauri 2 + React + TypeScript desktop applications, with a local-first SQLite layer planned for Phase 2 and a PostgreSQL cloud synchronization layer planned for Phase 2.

## Roadmap

1. Identity & Security Core
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

The Phase 1 implementation is being developed on `phase-1-identity-security` and reviewed in PR #1.
