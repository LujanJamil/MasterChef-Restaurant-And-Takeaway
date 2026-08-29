# Phase 3 Checklist — Workforce

- [x] `packages/workforce-core` crate with SQLite schema for attendance, shifts, pay_records
- [x] Clock-in rejects second open session for same worker
- [x] Clock-out requires open session; records break minutes
- [x] Shift create validates ends_at > starts_at
- [x] Pay create computes amount_minor = round(hours × rate_minor); unique period per worker
- [x] Approve pay transitions DRAFT → APPROVED only
- [x] Domain types exported from `@masterchef/domain`
- [x] Permissions already include attendance.* and pay.* (Phase 1/2)
- [x] Restaurant app: clock-in/out commands + Attendance panel
- [x] CEO app: list attendance / shifts / pay + create shift / create & approve pay
- [x] Supabase migration `202608300002_phase3_workforce.sql` (tables + record_workforce_entity RPC)
- [x] Status docs and README roadmap updated
- [x] Unit tests: clock_in_out_and_hours, shift_and_pay, cannot_clock_out_without_in

## How to pass verification locally

1. `cd packages/workforce-core && cargo test`
2. Ensure Phase 1 bootstrap has a CEO and at least one worker.
3. Restaurant app: sign in → Attendance panel → Clock in → Clock out with break.
4. CEO app: create a shift for the worker; create a pay record for a period; approve it.
5. Optionally enqueue sync ops with entity_type `ATTENDANCE` / `SHIFT` / `PAY` and confirm Phase 2 queue accepts them.
6. Apply Supabase migration on a free-tier project and call `record_workforce_entity` with a valid device token.
