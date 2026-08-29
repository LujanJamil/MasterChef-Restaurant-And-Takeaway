# Phase 3 Status — Workforce

**Status: COMPLETE**

Phase 3 delivers the offline-first workforce core for attendance, shift scheduling and basic payroll, integrated with the existing identity/security boundary and sync queue.

## Delivered guarantees

- Clock-in / clock-out is enforced locally with a single open attendance row per worker.
- Shift assignments are stored with role, location and time bounds.
- Pay records compute amount from hours × rate (minor units, UGX default) and support DRAFT → APPROVED lifecycle.
- All workforce writes are local-first; applications can enqueue corresponding sync operations (entity types `ATTENDANCE`, `SHIFT`, `PAY`) via the existing Phase 2 queue.
- Permission gates (`attendance.view`, `attendance.manage`, `pay.view`, `pay.manage`) are already defined in `@masterchef/permissions` and enforced by the native auth layer before commands execute.
- Supabase migration adds cloud tables + RPC for workforce entity ingest on free-tier.

## Crates / packages

- `packages/workforce-core` — Rust SQLite store (attendance, shifts, pay_records) with unit tests.
- `packages/domain` — TypeScript contracts for AttendanceRecord, Shift, PayRecord.
- Restaurant and CEO Tauri apps expose workforce commands behind authenticated sessions.

## Verification

See `docs/PHASE-3-CHECKLIST.md`. Run workforce-core unit tests and exercise clock-in / shift create / pay create from the restaurant UI on a provisioned device.

## Next phase

Phase 4 — Menu & Orders.
