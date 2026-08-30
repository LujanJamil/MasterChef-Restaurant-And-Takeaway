import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Order { id: string; order_number: number; order_type: string; status: string; total_minor: number; customer_name: string | null; customer_phone: string | null; }
interface Worker { id: string; display_name: string; role: string; status: string; }
interface Delivery { id: string; order_id: string; rider_id: string; status: string; collected_minor: number | null; handover_minor: number | null; currency: string; notes: string | null; assigned_at: number; }
interface Reconciliation { open_deliveries: number; expected_handover_minor: number; settled_minor: number; }

const money = (n: number) => `UGX ${n.toLocaleString()}`;
const fmt = (t: number) => new Date(t * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });

export default function DeliveryPanel({ token, role, userId, onError }: { token: string; role: string; userId: string; onError?: (message: string) => void }) {
  const canManage = ["CEO", "MANAGER"].includes(role);
  const isRider = role === "DELIVERY_RIDER";
  const [deliveries, setDeliveries] = useState<Delivery[]>([]);
  const [orders, setOrders] = useState<Order[]>([]);
  const [riders, setRiders] = useState<Worker[]>([]);
  const [workers, setWorkers] = useState<Worker[]>([]);
  const [recon, setRecon] = useState<Reconciliation | null>(null);
  const [busy, setBusy] = useState(false);
  const [assignForm, setAssignForm] = useState({ orderId: "", riderId: "", notes: "" });
  const [amounts, setAmounts] = useState<Record<string, string>>({});

  const load = useCallback(async () => {
    try {
      const [d, r] = await Promise.all([
        invoke<Delivery[]>("delivery_list", { token, riderId: null, status: null, limit: 100 }),
        invoke<Reconciliation>("delivery_reconciliation", { token, riderId: null }),
      ]);
      setDeliveries(d); setRecon(r);
      if (canManage) {
        const [o, w] = await Promise.all([
          invoke<Order[]>("order_list", { token, limit: 80 }),
          invoke<Worker[]>("users_list", { token }),
        ]);
        setOrders(o.filter((x) => x.order_type === "Delivery" || x.order_type === "DELIVERY").filter((x) => ["READY", "SERVED"].includes(x.status)));
        setWorkers(w.filter((x) => x.status === "ACTIVE"));
        setRiders(w.filter((x) => x.role === "DELIVERY_RIDER" && x.status === "ACTIVE"));
      }
    } catch (e) { onError?.(String(e)); }
  }, [token, canManage, onError]);

  useEffect(() => { void load(); }, [load]);

  const run = useCallback(async (fn: () => Promise<unknown>) => {
    setBusy(true);
    try { await fn(); await load(); } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }, [load, onError]);

  const orderFor = (id: string) => orders.find((o) => o.id === id);
  const workerName = (id: string) => workers.find((w) => w.id === id)?.display_name ?? (id === userId ? "You" : id.slice(0, 8));

  return (
    <section className="content">
      <div className="section-head">
        <div>
          <div className="eyebrow">PHASE 9 · DELIVERY</div>
          <h1>Delivery operations</h1>
          <p>Assignment, rider lifecycle and cash handover reconciliation. Rider identity is enforced by the native core.</p>
        </div>
        <button className="secondary" onClick={() => void load()}>Refresh</button>
      </div>

      {recon && (
        <section className="grid">
          <article className="card"><span className="eyebrow">OPEN DELIVERIES</span><h2>{recon.open_deliveries}</h2></article>
          <article className="card"><span className="eyebrow">EXPECTED HANDOVER</span><h2>{money(recon.expected_handover_minor)}</h2></article>
          <article className="card"><span className="eyebrow">SETTLED</span><h2>{money(recon.settled_minor)}</h2></article>
        </section>
      )}

      {canManage && (
        <div className="card admin">
          <h2>Assign delivery</h2>
          <div className="form-row">
            <select value={assignForm.orderId} onChange={(e) => setAssignForm((x) => ({ ...x, orderId: e.target.value }))}>
              <option value="">Select ready delivery order…</option>
              {orders.map((o) => <option key={o.id} value={o.id}>#{o.order_number} · {o.customer_name ?? "customer"} · {money(o.total_minor)}</option>)}
            </select>
            <select value={assignForm.riderId} onChange={(e) => setAssignForm((x) => ({ ...x, riderId: e.target.value }))}>
              <option value="">Select rider…</option>
              {riders.map((r) => <option key={r.id} value={r.id}>{r.display_name}</option>)}
            </select>
            <input placeholder="Notes (optional)" value={assignForm.notes} onChange={(e) => setAssignForm((x) => ({ ...x, notes: e.target.value }))} />
            <button disabled={busy || !assignForm.orderId || !assignForm.riderId}
              onClick={() => void run(() => invoke("delivery_assign", { token, orderId: assignForm.orderId, riderId: assignForm.riderId, notes: assignForm.notes.trim() || null }))}>
              Assign
            </button>
          </div>
        </div>
      )}

      <div className="table-card">
        <h2>{canManage ? "All deliveries" : "My deliveries"}</h2>
        <table>
          <thead><tr><th>Assigned</th><th>Order</th><th>Rider</th><th>Status</th><th>Collected</th><th>Handover</th><th></th></tr></thead>
          <tbody>
            {deliveries.length === 0 && <tr><td colSpan={7}>No deliveries yet.</td></tr>}
            {deliveries.map((d) => {
              const o = orderFor(d.order_id);
              return (
                <tr key={d.id}>
                  <td>{fmt(d.assigned_at)}</td>
                  <td>{o ? `#${o.order_number}` : d.order_id.slice(0, 8)}</td>
                  <td>{workerName(d.rider_id)}</td>
                  <td><span className={`pill ${d.status === "SETTLED" ? "pill-ok" : d.status === "CANCELLED" ? "pill-warn" : "pill-muted"}`}>{d.status}</span></td>
                  <td>{d.collected_minor !== null ? money(d.collected_minor) : "—"}</td>
                  <td>{d.handover_minor !== null ? money(d.handover_minor) : "—"}</td>
                  <td className="form-row">
                    {d.status === "ASSIGNED" && d.rider_id === userId && (
                      <button disabled={busy} onClick={() => void run(() => invoke("delivery_pickup", { token, deliveryId: d.id }))}>Pick up</button>
                    )}
                    {d.status === "PICKED_UP" && d.rider_id === userId && (
                      <>
                        <input type="number" min="0" placeholder="Collected UGX" value={amounts[d.id] ?? ""} onChange={(e) => setAmounts((x) => ({ ...x, [d.id]: e.target.value }))} />
                        <button disabled={busy || !(amounts[d.id] ?? "")}
                          onClick={() => void run(() => invoke("delivery_deliver", { token, deliveryId: d.id, collectedMinor: Number(amounts[d.id]) }))}>
                          Deliver
                        </button>
                      </>
                    )}
                    {d.status === "DELIVERED" && canManage && (
                      <>
                        <input type="number" min="0" placeholder="Handover UGX" value={amounts[d.id] ?? ""} onChange={(e) => setAmounts((x) => ({ ...x, [d.id]: e.target.value }))} />
                        <button disabled={busy || !(amounts[d.id] ?? "")}
                          onClick={() => void run(() => invoke("delivery_reconcile", { token, deliveryId: d.id, handoverMinor: Number(amounts[d.id]) }))}>
                          Settle
                        </button>
                      </>
                    )}
                    {canManage && !["SETTLED", "CANCELLED"].includes(d.status) && (
                      <button className="danger" disabled={busy} onClick={() => void run(() => invoke("delivery_cancel", { token, deliveryId: d.id }))}>Cancel</button>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {!canManage && !isRider && <div className="card"><p>Deliveries are assigned by a Manager or the CEO. Delivery riders see their assignments here.</p></div>}
    </section>
  );
}
