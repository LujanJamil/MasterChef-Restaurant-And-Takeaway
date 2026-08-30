import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type AttendanceStatus = "OPEN" | "CLOSED" | "ADJUSTED";
interface Attendance {
  id: string;
  worker_id: string;
  device_id: string;
  clock_in_at: number;
  clock_out_at: number | null;
  break_minutes: number;
  status: AttendanceStatus;
  notes: string | null;
  created_at: number;
  updated_at: number;
}

type ShiftType = "DAY" | "NIGHT" | "CUSTOM";
interface Shift {
  id: string;
  worker_id: string;
  role: string;
  shift_type: ShiftType;
  starts_at: number;
  ends_at: number;
  location: string;
  status: string;
  created_by: string;
  created_at: number;
  updated_at: number;
}

const SHIFT_STATUS: Record<string, string> = {
  SCHEDULED: "Scheduled",
  PRESENT: "Present",
  ABSENT: "Absent",
  EXCUSED: "Excused",
  CANCELLED: "Cancelled",
  REPLACED: "Replaced",
  COMPLETED: "Completed",
};

const fmtDateTime = (t: number) =>
  new Date(t * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
const fmtDuration = (s: number) => {
  const h = Math.floor(s / 3600);
  const m = Math.round((s % 3600) / 60);
  return `${h}h ${m}m`;
};
const workedSeconds = (a: Attendance) =>
  a.clock_out_at == null ? 0 : Math.max(0, a.clock_out_at - a.clock_in_at - a.break_minutes * 60);
const isToday = (t: number) => new Date(t * 1000).toDateString() === new Date().toDateString();

export default function WorkforcePanel({ token, onError }: { token: string; onError?: (message: string) => void }) {
  const [open, setOpen] = useState<Attendance | null>(null);
  const [records, setRecords] = useState<Attendance[]>([]);
  const [shifts, setShifts] = useState<Shift[]>([]);
  const [breakMinutes, setBreakMinutes] = useState(30);
  const [notes, setNotes] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const now = Math.floor(Date.now() / 1000);
      const [o, r, s] = await Promise.all([
        invoke<Attendance | null>("workforce_open_attendance", { token }),
        invoke<Attendance[]>("workforce_list_attendance", { token, limit: 20 }),
        invoke<Shift[]>("workforce_list_shifts", { token, from: now - 86400, to: now + 86400 * 14, limit: 40 }),
      ]);
      setOpen(o);
      setRecords(r);
      setShifts(s);
    } catch (e) {
      onError?.(String(e));
    }
  }, [token, onError]);

  useEffect(() => {
    void load();
  }, [load]);

  async function clockIn() {
    setBusy(true);
    try {
      await invoke<Attendance>("workforce_clock_in", { token, notes: notes.trim() || null });
      setNotes("");
      await load();
    } catch (e) {
      onError?.(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function clockOut() {
    setBusy(true);
    try {
      await invoke<Attendance>("workforce_clock_out", { token, breakMinutes, notes: notes.trim() || null });
      setNotes("");
      await load();
    } catch (e) {
      onError?.(String(e));
    } finally {
      setBusy(false);
    }
  }

  const nowSec = Math.floor(Date.now() / 1000);
  const elapsed = open ? fmtDuration(Math.max(0, nowSec - open.clock_in_at - open.break_minutes * 60)) : "—";
  const todaySeconds = records
    .filter((r) => r.status === "CLOSED" && isToday(r.clock_in_at))
    .reduce((a, r) => a + workedSeconds(r), 0);

  return (
    <section className="content">
      <div className="section-head">
        <div>
          <div className="eyebrow">WORKFORCE · PHASE 3</div>
          <h1>Attendance &amp; shifts</h1>
          <p>Clock in and out from this application. Records are written to the local workforce ledger before synchronization.</p>
        </div>
        <button className="secondary" onClick={() => void load()}>Refresh</button>
      </div>

      <section className="grid">
        <article className="card">
          <span className="eyebrow">MY SHIFT</span>
          {open ? (
            <>
              <h2>On duty</h2>
              <p>Clocked in {fmtDateTime(open.clock_in_at)} · elapsed <b>{elapsed}</b></p>
              <div className="form-row">
                <input
                  type="number"
                  min={0}
                  max={1440}
                  value={breakMinutes}
                  onChange={(e) => setBreakMinutes(Math.max(0, Math.min(1440, Number(e.target.value) || 0)))}
                  title="Break minutes"
                  aria-label="Break minutes"
                />
                <input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="Notes (optional)" aria-label="Notes" />
              </div>
              <button className="danger" disabled={busy} onClick={() => void clockOut()}>{busy ? "Working…" : "Clock out"}</button>
            </>
          ) : (
            <>
              <h2>Off duty</h2>
              <p>No open attendance session.</p>
              <div className="form-row">
                <input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="Notes (optional)" aria-label="Notes" />
                <button disabled={busy} onClick={() => void clockIn()}>{busy ? "Working…" : "Clock in"}</button>
              </div>
            </>
          )}
          <small>Break minutes are deducted when clocking out (0–1440).</small>
        </article>

        <article className="card">
          <span className="eyebrow">TODAY</span>
          <h2>{fmtDuration(todaySeconds)}</h2>
          <p>Closed sessions today{open ? " · one session still open" : ""}.</p>
        </article>

        <article className="card">
          <span className="eyebrow">SHIFTS · NEXT 14 DAYS</span>
          <h2>{shifts.length}</h2>
          <p>Your scheduled shifts ahead.</p>
        </article>
      </section>

      <div className="table-card">
        <h2>Recent attendance</h2>
        <table>
          <thead>
            <tr><th>Date</th><th>Clock in</th><th>Clock out</th><th>Break</th><th>Worked</th><th>Status</th><th>Notes</th></tr>
          </thead>
          <tbody>
            {records.length === 0 && <tr><td colSpan={7}>No attendance records yet — clock in to start.</td></tr>}
            {records.map((r) => (
              <tr key={r.id}>
                <td>{new Date(r.clock_in_at * 1000).toLocaleDateString()}</td>
                <td>{fmtDateTime(r.clock_in_at)}</td>
                <td>{r.clock_out_at ? fmtDateTime(r.clock_out_at) : "—"}</td>
                <td>{r.break_minutes} min</td>
                <td>{r.clock_out_at ? fmtDuration(workedSeconds(r)) : "—"}</td>
                <td><span className={`pill ${r.status === "OPEN" ? "pill-ok" : r.status === "ADJUSTED" ? "pill-warn" : "pill-muted"}`}>{r.status}</span></td>
                <td>{r.notes ?? ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="table-card">
        <h2>Upcoming shifts</h2>
        <table>
          <thead>
            <tr><th>Starts</th><th>Ends</th><th>Type</th><th>Role</th><th>Location</th><th>Status</th></tr>
          </thead>
          <tbody>
            {shifts.length === 0 && <tr><td colSpan={6}>No shifts scheduled in the next 14 days.</td></tr>}
            {shifts.map((s) => (
              <tr key={s.id}>
                <td>{fmtDateTime(s.starts_at)}</td>
                <td>{fmtDateTime(s.ends_at)}</td>
                <td>{s.shift_type}</td>
                <td>{s.role.replace("_", " ")}</td>
                <td>{s.location}</td>
                <td><span className={`pill ${s.status === "PRESENT" || s.status === "COMPLETED" ? "pill-ok" : s.status === "ABSENT" || s.status === "CANCELLED" ? "pill-warn" : "pill-muted"}`}>{SHIFT_STATUS[s.status] ?? s.status}</span></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
