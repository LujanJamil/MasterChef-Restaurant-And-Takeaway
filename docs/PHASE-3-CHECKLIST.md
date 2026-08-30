# Phase 3 Verification Checklist — Workforce

## Automated core tests

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --workspace`
- [ ] `cargo test --workspace`
- [ ] workspace `typecheck`
- [ ] workspace `lint`
- [ ] application production builds

## Attendance

- [ ] Clock-in creates a durable local record.
- [ ] Second open clock-in is rejected atomically.
- [ ] Clock-out requires an open session.
- [ ] Break minutes are validated and persisted.
- [ ] Application restart preserves attendance.
- [ ] Invalid worker/device identifiers are rejected.

## Availability and shifts

- [ ] Weekly availability can be created and queried.
- [ ] Invalid weekday/time windows are rejected.
- [ ] DAY, NIGHT, and CUSTOM shifts are supported.
- [ ] Shift end must be after start.
- [ ] Shift lifecycle states are enforced.
- [ ] A worker can be replaced through an auditable swap.
- [ ] Approved swap changes the actual assigned worker.
- [ ] A swap cannot replace a worker with themselves.
- [ ] Overnight NIGHT shifts remain a single logical shift across midnight.

## Payroll

- [ ] CEO-controlled worker/role/shift pay rates support effective dates.
- [ ] Historical pay uses the effective rate at the time of work.
- [ ] Payroll amount is deterministic in minor currency units.
- [ ] Draft → Approved → Paid lifecycle is enforced.
- [ ] Invalid state transitions are rejected.
- [ ] Adjustments require a reason and author.
- [ ] Advances are separately recorded as outstanding liabilities.
- [ ] Worker payments record CASH/MTN_MOMO/AIRTEL_MONEY/OTHER and payer.
- [ ] Outstanding balance is calculated from immutable ledger records.
- [ ] Delayed/partial payment leaves the correct balance owed.
- [ ] No historical pay record is silently rewritten when the CEO changes a current rate.

## Real MasterChef workflows

- [ ] Worker chooses/requests an available day or night shift according to configured availability.
- [ ] Manager/CEO can assign a worker to a shift.
- [ ] Worker can request a swap/replacement.
- [ ] Authorized management can approve the swap.
- [ ] Replacement worker receives the worked shift/pay attribution.
- [ ] Worker can work both day and night shifts on the same calendar day when explicitly assigned.
- [ ] Late arrival can be represented without falsifying clock-in time.
- [ ] Early departure can be represented and later handled through an adjustment policy.
- [ ] Partial shift work can be represented without corrupting the scheduled shift.
- [ ] Absence can be distinguished from a covered/replaced shift.
- [ ] Manager cannot perform CEO-only configuration unless explicitly granted permission.

## Offline and synchronization

- [ ] Every workforce write survives loss of internet.
- [ ] Workforce entities enqueue through the Phase 2 sync boundary.
- [ ] Replaying the same operation does not create duplicates.
- [ ] Failed synchronization retries safely.
- [ ] Conflicts do not silently overwrite history.
- [ ] Restart during pending synchronization preserves the operation.
- [ ] Local records remain readable while cloud services are unavailable.

## Security

- [ ] Authorization is enforced in native/domain code, not only UI controls.
- [ ] Every sensitive workforce mutation has an authenticated actor.
- [ ] CEO overrides are auditable.
- [ ] Ordinary users cannot directly edit historical payroll/payment evidence.
- [ ] Secrets are not committed to source control.
- [ ] Database integrity constraints reject malformed states.

## Manual end-to-end verification

1. Sign in as CEO.
2. Create a worker with an appropriate role.
3. Configure that worker's availability.
4. Create a DAY shift and a NIGHT shift.
5. Set distinct effective pay rates for the worker.
6. Sign in as the worker on the Restaurant application.
7. Clock in/out and confirm the records survive an application restart.
8. Create a replacement request and approve it as authorized management.
9. Generate payroll from actual attendance/shift data.
10. Approve payroll.
11. Record a partial cash payment.
12. Record an advance.
13. Confirm the worker balance equals earned + adjustments − advances − payments.
14. Change the worker's future pay rate and confirm historical payroll remains unchanged.
15. Disconnect internet and repeat a workforce write.
16. Restart while offline.
17. Restore internet and confirm the operation synchronizes exactly once.
18. Inspect audit records for the sensitive actions.

**Gate:** Phase 3 remains `IMPLEMENTATION COMPLETE / VERIFICATION PENDING` until every applicable automated and manual check above is green.
