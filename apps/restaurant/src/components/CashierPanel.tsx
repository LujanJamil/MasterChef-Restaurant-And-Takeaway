import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PayableOrder {
  id: string;
  order_number: number;
  order_type: string;
  status: string;
  total_minor: number;
  customer_name: string | null;
  customer_phone: string | null;
}
type Method = "CASH" | "MTN_MOMO" | "AIRTEL_MONEY";
interface CashPayment { orderNumber: number; amount: number; }
interface CashSession { openedAt: number; floatMinor: number; payments: CashPayment[]; }

const AWAITING = ["READY", "SERVED"];
const METHOD_LABEL: Record<Method, string> = { CASH: "Cash", MTN_MOMO: "MTN Mobile Money", AIRTEL_MONEY: "Airtel Money" };
const money = (n: number) => `UGX ${n.toLocaleString()}`;

export default function CashierPanel({ token, role, onError }: { token: string; role: string; onError?: (message: string) => void }) {
  const [orders, setOrders] = useState<PayableOrder[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [method, setMethod] = useState<Method>("CASH");
  const [amount, setAmount] = useState("");
  const [reference, setReference] = useState("");
  const [pending, setPending] = useState(0);
  const [session, setSession] = useState<CashSession | null>(null);
  const [floatInput, setFloatInput] = useState("");
  const [countedInput, setCountedInput] = useState("");
  const [refund, setRefund] = useState({ orderNumber: "", amount: "", reason: "" });
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const canRefund = ["CEO", "MANAGER", "CASHIER"].includes(role);

  const load = useCallback(async () => {
    try {
      const all = await invoke<PayableOrder[]>("order_list", { token, limit: 80 });
      setOrders(all.filter((o) => AWAITING.includes(o.status)));
      setPending(await invoke<number>("sync_pending_count", { token }));
    } catch (e) {
      onError?.(String(e));
    }
  }, [token, onError]);

  useEffect(() => { void load(); }, [load]);

  const selected = useMemo(() => orders.find((o) => o.id === selectedId) ?? null, [orders, selectedId]);
  const expectedCash = session ? session.floatMinor + session.payments.reduce((a, p) => a + p.amount, 0) : 0;

  async function enqueue(entityType: string, operation: string, payload: Record<string, unknown>, entityId: string) {
    await invoke("sync_enqueue", { token, entityType, entityId, operation, payload });
    setPending(await invoke<number>("sync_pending_count", { token }));
  }

  async function recordPayment() {
    if (!selected) { onError?.("Select an order awaiting payment."); return; }
    const amountMinor = amount.trim() ? Number(amount) : selected.total_minor;
    if (!Number.isSafeInteger(amountMinor) || amountMinor <= 0) { onError?.("Enter a valid amount in UGX."); return; }
    if (method !== "CASH" && !reference.trim()) { onError?.("Mobile money payments need the transaction reference."); return; }
    setBusy(true);
    try {
      const eventId = crypto.randomUUID();
      await enqueue("payment", "CREATE", {
        client_event_id: eventId,
        order_id: selected.id,
        order_number: selected.order_number,
        amount_minor: amountMinor,
        currency: "UGX",
        method,
        reference: reference.trim() || null,
      }, eventId);
      if (method === "CASH" && session) {
        setSession({ ...session, payments: [...session.payments, { orderNumber: selected.order_number, amount: amountMinor }] });
      }
      setNotice(`Payment of ${money(amountMinor)} via ${METHOD_LABEL[method]} queued for order #${selected.order_number}. Syncs when online.`);
      setOrders((x) => x.filter((o) => o.id !== selected.id));
      setSelectedId(""); setAmount(""); setReference("");
    } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }

  async function openSession() {
    const floatMinor = Number(floatInput);
    if (!Number.isSafeInteger(floatMinor) || floatMinor < 0) { onError?.("Enter the opening float in UGX."); return; }
    setBusy(true);
    try {
      const eventId = crypto.randomUUID();
      await enqueue("cash_session", "OPEN", { client_event_id: eventId, float_minor: floatMinor, currency: "UGX" }, eventId);
      setSession({ openedAt: Math.floor(Date.now() / 1000), floatMinor, payments: [] });
      setFloatInput("");
      setNotice(`Cash session opened with float ${money(floatMinor)}.`);
    } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }

  async function closeSession() {
    if (!session) return;
    const counted = Number(countedInput);
    if (!Number.isSafeInteger(counted) || counted < 0) { onError?.("Enter the counted drawer total in UGX."); return; }
    setBusy(true);
    try {
      const variance = counted - expectedCash;
      const eventId = crypto.randomUUID();
      await enqueue("cash_session", "CLOSE", {
        client_event_id: eventId,
        float_minor: session.floatMinor,
        expected_minor: expectedCash,
        counted_minor: counted,
        variance_minor: variance,
        cash_payments_count: session.payments.length,
        currency: "UGX",
      }, eventId);
      setNotice(`Cash session closed. Expected ${money(expectedCash)}, counted ${money(counted)}, variance ${money(variance)}. Reconciliation event queued.`);
      setSession(null); setCountedInput("");
    } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }

  async function recordRefund() {
    if (!refund.orderNumber.trim() || !refund.amount.trim() || !refund.reason.trim()) {
      onError?.("Refund needs order number, amount and reason."); return;
    }
    const amountMinor = Number(refund.amount);
    if (!Number.isSafeInteger(amountMinor) || amountMinor <= 0) { onError?.("Enter a valid refund amount."); return; }
    setBusy(true);
    try {
      const eventId = crypto.randomUUID();
      await enqueue("refund", "CREATE", {
        client_event_id: eventId,
        order_number: Number(refund.orderNumber) || refund.orderNumber.trim(),
        amount_minor: amountMinor,
        currency: "UGX",
        reason: refund.reason.trim(),
      }, eventId);
      setNotice(`Refund of ${money(amountMinor)} queued for order #${refund.orderNumber}.`);
      setRefund({ orderNumber: "", amount: "", reason: "" });
    } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }

  return (
    <section className="content">
      <div className="section-head">
        <div>
          <div className="eyebrow">RELEASE A · PAYMENTS &amp; CASH</div>
          <h1>Cashier</h1>
          <p>Record cash, MTN Mobile Money and Airtel Money against ready orders. Payments persist to the offline queue and synchronize by stable event ID.</p>
        </div>
        <button className="secondary" onClick={() => void load()}>Refresh</button>
      </div>

      {notice && <div className="card"><p>{notice}</p></div>}

      <section className="grid">
        <article className="card">
          <span className="eyebrow">CASH SESSION</span>
          {session ? (
            <>
              <h2>Open since {new Date(session.openedAt * 1000).toLocaleTimeString()}</h2>
              <p>Float {money(session.floatMinor)} · cash payments {session.payments.length} · expected in drawer <b>{money(expectedCash)}</b></p>
              <div className="form-row">
                <input type="number" min="0" placeholder="Counted drawer total (UGX)" value={countedInput} onChange={(e) => setCountedInput(e.target.value)} />
                <button disabled={busy} onClick={() => void closeSession()}>Close &amp; reconcile</button>
              </div>
            </>
          ) : (
            <>
              <h2>No open session</h2>
              <div className="form-row">
                <input type="number" min="0" placeholder="Opening float (UGX)" value={floatInput} onChange={(e) => setFloatInput(e.target.value)} />
                <button disabled={busy} onClick={() => void openSession()}>Open cash session</button>
              </div>
            </>
          )}
        </article>

        <article className="card">
          <span className="eyebrow">AWAITING PAYMENT</span>
          <h2>{orders.length}</h2>
          <p>Orders ready or served and not yet paid.</p>
        </article>

        <article className="card">
          <span className="eyebrow">SYNC QUEUE</span>
          <h2>{pending}</h2>
          <p>Operations queued on this device, uploading when connectivity returns.</p>
        </article>
      </section>

      <div className="orders-layout">
        <div>
          <div className="card composer">
            <h2>Record payment</h2>
            <div className="form-row">
              <select value={method} onChange={(e) => setMethod(e.target.value as Method)}>
                {(Object.keys(METHOD_LABEL) as Method[]).map((m) => <option key={m} value={m}>{METHOD_LABEL[m]}</option>)}
              </select>
              <input type="number" min="1" placeholder={selected ? `Amount (default ${selected.total_minor})` : "Amount (UGX)"} value={amount} onChange={(e) => setAmount(e.target.value)} />
              {method !== "CASH" && <input placeholder="Transaction reference" value={reference} onChange={(e) => setReference(e.target.value)} />}
              <button disabled={busy || !selected} onClick={() => void recordPayment()}>Record payment</button>
            </div>
            {selected && <p>Order #{selected.order_number} · {selected.order_type} · total {money(selected.total_minor)}{selected.customer_name ? ` · ${selected.customer_name}` : ""}</p>}
          </div>
          {canRefund && (
            <div className="card admin">
              <h2>Refund / reversal</h2>
              <div className="form-row">
                <input placeholder="Order number" value={refund.orderNumber} onChange={(e) => setRefund((x) => ({ ...x, orderNumber: e.target.value }))} />
                <input type="number" min="1" placeholder="Amount (UGX)" value={refund.amount} onChange={(e) => setRefund((x) => ({ ...x, amount: e.target.value }))} />
                <input placeholder="Reason (required)" value={refund.reason} onChange={(e) => setRefund((x) => ({ ...x, reason: e.target.value }))} />
                <button className="danger" disabled={busy} onClick={() => void recordRefund()}>Record refund</button>
              </div>
            </div>
          )}
        </div>

        <aside className="card order-panel">
          <h2>Orders awaiting payment</h2>
          {orders.length === 0 && <p>No orders are waiting for payment.</p>}
          {orders.map((o) => (
            <button className={`order-row ${selectedId === o.id ? "selected" : ""}`} key={o.id} onClick={() => setSelectedId(o.id)}>
              <b>#{o.order_number}</b>
              <span>{o.order_type}</span>
              <span>{money(o.total_minor)}</span>
              <em>{o.status}</em>
            </button>
          ))}
        </aside>
      </div>
    </section>
  );
}
