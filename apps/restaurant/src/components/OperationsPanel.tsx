import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Section = "receive" | "issue" | "waste" | "adjust" | "expense" | "equipment";
interface QueuedEvent { id: string; kind: string; summary: string; at: number; }

const SECTION_LABEL: Record<Section, string> = {
  receive: "Receive stock",
  issue: "Issue to kitchen",
  waste: "Waste / damage",
  adjust: "Adjustment",
  expense: "Expense",
  equipment: "Equipment",
};
const LOCATIONS = ["STORE", "KITCHEN", "DISPLAY", "FRIDGE", "ICE_CREAM", "CHAPATI", "LOCAL_FOOD", "OTHER"];
const EXPENSE_METHODS = ["CASH", "MTN_MOMO", "AIRTEL_MONEY", "OTHER"];
const money = (n: number) => `UGX ${n.toLocaleString()}`;

export default function OperationsPanel({ token, role, userId, onError }: { token: string; role: string; userId: string; onError?: (message: string) => void }) {
  const [section, setSection] = useState<Section>("receive");
  const [pending, setPending] = useState(0);
  const [queued, setQueued] = useState<QueuedEvent[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");

  const [stock, setStock] = useState({ item: "", quantity: "1", unit: "KG", location: "STORE", supplierRef: "", unitCost: "", reason: "" });
  const [expense, setExpense] = useState({ category: "", amount: "", method: "CASH", description: "" });
  const [equipment, setEquipment] = useState({ assetTag: "", description: "", location: "STORE", condition: "WORKING", notes: "" });

  const canManageStock = ["CEO", "MANAGER", "STOREKEEPER"].includes(role);
  const canApprove = ["CEO", "MANAGER"].includes(role);

  const refreshPending = useCallback(async () => {
    try { setPending(await invoke<number>("sync_pending_count", { token })); } catch (e) { onError?.(String(e)); }
  }, [token, onError]);

  useEffect(() => { void refreshPending(); }, [refreshPending]);

  async function queueEvent(entityType: string, operation: string, kind: string, summary: string, payload: Record<string, unknown>) {
    setBusy(true);
    try {
      const eventId = crypto.randomUUID();
      await invoke("sync_enqueue", { token, entityType, entityId: eventId, operation, payload: { client_event_id: eventId, actor_id: userId, ...payload } });
      setQueued((x) => [{ id: eventId, kind, summary, at: Math.floor(Date.now() / 1000) }, ...x].slice(0, 30));
      setNotice(`${kind} queued — ${summary}. Uploads with the next sync.`);
      await refreshPending();
    } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }

  async function submitStock(operation: "RECEIVE" | "ISSUE" | "WASTE" | "ADJUST") {
    if (!canManageStock) { onError?.("Your role cannot record stock movements."); return; }
    if (!stock.item.trim()) { onError?.("Enter the stock item name or SKU."); return; }
    const quantity = Number(stock.quantity);
    if (!Number.isFinite(quantity) || quantity <= 0) { onError?.("Enter a valid quantity."); return; }
    if ((operation === "WASTE" || operation === "ADJUST") && !stock.reason.trim()) { onError?.("Waste and adjustments require a reason."); return; }
    const unitCost = stock.unitCost.trim() ? Number(stock.unitCost) : null;
    await queueEvent("stock_movement", operation, SECTION_LABEL[section], `${stock.item.trim()} × ${quantity} ${stock.unit}`, {
      item: stock.item.trim(),
      quantity,
      unit: stock.unit,
      location: stock.location,
      supplier_reference: operation === "RECEIVE" ? stock.supplierRef.trim() || null : null,
      unit_cost_minor: operation === "RECEIVE" && unitCost !== null && Number.isSafeInteger(unitCost) ? unitCost : null,
      currency: "UGX",
      reason: stock.reason.trim() || null,
    });
    setStock({ item: "", quantity: "1", unit: stock.unit, location: stock.location, supplierRef: "", unitCost: "", reason: "" });
  }

  async function submitExpense() {
    const amountMinor = Number(expense.amount);
    if (!expense.category.trim() || !expense.description.trim()) { onError?.("Expense needs a category and description."); return; }
    if (!Number.isSafeInteger(amountMinor) || amountMinor <= 0) { onError?.("Enter a valid expense amount in UGX."); return; }
    await queueEvent("expense", "CREATE", "Expense", `${expense.category.trim()} — ${money(amountMinor)}`, {
      category: expense.category.trim(),
      amount_minor: amountMinor,
      currency: "UGX",
      payment_method: expense.method,
      description: expense.description.trim(),
      status: "PENDING_APPROVAL",
    });
    setExpense({ category: "", amount: "", method: expense.method, description: "" });
  }

  async function submitEquipment() {
    if (!equipment.assetTag.trim() || !equipment.description.trim()) { onError?.("Equipment event needs an asset tag and description."); return; }
    await queueEvent("equipment_event", "CREATE", "Equipment", `${equipment.assetTag.trim()} — ${equipment.condition}`, {
      asset_tag: equipment.assetTag.trim(),
      description: equipment.description.trim(),
      location: equipment.location,
      condition: equipment.condition,
      notes: equipment.notes.trim() || null,
    });
    setEquipment({ assetTag: "", description: "", location: equipment.location, condition: "WORKING", notes: "" });
  }

  const stockForm = (op: "RECEIVE" | "ISSUE" | "WASTE" | "ADJUST", cta: string) => (
    <div className="card composer">
      <h2>{SECTION_LABEL[section]}</h2>
      <div className="form-row">
        <input placeholder="Item name or SKU" value={stock.item} onChange={(e) => setStock((x) => ({ ...x, item: e.target.value }))} />
        <input type="number" min="1" value={stock.quantity} onChange={(e) => setStock((x) => ({ ...x, quantity: e.target.value }))} title="Quantity" aria-label="Quantity" />
        <select value={stock.unit} onChange={(e) => setStock((x) => ({ ...x, unit: e.target.value }))}>
          {["KG", "G", "L", "ML", "PCS", "PACK", "TRAY", "BUNCH"].map((u) => <option key={u} value={u}>{u}</option>)}
        </select>
        <select value={stock.location} onChange={(e) => setStock((x) => ({ ...x, location: e.target.value }))}>
          {LOCATIONS.map((l) => <option key={l} value={l}>{l.replace("_", " ")}</option>)}
        </select>
      </div>
      <div className="form-row">
        {op === "RECEIVE" && <input placeholder="Supplier reference / invoice" value={stock.supplierRef} onChange={(e) => setStock((x) => ({ ...x, supplierRef: e.target.value }))} />}
        {op === "RECEIVE" && <input type="number" min="0" placeholder="Unit cost (UGX)" value={stock.unitCost} onChange={(e) => setStock((x) => ({ ...x, unitCost: e.target.value }))} />}
        {(op === "WASTE" || op === "ADJUST") && <input placeholder="Reason (required)" value={stock.reason} onChange={(e) => setStock((x) => ({ ...x, reason: e.target.value }))} />}
        <button disabled={busy || !canManageStock} onClick={() => void submitStock(op)}>{cta}</button>
      </div>
      {!canManageStock && <p>Stock movements require the Storekeeper, Manager or CEO role.</p>}
    </div>
  );

  return (
    <section className="content">
      <div className="section-head">
        <div>
          <div className="eyebrow">RELEASE B · STORE &amp; OPERATIONS</div>
          <h1>Inventory, expenses &amp; equipment</h1>
          <p>Every movement is written to the offline sync queue with a stable event ID, actor attribution and reason capture. Stock balances reconcile when the inventory ledger syncs.</p>
        </div>
        <span className="pill pill-muted">{pending} queued to sync</span>
      </div>

      <div className="toolbar">
        {(Object.keys(SECTION_LABEL) as Section[]).map((s) => (
          <button key={s} className={`secondary ${section === s ? "active" : ""}`} onClick={() => setSection(s)}>{SECTION_LABEL[s]}</button>
        ))}
      </div>

      {notice && <div className="card"><p>{notice}</p></div>}

      {section === "receive" && stockForm("RECEIVE", "Record receiving")}
      {section === "issue" && stockForm("ISSUE", "Issue to section")}
      {section === "waste" && stockForm("WASTE", "Record waste")}
      {section === "adjust" && stockForm("ADJUST", "Queue adjustment")}

      {section === "expense" && (
        <div className="card composer">
          <h2>Record expense</h2>
          <div className="form-row">
            <input placeholder="Category e.g. Utilities" value={expense.category} onChange={(e) => setExpense((x) => ({ ...x, category: e.target.value }))} />
            <input type="number" min="1" placeholder="Amount (UGX)" value={expense.amount} onChange={(e) => setExpense((x) => ({ ...x, amount: e.target.value }))} />
            <select value={expense.method} onChange={(e) => setExpense((x) => ({ ...x, method: e.target.value }))}>
              {EXPENSE_METHODS.map((m) => <option key={m} value={m}>{m.replace("_", " ")}</option>)}
            </select>
          </div>
          <div className="form-row">
            <input placeholder="Description (required)" value={expense.description} onChange={(e) => setExpense((x) => ({ ...x, description: e.target.value }))} />
            <button disabled={busy} onClick={() => void submitExpense()}>Submit for approval</button>
          </div>
          <p>Expenses enter as pending approval. {canApprove ? "Your role approves expenses from the synced ledger." : "A Manager or the CEO approves them after sync."}</p>
        </div>
      )}

      {section === "equipment" && (
        <div className="card composer">
          <h2>Equipment register event</h2>
          <div className="form-row">
            <input placeholder="Asset tag e.g. OVEN-01" value={equipment.assetTag} onChange={(e) => setEquipment((x) => ({ ...x, assetTag: e.target.value }))} />
            <input placeholder="Description e.g. Deep fryer thermostat fault" value={equipment.description} onChange={(e) => setEquipment((x) => ({ ...x, description: e.target.value }))} />
            <select value={equipment.location} onChange={(e) => setEquipment((x) => ({ ...x, location: e.target.value }))}>
              {LOCATIONS.map((l) => <option key={l} value={l}>{l.replace("_", " ")}</option>)}
            </select>
            <select value={equipment.condition} onChange={(e) => setEquipment((x) => ({ ...x, condition: e.target.value }))}>
              {["WORKING", "NEEDS_REPAIR", "DAMAGED", "LOST", "RETIRED"].map((c) => <option key={c} value={c}>{c.replace("_", " ")}</option>)}
            </select>
          </div>
          <div className="form-row">
            <input placeholder="Notes (optional)" value={equipment.notes} onChange={(e) => setEquipment((x) => ({ ...x, notes: e.target.value }))} />
            <button disabled={busy} onClick={() => void submitEquipment()}>Record equipment event</button>
          </div>
          <p>Equipment changes are compensating events — history is never silently edited.</p>
        </div>
      )}

      <div className="table-card">
        <h2>Queued on this device</h2>
        <table>
          <thead><tr><th>Time</th><th>Type</th><th>Summary</th><th>Event ID</th></tr></thead>
          <tbody>
            {queued.length === 0 && <tr><td colSpan={4}>Nothing queued this session yet. Submissions above appear here immediately and sync when online.</td></tr>}
            {queued.map((q) => (
              <tr key={q.id}>
                <td>{new Date(q.at * 1000).toLocaleTimeString()}</td>
                <td>{q.kind}</td>
                <td>{q.summary}</td>
                <td><small>{q.id.slice(0, 8)}…</small></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
