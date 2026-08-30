# Phase 4 — Menu & Orders

## Status

**IMPLEMENTATION IN PROGRESS / VERIFICATION PENDING**

Canonical location: **Najjera II Central Market**.

Phase 4 must support MasterChef as both a restaurant and takeaway operation and must remain usable when the Internet is unavailable.

## Required capabilities

### Menu
- Categories: local food, snacks, drinks, ice cream, chapati, bread and future categories.
- Products may be introduced, disabled, made temporarily unavailable, or retired without deleting historical references.
- Preparation station routing: kitchen, display, fridge, ice cream, chapati, local food, other.
- Dine-in/takeaway/delivery availability per product.
- CEO-controlled prices with effective-dated price history.
- Historical orders store immutable item-name and unit-price snapshots.
- SKU uniqueness and strict validation.

### Tables
- Configurable table identifiers.
- Dine-in orders require a table.
- Takeaway and delivery orders must not consume a dine-in table.

### Orders
- Dine-in, takeaway and delivery order types.
- Customer name/phone where applicable.
- Waiter attribution.
- Add items to an existing order.
- Change quantities while permitted.
- Customer notes.
- Item-level cancellation with reason and actor.
- Order-level discounts with reason and authorization policy.
- Strict status transitions.
- Full event/audit history.
- No destructive deletion of business history.

### Operational routing
Every item retains its preparation station so later Phase 5 can route work to the correct operational section without rebuilding the order model.

### Offline-first
Every mutation must be durable locally first, represented by a unique operation/event identity, and queued for synchronization. Replaying an operation must not duplicate the business result.

### Security
The frontend is not the security boundary. Tauri commands/native services must authenticate the session and enforce permissions before mutating menu or order state. Sensitive operations must be auditable. Tauri's runtime authority and command permissions are additional defense layers; the application must still enforce business authorization.

## Definition of Done

Phase 4 cannot be marked complete until all of the following pass:

1. TypeScript typecheck.
2. TypeScript lint/compile checks.
3. Rust formatting/checks/tests.
4. Clean production build for CEO and Restaurant applications.
5. Menu category/product/price CRUD acceptance tests.
6. Price effective-date and historical-price tests.
7. Dine-in/takeaway/delivery validation tests.
8. Add-item, quantity, cancellation and discount tests.
9. Order status transition tests, including forbidden transitions.
10. Event/audit persistence tests.
11. Offline create/update/restart tests.
12. Sync queue and replay/idempotency tests.
13. Unauthorized mutation tests for every worker role.
14. Cloud migration and sync integration tests.
15. Backup/restore test covering orders and menu history.
16. Manual restaurant workflow test from customer order through handoff to kitchen/display routing.
17. GitHub CI verification green.

Only then may the roadmap advance to Phase 5 — Kitchen & Preparation.
