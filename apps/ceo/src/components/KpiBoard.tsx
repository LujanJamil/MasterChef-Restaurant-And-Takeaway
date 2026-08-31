import { useCallback, useEffect, useState } from "react";

interface LowStockItem { item: string; on_hand: number; reorder_level: number; unit: string; }
interface Kpis {
  generated_at: string;
  sales_today_minor: number;
  orders_today: number;
  delayed_orders: number;
  expenses_today_minor: number;
  workers_present: number;
  workers_scheduled_today: number;
  low_stock_items: LowStockItem[];
}

const env = (import.meta as unknown as { env?: Record<string, string | undefined> }).env ?? {};
const KPI_URL = env.VITE_KPI_FUNCTION_URL ?? "";
const DEVICE_ID = env.VITE_SYNC_DEVICE_ID ?? "";
const DEVICE_TOKEN = env.VITE_SYNC_DEVICE_TOKEN ?? "";
const configured = Boolean(KPI_URL && DEVICE_ID && DEVICE_TOKEN);
const money = (n: number) => `UGX ${n.toLocaleString()}`;

export default function KpiBoard() {
  const [kpis, setKpis] = useState<Kpis | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    if (!configured) return;
    setLoading(true);
    setError("");
    try {
      const res = await fetch(KPI_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json", "x-device-id": DEVICE_ID, "x-device-token": DEVICE_TOKEN },
        body: JSON.stringify({ action: "daily_kpis" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error ?? `HTTP ${res.status}`);
      setKpis(data.kpis as Kpis);
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
    const t = setInterval(() => void load(), 60000);
    return () => clearInterval(t);
  }, [load]);

  if (!configured) {
    return (
      <section className="panel">
        <div className="panel-head">
          <div>
            <h2>Operational KPIs — sync not configured</h2>
            <p>These cards read live aggregated data from the cloud sync pipeline. Set <code>VITE_KPI_FUNCTION_URL</code>, <code>VITE_SYNC_DEVICE_ID</code> and <code>VITE_SYNC_DEVICE_TOKEN</code> for this device, and they populate automatically. No placeholder numbers are shown.</p>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section className="panel">
      <div className="panel-head">
        <div>
          <h2>Operational KPIs — today</h2>
          <p>{kpis ? `Cloud truth · generated ${new Date(kpis.generated_at).toLocaleTimeString()} · refreshes every 60s` : loading ? "Loading…" : "Waiting for first sync"}</p>
        </div>
        <button className="secondary" onClick={() => void load()} disabled={loading}>{loading ? "Loading…" : "Refresh"}</button>
      </div>
      {error && <div className="error app-error">{error}</div>}
      {kpis && (
        <>
          <section className="metrics">
            <article><span>Sales today</span><b>{money(kpis.sales_today_minor)}</b></article>
            <article><span>Orders today</span><b>{kpis.orders_today}</b></article>
            <article><span>Delayed orders</span><b className={kpis.delayed_orders > 0 ? "kpi-warn" : undefined}>{kpis.delayed_orders}</b></article>
            <article><span>Expenses today</span><b>{money(kpis.expenses_today_minor)}</b></article>
            <article><span>Workers present</span><b>{kpis.workers_present} / {kpis.workers_scheduled_today} scheduled</b></article>
            <article><span>Low stock items</span><b className={kpis.low_stock_items.length > 0 ? "kpi-warn" : undefined}>{kpis.low_stock_items.length}</b></article>
          </section>
          {kpis.low_stock_items.length > 0 && (
            <div className="table-wrap">
              <table>
                <thead><tr><th>Item</th><th>On hand</th><th>Reorder level</th><th>Unit</th></tr></thead>
                <tbody>
                  {kpis.low_stock_items.map((i) => (
                    <tr key={i.item}><td>{i.item}</td><td>{i.on_hand}</td><td>{i.reorder_level}</td><td>{i.unit}</td></tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </section>
  );
}
