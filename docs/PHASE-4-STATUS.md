# Phase 4 — Menu & Orders

## Status
**IMPLEMENTATION COMPLETE / VERIFICATION PENDING**

Location: **Najjera II Central Market**

This phase is considered implementation-complete only when the local application, native command boundary, SQLite persistence, synchronization queue, cloud schema and security controls are all exercised together.

## Implemented

- Dynamic menu categories and menu items.
- Preparation station assignment per menu item.
- Dine-in, takeaway and delivery availability flags.
- UGX integer-minor-unit money storage; no floating-point money arithmetic in the native order core.
- Effective price history and historical price snapshots on order items.
- Restaurant table registry.
- Dine-in order validation requiring a table.
- Takeaway and delivery order creation.
- Sequential local order numbers.
- Item additions with quantity validation and overflow protection.
- Item-level cancellation with mandatory reason and immutable order event history.
- Order-level cancellation with mandatory reason.
- Discounts with non-negative validation and a hard ceiling at subtotal.
- Strict order lifecycle: DRAFT -> SUBMITTED -> PREPARING -> READY -> SERVED -> COMPLETED, with controlled cancellation.
- Actor identity attached to mutations.
- Local SQLite persistence independent of Internet connectivity.
- Synchronization queue entries for menu/order mutations.
- Authenticated Tauri command boundary for every sensitive mutation.
- CEO-only menu/table administration through `system.manage`.
- Role-based order permissions through the Phase 1 security core.
- Cloud PostgreSQL schema with RLS enabled and client grants revoked by default.
- CI quality workflow covering TypeScript, builds and Rust order/application compilation/tests.
- Production Restaurant application screens for menu, order terminal and order lifecycle.

## Security requirements

1. Never trust frontend authorization.
2. Never accept a price supplied by a waiter as authoritative; the native store reads the current menu price.
3. Preserve name and price snapshots on order items.
4. Do not delete operational history to represent cancellation.
5. Require reasons for cancellation and discounts.
6. Keep CEO-only configuration behind native permission checks.
7. Treat synchronization as untrusted input and preserve local auditability.
8. Keep cloud tables inaccessible to anonymous/authenticated clients except through the intended authenticated sync boundary.

## Verification gate

Before merging this phase, run:

```powershell
git pull
pnpm install
pnpm typecheck
pnpm lint
pnpm test
pnpm build
cargo fmt --manifest-path packages/orders-core/Cargo.toml -- --check
cargo test --manifest-path packages/orders-core/Cargo.toml
cargo check --manifest-path apps/restaurant/src-tauri/Cargo.toml
cargo test --manifest-path apps/restaurant/src-tauri/Cargo.toml
```

Then manually verify:

- First-launch CEO bootstrap.
- Authorized login and logout.
- CEO creates category, table and product.
- Non-CEO cannot create menu configuration.
- Waiter creates dine-in order only after selecting a table.
- Waiter creates takeaway order without a table.
- Delivery order can be created for later delivery phase processing.
- Add multiple products and quantities.
- Confirm subtotal and total use integer UGX amounts.
- Add another item to an existing draft/submitted order.
- Cancel one item with a reason and confirm the item remains in history.
- Apply a valid discount and reject one greater than subtotal.
- Reject negative price, zero/negative quantity and invalid order transitions.
- Submit order and progress it through the lifecycle.
- Cancel order with reason and reject cancellation without reason.
- Disconnect Internet and repeat order creation/modification; data must remain available after application restart.
- Restore Internet and verify operations enter/survive the sync queue without duplicate application.
- Verify actor/device attribution in events.
- Verify unauthorized native command attempts are rejected.
- Verify cloud RLS/grants remain deny-by-default.

## Exit rule

Do not mark Phase 5 started until this document is updated from **VERIFICATION PENDING** to **VERIFIED** based on actual command output and manual acceptance evidence.