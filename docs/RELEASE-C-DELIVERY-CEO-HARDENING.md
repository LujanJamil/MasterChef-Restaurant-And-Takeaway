# Release C — Delivery, CEO Command Center & Production Hardening

## Implemented in this release line

### Delivery
- First-class delivery lifecycle: pending → assigned → picked up → out for delivery → delivered.
- Failed/cancelled delivery states with mandatory reasons.
- Rider assignment.
- Cash, MTN Mobile Money, Airtel Money and prepaid collection methods.
- Amount-due and over-collection invariants.
- Proof reference required before completion.
- Durable local delivery SQLite store.
- Delivery mutations are queued for synchronization with stable entity IDs.

### CEO command center foundation
- Authoritative daily KPI SQL view sourced from persisted orders, expenses, deliveries and stock-loss movements.
- KPI data is server-derived rather than calculated only in the UI.
- CEO application remains protected by the native identity/permission boundary.

### Production hardening
- Tauri launch scripts exposed for both desktop applications.
- CI release gate runs TypeScript verification, Rust checks/tests and dependency audit.
- Sync edge function now rejects missing server configuration instead of silently dereferencing absent secrets.
- Sync CORS is restricted to an explicit configured origin instead of wildcard access.

## Runtime acceptance gate

The following must be demonstrated on a Windows workstation before release sign-off:

1. Launch CEO application with `pnpm dev:ceo`.
2. Launch Restaurant application with `pnpm dev:restaurant`.
3. Bootstrap/login a CEO.
4. Create worker identities and verify role restrictions.
5. Create a delivery order and progress it through rider assignment, pickup, dispatch, collection and proof-of-delivery.
6. Restart the Restaurant application and verify the delivery remains present.
7. Disconnect the network, create mutations, restart, reconnect and verify sync without duplicates.
8. Attempt unauthorized operations with non-CEO accounts and confirm native permission denial.
9. Apply the Release C Supabase migration and verify the KPI view from the protected server boundary.
10. Execute backup and restore procedure and record a successful restore before production use.

## Required production controls

- Never ship a service-role key inside either Tauri application.
- Configure `MASTERCHIEF_ALLOWED_ORIGIN` to the actual trusted application origin in the Supabase function environment.
- Keep production database credentials out of Git.
- Back up before migrations and test restoration, not merely backup creation.
- Keep sync and operational audit logs long enough to investigate payroll, cash, inventory and delivery disputes.
- Release only from a commit where CI and the runtime acceptance checklist are both green.

## Known verification boundary

Automated repository checks can establish compilation, unit tests and static policy. They cannot prove that a real Windows Tauri window launches, that a configured Supabase project has the migration applied, or that a physical offline/reconnect scenario behaves correctly. Those require the workstation acceptance gate above.
