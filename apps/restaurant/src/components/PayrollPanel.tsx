import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Worker { id: string; username: string; display_name: string; role: string; status: string; created_at: number; }
interface Shift { id: string; worker_id: string; role: string; shift_type: string; starts_at: number; ends_at: number; location: string; status: string; created_by: string; created_at: number; updated_at: number; }
interface ShiftSwap { id: string; shift_id: string; from_worker_id: string; to_worker_id: string; reason: string | null; status: string; requested_by: string; approved_by: string | null; created_at: number; updated_at: number; }
interface PayRecord { id: string; worker_id: string; period_start: number; period_end: number; hours_worked: number; rate_minor: number; amount_minor: number; currency: string; status: string; created_by: string; created_at: number; updated_at: number; }
interface PayrollSummary { gross_minor: number; adjustments_minor: number; advances_minor: number; paid_minor: number; balance_minor: number; }

const ROLES = ["MANAGER", "WAITER", "COOK", "CASHIER", "STOREKEEPER", "DELIVERY_RIDER"];
const TYPES = ["DAY", "NIGHT", "CUSTOM"];
const METHODS = ["CASH", "MTN_MOMO", "AIRTEL_MONEY"];
const money = (n: number) => `UGX ${n.toLocaleString()}`;
const fmt = (t: number) => new Date(t * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
const epoch = (v: string) => Math.floor(new Date(v).getTime() / 1000);
const nowSec = () => Math.floor(Date.now() / 1000);

export default function PayrollPanel({ token, role, userId, onError }: { token: string; role: string; userId: string; onError?: (message: string) => void }) {
  const canManage = ["CEO", "MANAGER"].includes(role); // native core enforces the final decision
  const [tab, setTab] = useState<"shifts" | "swaps" | "payroll">("shifts");
  const [workers, setWorkers] = useState<Worker[]>([]);
  const [shifts, setShifts] = useState<Shift[]>([]);
  const [myShifts, setMyShifts] = useState<Shift[]>([]);
  const [swaps, setSwaps] = useState<ShiftSwap[]>([]);
  const [pay, setPay] = useState<PayRecord[]>([]);
  const [summary, setSummary] = useState<PayrollSummary | null>(null);
  const [selWorker, setSelWorker] = useState("");
  const [busy, setBusy] = useState(false);

  const [shiftForm, setShiftForm] = useState({ workerId: "", role: "WAITER", type: "DAY", start: "", end: "", location: "MAIN" });
  const [swapForm, setSwapForm] = useState({ shiftId: "", toWorkerId: "", reason: "" });
  const [rateForm, setRateForm] = useState({ role: "WAITER", type: "DAY", rate: "", effective: "" });
  const [adjustForm, setAdjustForm] = useState({ kind: "BONUS", amount: "", reason: "" });
  const [advanceForm, setAdvanceForm] = useState({ amount: "", reason: "" });
  const [paymentForm, setPaymentForm] = useState({ amount: "", method: "CASH", reference: "" });
  const [shiftPayId, setShiftPayId] = useState("");

  const name = useCallback((id: string) => workers.find((w) => w.id === id)?.display_name ?? id.slice(0, 8), [workers]);

  const load = useCallback(async () => {
    try {
      const to = nowSec() + 86400 * 30;
      const calls: Promise<void>[] = [
        invoke<ShiftSwap[]>("workforce_list_swaps", { token, status: null, limit: 100 }).then((s) => setSwaps(s)),
        invoke<Shift[]>("workforce_list_shifts", { token, from: nowSec() - 86400, to, limit: 50 }).then((s) => setMyShifts(s)),
      ];
      if (canManage) {
        calls.push(invoke<Worker[]>("users_list", { token }).then((w) => setWorkers(w.filter((x) => x.status === "ACTIVE"))));
        calls.push(invoke<Shift[]>("workforce_list_shifts_admin", { token, workerId: null, from: nowSec() - 86400, to, limit: 100 }).then((s) => setShifts(s)));
      }
      await Promise.all(calls);
    } catch (e) { onError?.(String(e)); }
  }, [token, canManage, onError]);

  const loadPay = useCallback(async (workerId: string) => {
    if (!workerId) return;
    try {
      const [records, sum] = await Promise.all([
        invoke<PayRecord[]>("workforce_list_pay", { token, workerId, limit: 50 }),
        invoke<PayrollSummary>("workforce_payroll_summary", { token, workerId }),
      ]);
      setPay(records); setSummary(sum);
    } catch (e) { onError?.(String(e)); }
  }, [token, onError]);

  useEffect(() => { void load(); }, [load]);
  useEffect(() => { if (selWorker) void loadPay(selWorker); }, [selWorker, loadPay]);

  const run = useCallback(async (fn: () => Promise<unknown>) => {
    setBusy(true);
    try { await fn(); await load(); if (selWorker) await loadPay(selWorker); } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }, [load, loadPay, selWorker, onError]);

  const selWorkerShifts = useMemo(() => shifts.filter((s) => s.worker_id === selWorker && s.status !== "CANCELLED"), [shifts, selWorker]);

  return (
    <section className="content">
      <div className="section-head">
        <div>
          <div className="eyebrow">WORKFORCE · PHASE 3 COMPLETION</div>
          <h1>Shifts, swaps &amp; payroll</h1>
          <p>Scheduling, swap approval and pay records write to the local workforce ledger and synchronize with stable event IDs.</p>
        </div>
        <button className="secondary" onClick={() => void load()}>Refresh</button>
      </div>

      <div className="toolbar">
        {(["shifts", "swaps", ...(canManage ? ["payroll"] : [])] as ("shifts" | "swaps" | "payroll")[]).map((t) => (
          <button key={t} className={`secondary ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t === "shifts" ? "Shifts" : t === "swaps" ? "Swaps" : "Payroll"}
          </button>
        ))}
      </div>

      {tab === "shifts" && (
        <>
          {canManage && (
            <div className="card admin">
              <h2>Assign shift</h2>
              <div className="form-row">
                <select value={shiftForm.workerId} onChange={(e) => setShiftForm((x) => ({ ...x, workerId: e.target.value }))}>
                  <option value="">Select worker…</option>
                  {workers.map((w) => <option key={w.id} value={w.id}>{w.display_name} · {w.role}</option>)}
                </select>
                <select value={shiftForm.role} onChange={(e) => setShiftForm((x) => ({ ...x, role: e.target.value }))}>{ROLES.map((r) => <option key={r}>{r}</option>)}</select>
                <select value={shiftForm.type} onChange={(e) => setShiftForm((x) => ({ ...x, type: e.target.value }))}>{TYPES.map((t) => <option key={t}>{t}</option>)}</select>
              </div>
              <div className="form-row">
                <input type="datetime-local" value={shiftForm.start} onChange={(e) => setShiftForm((x) => ({ ...x, start: e.target.value }))} title="Shift start" aria-label="Shift start" />
                <input type="datetime-local" value={shiftForm.end} onChange={(e) => setShiftForm((x) => ({ ...x, end: e.target.value }))} title="Shift end" aria-label="Shift end" />
                <input placeholder="Location" value={shiftForm.location} onChange={(e) => setShiftForm((x) => ({ ...x, location: e.target.value }))} />
                <button disabled={busy || !shiftForm.workerId || !shiftForm.start || !shiftForm.end}
                  onClick={() => void run(() => invoke("workforce_create_shift", { token, workerId: shiftForm.workerId, role: shiftForm.role, shiftType: shiftForm.type, startsAt: epoch(shiftForm.start), endsAt: epoch(shiftForm.end), location: shiftForm.location }))}>
                  Assign
                </button>
              </div>
            </div>
          )}
          <div className="table-card">
            <h2>{canManage ? "All shifts · next 30 days" : "My shifts"}</h2>
            <table>
              <thead><tr>{canManage && <th>Worker</th>}<th>Starts</th><th>Ends</th><th>Type</th><th>Role</th><th>Location</th><th>Status</th></tr></thead>
              <tbody>
                {(canManage ? shifts : myShifts).length === 0 && <tr><td colSpan={7}>No shifts scheduled.</td></tr>}
                {(canManage ? shifts : myShifts).map((s) => (
                  <tr key={s.id}>
                    {canManage && <td>{name(s.worker_id)}</td>}
                    <td>{fmt(s.starts_at)}</td><td>{fmt(s.ends_at)}</td><td>{s.shift_type}</td><td>{s.role}</td><td>{s.location}</td>
                    <td><span className={`pill ${s.status === "COMPLETED" || s.status === "PRESENT" ? "pill-ok" : s.status === "CANCELLED" || s.status === "ABSENT" ? "pill-warn" : "pill-muted"}`}>{s.status}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {tab === "swaps" && (
        <>
          <div className="card composer">
            <h2>Request a swap</h2>
            <div className="form-row">
              <select value={swapForm.shiftId} onChange={(e) => setSwapForm((x) => ({ ...x, shiftId: e.target.value }))}>
                <option value="">Select your shift…</option>
                {myShifts.filter((s) => s.status === "SCHEDULED").map((s) => (
                  <option key={s.id} value={s.id}>{fmt(s.starts_at)} · {s.shift_type} · {s.role}</option>
                ))}
              </select>
              <select value={swapForm.toWorkerId} onChange={(e) => setSwapForm((x) => ({ ...x, toWorkerId: e.target.value }))}>
                <option value="">Covering worker…</option>
                {workers.filter((w) => w.id !== userId).map((w) => <option key={w.id} value={w.id}>{w.display_name}</option>)}
              </select>
              <input placeholder="Reason (optional)" value={swapForm.reason} onChange={(e) => setSwapForm((x) => ({ ...x, reason: e.target.value }))} />
              <button disabled={busy || !swapForm.shiftId || !swapForm.toWorkerId}
                onClick={() => void run(() => invoke("workforce_request_swap", { token, shiftId: swapForm.shiftId, toWorkerId: swapForm.toWorkerId, reason: swapForm.reason || null }))}>
                Request swap
              </button>
            </div>
          </div>
          <div className="table-card">
            <h2>{canManage ? "All swap requests" : "My swap requests"}</h2>
            <table>
              <thead><tr><th>Requested</th><th>From</th><th>To</th><th>Reason</th><th>Status</th>{canManage && <th></th>}</tr></thead>
              <tbody>
                {swaps.length === 0 && <tr><td colSpan={6}>No swap requests.</td></tr>}
                {swaps.map((s) => (
                  <tr key={s.id}>
                    <td>{fmt(s.created_at)}</td><td>{name(s.from_worker_id)}</td><td>{name(s.to_worker_id)}</td><td>{s.reason ?? ""}</td>
                    <td><span className={`pill ${s.status === "APPROVED" ? "pill-ok" : s.status === "REJECTED" ? "pill-warn" : "pill-muted"}`}>{s.status}</span></td>
                    {canManage && (
                      <td>{s.status === "REQUESTED" && (
                        <button disabled={busy} onClick={() => void run(() => invoke("workforce_approve_swap", { token, swapId: s.id }))}>Approve</button>
                      )}</td>
                    )}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {tab === "payroll" && canManage && (
        <>
          <div className="card composer">
            <h2>Worker</h2>
            <div className="form-row">
              <select value={selWorker} onChange={(e) => setSelWorker(e.target.value)}>
                <option value="">Select worker…</option>
                {workers.map((w) => <option key={w.id} value={w.id}>{w.display_name} · {w.role}</option>)}
              </select>
            </div>
          </div>

          {selWorker && summary && (
            <>
              <section className="grid">
                <article className="card"><span className="eyebrow">GROSS EARNED</span><h2>{money(summary.gross_minor)}</h2></article>
                <article className="card"><span className="eyebrow">ADJUSTMENTS</span><h2>{money(summary.adjustments_minor)}</h2></article>
                <article className="card"><span className="eyebrow">ADVANCES OUT</span><h2>{money(summary.advances_minor)}</h2></article>
                <article className="card"><span className="eyebrow">PAID</span><h2>{money(summary.paid_minor)}</h2></article>
                <article className="card"><span className="eyebrow">BALANCE DUE</span><h2>{money(summary.balance_minor)}</h2></article>
              </section>

              <div className="card admin">
                <h2>Set pay rate</h2>
                <div className="form-row">
                  <select value={rateForm.role} onChange={(e) => setRateForm((x) => ({ ...x, role: e.target.value }))}>{ROLES.map((r) => <option key={r}>{r}</option>)}</select>
                  <select value={rateForm.type} onChange={(e) => setRateForm((x) => ({ ...x, type: e.target.value }))}>{TYPES.map((t) => <option key={t}>{t}</option>)}</select>
                  <input type="number" min="1" placeholder="Rate per shift (UGX)" value={rateForm.rate} onChange={(e) => setRateForm((x) => ({ ...x, rate: e.target.value }))} />
                  <input type="datetime-local" value={rateForm.effective} onChange={(e) => setRateForm((x) => ({ ...x, effective: e.target.value }))} title="Effective from" aria-label="Effective from" />
                  <button disabled={busy || !rateForm.rate || !rateForm.effective}
                    onClick={() => void run(() => invoke("workforce_set_pay_rate", { token, workerId: selWorker, role: rateForm.role, shiftType: rateForm.type, rateMinor: Number(rateForm.rate), effectiveFrom: epoch(rateForm.effective) }))}>
                    Set rate
                  </button>
                </div>
              </div>

              <div className="card admin">
                <h2>Generate pay from shift</h2>
                <div className="form-row">
                  <select value={shiftPayId} onChange={(e) => setShiftPayId(e.target.value)}>
                    <option value="">Select completed shift…</option>
                    {selWorkerShifts.map((s) => <option key={s.id} value={s.id}>{fmt(s.starts_at)} · {s.shift_type} · {s.status}</option>)}
                  </select>
                  <button disabled={busy || !shiftPayId}
                    onClick={() => void run(() => invoke("workforce_create_shift_pay", { token, workerId: selWorker, shiftId: shiftPayId }))}>
                    Create pay record
                  </button>
                </div>
              </div>

              <div className="card admin">
                <h2>Adjustment · advance · payment</h2>
                <div className="form-row">
                  <select value={adjustForm.kind} onChange={(e) => setAdjustForm((x) => ({ ...x, kind: e.target.value }))}>{["BONUS", "DEDUCTION", "CORRECTION"].map((k) => <option key={k}>{k}</option>)}</select>
                  <input type="number" placeholder="Amount (UGX, negative to deduct)" value={adjustForm.amount} onChange={(e) => setAdjustForm((x) => ({ ...x, amount: e.target.value }))} />
                  <input placeholder="Reason (required)" value={adjustForm.reason} onChange={(e) => setAdjustForm((x) => ({ ...x, reason: e.target.value }))} />
                  <button disabled={busy || !adjustForm.amount || !adjustForm.reason.trim()}
                    onClick={() => void run(() => invoke("workforce_add_adjustment", { token, workerId: selWorker, payRecordId: null, kind: adjustForm.kind, amountMinor: Number(adjustForm.amount), reason: adjustForm.reason }))}>
                    Add adjustment
                  </button>
                </div>
                <div className="form-row">
                  <input type="number" min="1" placeholder="Advance amount (UGX)" value={advanceForm.amount} onChange={(e) => setAdvanceForm((x) => ({ ...x, amount: e.target.value }))} />
                  <input placeholder="Advance reason (required)" value={advanceForm.reason} onChange={(e) => setAdvanceForm((x) => ({ ...x, reason: e.target.value }))} />
                  <button disabled={busy || !advanceForm.amount || !advanceForm.reason.trim()}
                    onClick={() => void run(() => invoke("workforce_issue_advance", { token, workerId: selWorker, amountMinor: Number(advanceForm.amount), reason: advanceForm.reason }))}>
                    Issue advance
                  </button>
                </div>
                <div className="form-row">
                  <input type="number" min="1" placeholder="Payment amount (UGX)" value={paymentForm.amount} onChange={(e) => setPaymentForm((x) => ({ ...x, amount: e.target.value }))} />
                  <select value={paymentForm.method} onChange={(e) => setPaymentForm((x) => ({ ...x, method: e.target.value }))}>{METHODS.map((m) => <option key={m}>{m}</option>)}</select>
                  <input placeholder="Reference (mobile money)" value={paymentForm.reference} onChange={(e) => setPaymentForm((x) => ({ ...x, reference: e.target.value }))} />
                  <button disabled={busy || !paymentForm.amount}
                    onClick={() => void run(() => invoke("workforce_record_payment", { token, workerId: selWorker, amountMinor: Number(paymentForm.amount), paymentMethod: paymentForm.method, reference: paymentForm.reference || null }))}>
                    Record payment
                  </button>
                </div>
              </div>

              <div className="table-card">
                <h2>Pay records</h2>
                <table>
                  <thead><tr><th>Period</th><th>Hours</th><th>Rate</th><th>Amount</th><th>Status</th><th></th></tr></thead>
                  <tbody>
                    {pay.length === 0 && <tr><td colSpan={6}>No pay records for this worker.</td></tr>}
                    {pay.map((p) => (
                      <tr key={p.id}>
                        <td>{new Date(p.period_start * 1000).toLocaleDateString()} – {new Date(p.period_end * 1000).toLocaleDateString()}</td>
                        <td>{p.hours_worked.toFixed(1)}</td><td>{money(p.rate_minor)}</td><td>{money(p.amount_minor)}</td>
                        <td><span className={`pill ${p.status === "PAID" ? "pill-ok" : p.status === "APPROVED" ? "pill-warn" : "pill-muted"}`}>{p.status}</span></td>
                        <td className="form-row">
                          {p.status === "DRAFT" && <button disabled={busy} onClick={() => void run(() => invoke("workforce_approve_pay", { token, payId: p.id }))}>Approve</button>}
                          {p.status === "APPROVED" && <button disabled={busy} onClick={() => void run(() => invoke("workforce_mark_paid", { token, payId: p.id }))}>Mark paid</button>}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </>
      )}
    </section>
  );
}
