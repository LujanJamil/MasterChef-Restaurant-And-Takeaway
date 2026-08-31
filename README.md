# MasterChef Restaurant & Takeaway

MasterChef is an application-first, offline-first restaurant operating system for the Master Restaurant at **Najjera II Central Market**.

## V1 completion target

V1 replaces dependence on verbal instructions, memory and notebooks with an attributable operational system. The CEO application is designed to let the CEO monitor the restaurant remotely after secure cloud synchronization is configured and runtime acceptance passes.

## Applications

- `apps/ceo` — dedicated CEO desktop command application
- `apps/restaurant` — dedicated restaurant operations desktop application

## Core

- `packages/domain` — shared business rules
- `packages/permissions` — authorization contracts
- `packages/audit` — audit contracts
- `packages/validation` — shared validation
- `packages/native-core` — Rust identity, authorization, sessions and security persistence
- `packages/sync-core` — offline transaction queue, integrity, sequencing, retry and conflicts
- `packages/workforce-core` — attendance, shifts and payroll
- `packages/orders-core` — menu, tables and order lifecycle
- `packages/inventory-core` — stock ledger and invariants
- `packages/delivery-core` — delivery lifecycle and collections

## V1 business coverage

### Workforce
Individual accounts for CEO, manager, waiter, cook, cashier, storekeeper and delivery rider. Day/night shifts, availability, worker-selected shifts, swaps, double shifts, attendance, lateness/absence, daily pay, different rates, advances, deductions, delayed balances and auditable payments are part of the workforce model. CEO authority can override settings.

### Restaurant operations
Dine-in, takeaway and delivery orders; tables; dynamic menu and prices; kitchen stations; order routing; additions; cancellations with reasons; unavailable products; and order lifecycle events.

### Money
Cash, MTN Mobile Money and Airtel Money are distinct methods. V1 includes payment records, verification state, references, cash reconciliation and expected-versus-counted variance. Delivery collections are attributable to the collecting actor.

### Store
Inventory, units, reorder levels, suppliers, supplier price history, receipts, issues, returns, transfers, wastage, damage, expiry, physical counts, variance and equipment accountability.

### Recipes, costing and profitability
Recipes/BOMs connect menu items to ingredients and yields. Completing an order can automatically create idempotent recipe-driven stock consumption. Supplier prices feed product cost snapshots and gross-margin reporting.

### CEO Command Center
The CEO layer is designed around persisted operational data: sales, orders, payment methods, expenses, workforce, stock, low-stock alerts, deliveries, reconciliation variance, product margins and operational health.

### Notifications
Operational notifications support INFO, WARNING and CRITICAL severity plus acknowledgement/resolution. Low-stock notification generation is included in the database automation layer.

### Offline and backup
Desktop SQLite remains authoritative for local operation. Mutations enter the durable sync queue. Backup scripts and backup metadata are included. Runtime offline/reconnect and backup/restore tests remain mandatory before production use.

## V1 database completion layer

`supabase/migrations/202609010001_masterchef_v1_completion.sql` adds recipe/BOM, cost layers, product cost snapshots, payments, cash and stock reconciliation, notifications, backup metadata and CEO reporting views.

`supabase/migrations/202609010002_masterchef_v1_automation.sql` adds automatic recipe stock consumption, product-cost calculation and low-stock notification generation.

## V2 backlog

Intentionally deferred from the V1 completion gate: advanced waste/loss anomaly intelligence, customer complaints and CRM/history, loyalty/promotions, supplier forecasting, demand forecasting/automated purchasing, advanced delivery routing, multi-location support, advanced BI and SMS/WhatsApp/push notification channels.

## Verification gate

A green compile is necessary but not sufficient. Before production, run `pnpm install`, `pnpm typecheck`, `pnpm lint`, `pnpm test`, `pnpm build`, Rust checks/tests, launch both Tauri applications, apply and verify Supabase migrations, then execute full order→kitchen→recipe stock→payment→reconciliation, delivery→collection→proof→reconciliation, offline→restart→reconnect→sync, backup→destructive test→restore, role/permission negative tests and Windows packaging/install/update tests.

Until those runtime checks are demonstrated, the correct status is **Implementation Complete / Verification Pending**.

## Location

**Najjera II Central Market**
