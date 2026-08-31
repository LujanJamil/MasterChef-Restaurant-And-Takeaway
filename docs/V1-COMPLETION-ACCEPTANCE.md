# MasterChef V1 Completion Acceptance

This document is the final runtime gate. Compilation alone is not certification.

## 1. Automated checks

```powershell
pnpm install --frozen-lockfile
pnpm verify:v1
pnpm verify:rust
```

## 2. Desktop acceptance

Launch both native applications:

```powershell
pnpm dev:restaurant
pnpm dev:ceo
```

Confirm login, logout, role restrictions, worker creation, suspension and audit visibility.

## 3. Workforce acceptance

Create workers for waiter/cook/cashier/storekeeper/rider. Test day and night availability, shift assignment, swap, double shift, clock-in/out, late/absence handling, daily pay, advances/deductions and balance owed. Verify every mutation is attributable to the acting identity.

## 4. Restaurant acceptance

Create a dine-in, takeaway and delivery order. Add multiple items, add an item later, cancel an item with a reason, transition through kitchen states and complete the order. Verify table/customer data and immutable event history.

## 5. Recipe/BOM and stock acceptance

Create an inventory item and supplier price. Create a recipe with yield and ingredient quantities. Complete a test order. Verify exactly one recipe-driven ISSUE movement per order item/recipe, correct quantity including configured waste, and no duplicate movement after replay/retry.

## 6. Costing acceptance

Verify supplier price changes affect newly calculated product cost snapshots without rewriting historical snapshots. Verify selling price, ingredient cost, total cost, gross margin and margin percentage are consistent.

## 7. Money acceptance

Record cash, MTN Mobile Money and Airtel Money payments. Verify payment actor, amount, method, reference and verification state. Reconcile cash by shift and deliberately create a shortage/overage; verify variance is visible and cannot be silently edited.

## 8. Delivery acceptance

Create delivery, assign rider, pickup, dispatch, collect payment, attach proof, deliver. Test failure/cancellation. Verify the collector and amount are attributable and delivery collection is included in reconciliation.

## 9. CEO remote acceptance

With the restaurant device connected, verify that the CEO application receives synchronized operational data. Disconnect the restaurant device, create transactions, restart the application, reconnect and verify idempotent synchronization. No transaction may disappear or duplicate.

## 10. Offline acceptance

Disable Internet and continue operating. Close/reopen the application. Verify local records remain available. Reconnect and verify queue drains without duplicates or corruption.

## 11. Backup/restore acceptance

Run:

```powershell
pnpm backup
```

Confirm the backup contains application data, a manifest, file SHA-256 hashes and an archive checksum. Restore into a controlled test environment and verify workers, orders, payments, stock, audit and sync state. Never test destructive restore against the live restaurant database.

## 12. Security acceptance

Attempt unauthorized actions as every non-CEO role: price change, refund, discount, worker administration, store adjustment, cash approval, audit access and CEO reporting. Every forbidden operation must be rejected by the native authorization boundary, not merely hidden by the UI.

## 13. Production acceptance

Build release artifacts on the target Windows machine. Install, upgrade, uninstall/reinstall, launch after reboot and verify application data persistence. Confirm backup scheduling, secret configuration, Supabase migrations and sync endpoint configuration.

## Release status rule

Only after every section above passes should the project status change from **Implementation Complete / Verification Pending** to **V1 Runtime Accepted**. Automated green checks do not replace the manual operational tests.
