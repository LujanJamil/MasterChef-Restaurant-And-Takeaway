# Release A — Kitchen + Payments

Status: IMPLEMENTATION COMPLETE / VERIFICATION PENDING

This release combines Phase 5 (Kitchen & Preparation) and Phase 6 (Payments, Cash & Reconciliation).

## Scope
- Preparation stations and station configuration
- Kitchen production queue derived from order items
- Item routing by preparation station
- Queue lifecycle: queued, accepted, preparing, ready, served/packed, cancelled
- Kitchen priority and delay tracking
- Preparation notes and operational events
- Wastage/failed preparation capture with reason and actor
- No silent deletion of kitchen events
- Cashier/payment workflow for cash, MTN Mobile Money, Airtel Money and future methods
- Payment allocation to orders
- Partial payments and remaining balance
- Payment reversals/refunds with authorization and audit trail
- Expected-vs-received reconciliation
- Shift/day reconciliation
- Cash drawer opening, movement and closing balances
- Mobile-money reconciliation records
- Offline-first local persistence and durable sync queue
- Idempotent cloud synchronization
- Native authorization for privileged kitchen/payment actions
- Immutable audit/event history for sensitive mutations
- CEO/manager operational visibility

## Money invariants
All monetary values are integer minor units. No floating-point arithmetic is permitted at financial boundaries. Payment totals may never exceed the order balance without an explicitly authorized overpayment policy. Reversals reference the original payment and cannot mutate historical payment records.

## Kitchen invariants
Every production item has an attributable order/item reference and station. State transitions are explicit and validated. A completed order cannot silently return to an earlier state. Wastage and cancellations retain actor, time and reason.

## Offline invariants
Transactions are written locally before acknowledgement. Synchronization is retryable and idempotent. Replaying the same mutation must not create duplicate payments, kitchen tickets, or order events.

## Security invariants
UI visibility is not authorization. Sensitive commands require native permission checks. Payment reversal, reconciliation finalization, price/payment configuration and privileged kitchen actions require explicit authorized roles and audit records.

## Verification gate
Release A must pass formatting, compilation, unit tests, integration tests, offline/restart tests, synchronization/replay tests, authorization tests, payment reconciliation tests, and production builds before being marked verified.

## Operational acceptance scenarios
1. Order with multiple items routes each item to its assigned station.
2. Kitchen can accept, prepare and complete an item without losing order attribution.
3. Delayed items are visible and measurable.
4. Failed/spoiled preparation is recorded as wastage with a reason.
5. Cash payment closes against the correct order and worker/shift.
6. Mobile-money payment records its provider/reference without storing secrets.
7. Partial payment leaves an exact outstanding balance.
8. Reversal does not rewrite the original payment.
9. Closing reconciliation exposes expected vs actual cash and mobile-money totals.
10. Offline kitchen/payment operations survive restart and synchronize exactly once.
11. Unauthorized roles cannot perform privileged payment or reconciliation actions.
12. Audit history identifies actor, action, timestamp, device and affected entity.
