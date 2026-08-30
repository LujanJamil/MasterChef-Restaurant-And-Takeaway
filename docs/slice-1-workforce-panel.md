# Slice 1 — Workforce Panel (Phase 3 UI)

**Commit:** `cb7fb68` — "feat(workforce): wire Phase 3 attendance and shifts UI to native core"
**Date:** 2026-08-30
**App:** `apps/restaurant` (Tauri 2 + React + TypeScript)
**Verified in this environment:** typecheck ✅ · production build ✅

## What was built

The restaurant desktop application now has a real **Workforce (Attendance & Shifts)** panel. Phase 3's native core — attendance, shifts, availability, pay — was already implemented in Rust (`packages/workforce-core`, exposed as `workforce_*` commands in `apps/restaurant/src-tauri/src/main.rs`), but the UI only showed a placeholder paragraph. This slice wires the UI to the existing native commands — no Rust changes were required.

### Features

- **Clock in / Clock out** with optional notes and break minutes (0–1440)
- **Single open session rule** enforced by the native core (the open session is returned to the UI; a second clock-in is rejected natively)
- **"My shift" card** — clock-in time plus live elapsed time on duty
- **"Today" card** — total worked time from closed sessions
- **Shifts overview** — next 14 days: start, end, type (DAY/NIGHT/CUSTOM), role, location, status pill
- **Attendance history table** — date, clock in/out, break, worked duration, status (OPEN / CLOSED / ADJUSTED), notes
- **Refresh button** + auto-load on panel open

## Files changed

| File | Change | Purpose |
|---|---|---|
| `apps/restaurant/src/components/WorkforcePanel.tsx` | +218 | New component: attendance + shifts UI invoking the native `workforce_*` commands |
| `apps/restaurant/src/App.tsx` | 3 lines (+2 −1) | Import `WorkforcePanel`; replace placeholder Attendance section with `<WorkforcePanel token={session.token} onError={setError}/>` |
| `apps/restaurant/src/styles.css` | +11 | Add `.table-card`, `.pill`, `.pill-ok`, `.pill-warn`, `.pill-muted` styles |

**Total: 3 files changed, 231 insertions, 1 deletion** (verified via `git show --stat cb7fb68`).

## Data flow

1. User opens the **Attendance** tab → `App.tsx` renders `<WorkforcePanel token={session.token} .../>`
2. On mount the component calls three native commands in parallel:
   - `workforce_open_attendance` → the current open session (if any)
   - `workforce_list_attendance` (limit 20) → recent records
   - `workforce_list_shifts` (from now−1 day, to now+14 days, limit 40) → upcoming shifts
3. **Clock in** → `workforce_clock_in` (notes) → native core rejects if an OPEN session exists (`AlreadyClockedIn`) → list refreshes
4. **Clock out** → `workforce_clock_out` (breakMinutes, notes) → native core validates break 0–1440 and closes the OPEN session → list refreshes
5. All writes land in the local SQLite workforce ledger (`masterchef-restaurant-workforce.sqlite3`). Nothing touches the network; sync happens through the existing Phase 2 queue.

## What was NOT changed

- No Rust / `src-tauri` changes (the native commands already existed in the invoke handler)
- No SQL migrations (workforce schema already lives in `workforce-core`)
- No `@masterchef/domain` changes
- No CEO app changes
- No behavior change to menu / orders

## Known gaps / deferred (future slices)

- Shift availability rules, swap requests, and payroll UI are not yet surfaced (the native core already has `set_availability`, swaps, pay records and `payroll_summary`)
- Break minutes are entered at clock-out and recorded; not tracked in real time
- No attendance adjustment UI (native `ADJUSTED` status exists but is not exposed)
- Attendance list shows the signed-in worker's own records (authentication is per-user)

## How to test locally

### Prerequisites

- Node.js ≥ 20 (verified here with v22.23.2)
- pnpm 10.15.0 — `corepack enable && corepack prepare pnpm@10.15.0 --activate`
- Rust toolchain (stable) — required by Tauri
- Tauri 2 system dependencies (see https://tauri.app/start/prerequisites/; on Linux: `libwebkit2gtk-4.1-dev`, `build-essential`, etc.)

### 1. Install dependencies

```bash
pnpm install
```

### 2. Typecheck (verified ✅ exit 0)

```bash
pnpm --filter @masterchef/restaurant typecheck
```

### 3. Production build (verified ✅ exit 0)

```bash
pnpm --filter @masterchef/restaurant build
```

Expected output ends with `✓ built in ~1.2s` and emits `dist/` assets (index.html 0.37 kB, JS ~212 kB, CSS ~4.4 kB).

### 4. Run the desktop app

```bash
pnpm dev:restaurant
```

Equivalent: `pnpm --filter @masterchef/restaurant tauri dev`. This requires the Rust toolchain and a desktop environment. On first launch the app's one-time CEO bootstrap applies (same as before — the security core is unchanged).

### 5. Expected UI behavior

1. Sign in with a worker account (or the CEO account).
2. Click **Attendance** in the navigation.
3. Click **Clock in** → the "My shift" card switches to "On duty" with clock-in time and elapsed time.
4. Enter a break value (e.g. 30) and click **Clock out** → the session closes, the "Today" total updates, and the record appears under "Recent attendance" with status **CLOSED** and worked time = clock-out − clock-in − break.
5. Clicking **Clock in** again starts a new session. Clicking **Clock in** while a session is open shows the native error message ("already clocked in").
6. "Upcoming shifts" lists scheduled shifts (empty until the CEO/scheduler assigns shifts).
7. **Refresh** reloads attendance + shifts without a page reload.

## Verification evidence (this environment)

| Check | Command | Result |
|---|---|---|
| Typecheck | `pnpm --filter @masterchef/restaurant typecheck` | exit 0 |
| Production build | `pnpm --filter @masterchef/restaurant build` | exit 0 — dist/index.html 0.37 kB, CSS 4.44 kB, JS 211.83 kB, built in 1.18s |
| Commit stat | `git show --stat cb7fb68` | 3 files, +231 −1 |

## Next slices

1. Kitchen display — station queue from orders, accept → prepare → ready, delay flags, waste capture
2. Cashier / Payments — cash, MTN, Airtel, cash sessions, expected-vs-actual reconciliation
3. Real inventory wiring — replace the mock Release B screen with persisted stock movements, suppliers, expenses, equipment
4. CEO Command Center — dashboard with today's sales, orders, payments, expenses, workers present/absent, low stock, sync freshness
