# Release B — Inventory, Store, Suppliers, Expenses & Equipment

Status: IMPLEMENTATION COMPLETE / VERIFICATION PENDING

Release B combines Phase 7 (Inventory / Store / Suppliers) and Phase 8 (Expenses / Equipment).

## Operational model

MasterChef operates one location at Najjera II Central Market. The CEO has unrestricted store authority. The storekeeper has only permissions explicitly granted by the CEO. Kitchen users receive stock through recorded issues; stock is never silently removed.

## Inventory
- Item master with ingredient, packaging, beverage, snack, equipment-consumable and other categories
- Units of measure and controlled conversions
- Minimum/reorder levels
- Opening stock
- Purchases/receiving
- Supplier-linked receiving
- Store issues to kitchen and operational sections
- Returns to store
- Transfers between controlled locations/sections
- Stock counts and count adjustments
- Wastage with reason, actor, timestamp and evidence
- Damage and expiry capture
- Missing/unexplained variance capture
- Immutable stock movement ledger
- Expected versus actual stock
- Stock valuation using an explicit costing policy
- Historical supplier prices and purchase history
- Low-stock and variance alerts

## Supplier management
- Supplier profiles
- Contacts and status
- Supplier products
- Purchase history
- Unit cost history
- Multiple suppliers for one item
- Purchase/receiving references
- Supplier balances/obligations without exposing payment secrets

## Expenses
- Expense categories
- Cash, MTN Mobile Money, Airtel Money and other approved methods
- Expense creation with actor and shift attribution
- Approval thresholds
- CEO override
- Supporting note/reference
- Reversal/correction through compensating records, never destructive edits
- Daily expense totals
- Expense audit history

## Equipment
- Equipment register
- Serial/asset identifiers
- Assigned area
- Responsible worker
- Condition
- Assignment history
- Damage reporting
- Lost/missing reporting
- Repair history
- Replacement history
- Retirement/disposal
- CEO-controlled accountability

## Cross-system integration
Orders and kitchen production consume inventory through explicit stock movements. Purchases increase stock. Wastage and damage decrease stock through attributable movements. Expenses feed financial reporting. Equipment events remain linked to workers/areas and audit history. CEO reporting can reconcile sales, purchases, expenses, stock movements and operational losses.

## Offline-first
All operational writes are durable locally before acknowledgement. Inventory movements, receipts, issues, counts, expenses and equipment events receive stable IDs and sync idempotently when connectivity returns. Replayed operations do not duplicate stock or money.

## Security
Authorization is enforced at the native/service boundary, not only in the UI. CEO-only configuration, stock adjustments, count finalization, expense approval, equipment reassignment and destructive-looking operations require explicit permissions. Audit records identify actor, action, entity, timestamp, device and relevant before/after values.

## Financial and stock invariants
- Monetary values use integer minor units.
- Stock quantities use explicit decimal/integer quantity representation appropriate to the unit; floating point is forbidden for money.
- Every stock change has a source and destination/context.
- Historical purchase costs are immutable.
- Adjustments are additive ledger events, not silent rewrites.
- A finalized stock count cannot be edited; corrections create a new adjustment.
- Reversals reference the original event.

## Verification gate
Before Release B is marked verified it must pass formatting, typechecking, compilation, automated tests, integration tests, authorization tests, stock-ledger invariants, offline/restart tests, sync replay tests, recovery tests, and a production build. Runtime acceptance must cover receiving → store → kitchen issue → production consumption → sale, stock count/variance, supplier price history, expense approval, equipment assignment/damage, offline operation and synchronization.
