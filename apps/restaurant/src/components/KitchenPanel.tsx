import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface KitchenOrder {
  id: string;
  order_number: number;
  order_type: string;
  table_id: string | null;
  customer_name: string | null;
  status: string;
  total_minor: number;
  notes: string | null;
  created_at?: number;
  updated_at?: number;
}
interface KitchenItem {
  id: string;
  order_id: string;
  name_snapshot: string;
  quantity: number;
  station: string;
  status: string;
  notes: string | null;
}

const ACTIVE_STATUSES = ["SUBMITTED", "PREPARING"];
const DELAY_MINUTES = 20;
const money = (n: number) => `UGX ${n.toLocaleString()}`;
const fmtElapsed = (fromSec: number) => {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - fromSec);
  return `${Math.floor(s / 60)}m`;
};

export default function KitchenPanel({ token, role, onError }: { token: string; role: string; onError?: (message: string) => void }) {
  const [orders, setOrders] = useState<KitchenOrder[]>([]);
  const [itemsByOrder, setItemsByOrder] = useState<Record<string, KitchenItem[]>>({});
  const [station, setStation] = useState("ALL");
  const [busy, setBusy] = useState(false);
  const [pending, setPending] = useState(0);
  const [waste, setWaste] = useState({ item: "", quantity: "1", station: "KITCHEN", reason: "" });
  const [notice, setNotice] = useState("");
  const canTransition = ["CEO", "MANAGER", "WAITER", "COOK"].includes(role); // native security core enforces the final decision

  const load = useCallback(async () => {
    try {
      const all = await invoke<KitchenOrder[]>("order_list", { token, limit: 60 });
      const active = all.filter((o) => ACTIVE_STATUSES.includes(o.status)).slice(0, 15);
      const entries = await Promise.all(
        active.map(async (o) => [o.id, await invoke<KitchenItem[]>("order_items", { token, orderId: o.id })] as const)
      );
      const map: Record<string, KitchenItem[]> = {};
      for (const [id, items] of entries) map[id] = items.filter((i) => i.status !== "CANCELLED");
      setOrders(active);
      setItemsByOrder(map);
      setPending(await invoke<number>("sync_pending_count", { token }));
    } catch (e) {
      onError?.(String(e));
    }
  }, [token, onError]);

  useEffect(() => {
    void load();
    const t = setInterval(() => void load(), 30000);
    return () => clearInterval(t);
  }, [load]);

  const stations = useMemo(() => {
    const s = new Set<string>();
    Object.values(itemsByOrder).flat().forEach((i) => s.add(i.station));
    return ["ALL", ...Array.from(s).sort()];
  }, [itemsByOrder]);

  const visible = useMemo(
    () => orders.filter((o) => station === "ALL" || (itemsByOrder[o.id] ?? []).some((i) => i.station === station)),
    [orders, itemsByOrder, station]
  );

  async function transition(o: KitchenOrder, to: "Preparing" | "Ready") {
    setBusy(true);
    try {
      await invoke("order_transition", { token, orderId: o.id, status: to, reason: null });
      await load();
    } catch (e) {
      onError?.(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function recordWaste() {
    if (!waste.item.trim() || !waste.reason.trim()) {
      onError?.("Waste capture needs an item name and a reason.");
      return;
    }
    setBusy(true);
    try {
      const eventId = crypto.randomUUID();
      await invoke("sync_enqueue", {
        token,
        entityType: "waste_event",
        entityId: eventId,
        operation: "CREATE",
        payload: {
          client_event_id: eventId,
          item: waste.item.trim(),
          quantity: Math.max(1, Number(waste.quantity) || 1),
          station: waste.station,
          reason: waste.reason.trim(),
        },
      });
      setNotice(`Waste event queued for ${waste.item.trim()} — syncs when online.`);
      setWaste({ item: "", quantity: "1", station: waste.station, reason: "" });
      setPending(await invoke<number>("sync_pending_count", { token }));
    } catch (e) {
      onError?.(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="content">
      <div className="section-head">
        <div>
          <div className="eyebrow">RELEASE A · KITCHEN</div>
          <h1>Kitchen display</h1>
          <p>Live production queue from submitted orders. Accept, prepare and mark ready by station. Auto-refreshes every 30 seconds.</p>
        </div>
        <button className="secondary" onClick={() => void load()}>Refresh</button>
      </div>

      <div className="toolbar">
        {stations.map((s) => (
          <button key={s} className={`secondary ${station === s ? "active" : ""}`} onClick={() => setStation(s)}>
            {s === "ALL" ? "All stations" : s.replace("_", " ")}
          </button>
        ))}
        <span className="pill pill-muted">{pending} queued to sync</span>
      </div>

      {notice && <div className="card"><p>{notice}</p></div>}

      <div className="menu-grid">
        {visible.length === 0 && (
          <article className="card">
            <h2>Queue clear</h2>
            <p>No orders waiting for preparation{station !== "ALL" ? ` at ${station.replace("_", " ")}` : ""}.</p>
          </article>
        )}
        {visible.map((o) => {
          const ref = o.created_at ?? o.updated_at;
          const elapsedSec = ref ? Math.floor(Date.now() / 1000) - ref : null;
          const delayed = elapsedSec !== null && elapsedSec > DELAY_MINUTES * 60;
          const items = (itemsByOrder[o.id] ?? []).filter((i) => station === "ALL" || i.station === station);
          return (
            <article className="card product" key={o.id}>
              <div>
                <span className="eyebrow">
                  #{o.order_number} · {o.order_type}
                  {o.customer_name ? ` · ${o.customer_name}` : ""}
                </span>
                <h2>
                  {o.status === "SUBMITTED" ? "Waiting to start" : "In preparation"}{" "}
                  {delayed && <span className="pill pill-warn">DELAYED{elapsedSec !== null ? ` · ${fmtElapsed(ref!)}` : ""}</span>}
                </h2>
                {items.map((i) => (
                  <div className="line" key={i.id}>
                    <span>
                      {i.quantity} × {i.name_snapshot} <small>{i.station.replace("_", " ")}</small>
                      {i.notes ? ` — ${i.notes}` : ""}
                    </span>
                  </div>
                ))}
                {o.notes && <p>Order note: {o.notes}</p>}
                <strong>{money(o.total_minor)}</strong>
              </div>
              <div className="form-row actions">
                {o.status === "SUBMITTED" && (
                  <button disabled={busy || !canTransition} onClick={() => void transition(o, "Preparing")}>Accept &amp; start preparing</button>
                )}
                {o.status === "PREPARING" && (
                  <button disabled={busy || !canTransition} onClick={() => void transition(o, "Ready")}>Mark ready</button>
                )}
              </div>
            </article>
          );
        })}
      </div>

      <div className="card admin">
        <h2>Waste &amp; spoilage capture</h2>
        <p>Kitchen waste is recorded with reason and actor attribution, queued locally, and synchronizes by stable event ID.</p>
        <div className="form-row">
          <input placeholder="Item e.g. Chapati" value={waste.item} onChange={(e) => setWaste((x) => ({ ...x, item: e.target.value }))} />
          <input type="number" min={1} value={waste.quantity} onChange={(e) => setWaste((x) => ({ ...x, quantity: e.target.value }))} title="Quantity" aria-label="Quantity" />
          <select value={waste.station} onChange={(e) => setWaste((x) => ({ ...x, station: e.target.value }))}>
            {["KITCHEN", "CHAPATI", "LOCAL_FOOD", "DISPLAY", "FRIDGE", "ICE_CREAM", "OTHER"].map((s) => (
              <option key={s} value={s}>{s.replace("_", " ")}</option>
            ))}
          </select>
          <input placeholder="Reason (required)" value={waste.reason} onChange={(e) => setWaste((x) => ({ ...x, reason: e.target.value }))} />
          <button disabled={busy} onClick={() => void recordWaste()}>Record waste</button>
        </div>
      </div>
    </section>
  );
}
