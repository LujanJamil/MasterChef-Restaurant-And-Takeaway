import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Item { id: string; sku: string; name: string; unit: string; category: string; reorder_level: number; active: boolean; }
interface Level { item_id: string; sku: string; name: string; unit: string; reorder_level: number; on_hand: number; }
interface Supplier { id: string; name: string; contact: string | null; phone: string | null; status: string; }
interface Count { id: string; item_id: string; location: string; expected: number; counted: number; variance: number; counted_by: string; created_at: number; }
interface Expense { id: string; category: string; amount_minor: number; payment_method: string; status: string; description: string; reference: string | null; created_by: string; created_at: number; }

const SECTIONS = ["stock", "movements", "counts", "suppliers", "expenses"] as const;
type Section = (typeof SECTIONS)[number];
const LOCS = ["STORE", "KITCHEN", "DISPLAY", "FRIDGE", "ICE_CREAM", "CHAPATI", "LOCAL_FOOD", "OTHER"];
const money = (n: number) => `UGX ${n.toLocaleString()}`;

export default function InventoryPanel({ token, role, onError }: { token: string; role: string; onError?: (message: string) => void }) {
  const canApprove = ["CEO", "MANAGER"].includes(role);
  const [section, setSection] = useState<Section>("stock");
  const [items, setItems] = useState<Item[]>([]);
  const [levels, setLevels] = useState<Level[]>([]);
  const [counts, setCounts] = useState<Count[]>([]);
  const [suppliers, setSuppliers] = useState<Supplier[]>([]);
  const [expenses, setExpenses] = useState<Expense[]>([]);
  const [busy, setBusy] = useState(false);

  const [itemForm, setItemForm] = useState({ sku: "", name: "", unit: "KG", category: "INGREDIENT", reorder: "5" });
  const [moveForm, setMoveForm] = useState({ itemId: "", type: "RECEIVE", quantity: "1", unit: "KG", from: "STORE", to: "KITCHEN", unitCost: "", supplierId: "", reason: "" });
  const [countForm, setCountForm] = useState({ itemId: "", location: "STORE", counted: "" });
  const [supForm, setSupForm] = useState({ name: "", contact: "", phone: "" });
  const [expForm, setExpForm] = useState({ category: "", amount: "", method: "CASH", description: "", reference: "" });

  const load = useCallback(async () => {
    try {
      const [i, l, c, s, e] = await Promise.all([
        invoke<Item[]>("inv_items", { token }),
        invoke<Level[]>("inv_levels", { token, location: null }),
        invoke<Count[]>("inv_counts", { token, limit: 50 }),
        invoke<Supplier[]>("inv_suppliers", { token }),
        invoke<Expense[]>("inv_expenses", { token, status: null, limit: 100 }),
      ]);
      setItems(i); setLevels(l); setCounts(c); setSuppliers(s); setExpenses(e);
    } catch (e) { onError?.(String(e)); }
  }, [token, onError]);

  useEffect(() => { void load(); }, [load]);

  const run = useCallback(async (fn: () => Promise<unknown>) => {
    setBusy(true);
    try { await fn(); await load(); } catch (e) { onError?.(String(e)); } finally { setBusy(false); }
  }, [load, onError]);

  const lowStock = levels.filter((l) => l.on_hand <= l.reorder_level);
  const itemName = (id: string) => items.find((i) => i.id === id)?.name ?? id.slice(0, 8);

  return (
    <section className="content">
      <div className="section-head">
        <div>
          <div className="eyebrow">PHASE 7 · INVENTORY, STORE &amp; SUPPLIERS</div>
          <h1>Inventory ledger</h1>
          <p>Persisted locally, ledger-based, synchronizing by stable event ID. Counts create compensating adjustments — history is never rewritten.</p>
        </div>
        <button className="secondary" onClick={() => void load()}>Refresh</button>
      </div>

      <div className="toolbar">
        {SECTIONS.map((s) => <button key={s} className={`secondary ${section === s ? "active" : ""}`} onClick={() => setSection(s)}>{s[0].toUpperCase() + s.slice(1)}</button>)}
        {lowStock.length > 0 && <span className="pill pill-warn">{lowStock.length} LOW STOCK</span>}
      </div>

      {section === "stock" && (
        <>
          <div className="card admin">
            <h2>Register / update item</h2>
            <div className="form-row">
              <input placeholder="SKU e.g. FLR-001" value={itemForm.sku} onChange={(e) => setItemForm((x) => ({ ...x, sku: e.target.value }))} />
              <input placeholder="Name" value={itemForm.name} onChange={(e) => setItemForm((x) => ({ ...x, name: e.target.value }))} />
              <select value={itemForm.unit} onChange={(e) => setItemForm((x) => ({ ...x, unit: e.target.value }))}>{["KG", "G", "L", "ML", "PCS", "PACK", "BOTTLE", "BOX"].map((u) => <option key={u}>{u}</option>)}</select>
              <select value={itemForm.category} onChange={(e) => setItemForm((x) => ({ ...x, category: e.target.value }))}>{["INGREDIENT", "PACKAGING", "BEVERAGE", "SNACK", "EQUIPMENT_CONSUMABLE", "OTHER"].map((c) => <option key={c}>{c}</option>)}</select>
              <input type="number" min="0" placeholder="Reorder level" value={itemForm.reorder} onChange={(e) => setItemForm((x) => ({ ...x, reorder: e.target.value }))} />
              <button disabled={busy || !itemForm.sku.trim() || !itemForm.name.trim()}
                onClick={() => void run(() => invoke("inv_upsert_item", { token, sku: itemForm.sku.trim(), name: itemForm.name.trim(), unit: itemForm.unit, category: itemForm.category, reorderLevel: Number(itemForm.reorder) || 0 }))}>
                Save item
              </button>
            </div>
          </div>
          <div className="table-card">
            <h2>Stock levels · all locations</h2>
            <table>
              <thead><tr><th>Item</th><th>SKU</th><th>On hand</th><th>Reorder at</th><th>Unit</th><th>State</th></tr></thead>
              <tbody>
                {levels.length === 0 && <tr><td colSpan={6}>No items registered yet.</td></tr>}
                {levels.map((l) => (
                  <tr key={l.item_id}>
                    <td><b>{l.name}</b></td><td>{l.sku}</td><td>{l.on_hand}</td><td>{l.reorder_level}</td><td>{l.unit}</td>
                    <td><span className={`pill ${l.on_hand <= l.reorder_level ? "pill-warn" : "pill-ok"}`}>{l.on_hand <= l.reorder_level ? "REORDER" : "HEALTHY"}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {section === "movements" && (
        <div className="card composer">
          <h2>Record stock movement</h2>
          <div className="form-row">
            <select value={moveForm.itemId} onChange={(e) => setMoveForm((x) => ({ ...x, itemId: e.target.value }))}>
              <option value="">Select item…</option>
              {items.map((i) => <option key={i.id} value={i.id}>{i.name} · {i.sku}</option>)}
            </select>
            <select value={moveForm.type} onChange={(e) => setMoveForm((x) => ({ ...x, type: e.target.value }))}>{["RECEIVE", "ISSUE", "WASTAGE", "ADJUSTMENT"].map((t) => <option key={t}>{t}</option>)}</select>
            <input type="number" min="0" step="any" value={moveForm.quantity} onChange={(e) => setMoveForm((x) => ({ ...x, quantity: e.target.value }))} title="Quantity" aria-label="Quantity" />
            <select value={moveForm.unit} onChange={(e) => setMoveForm((x) => ({ ...x, unit: e.target.value }))}>{["KG", "G", "L", "ML", "PCS", "PACK", "BOTTLE", "BOX"].map((u) => <option key={u}>{u}</option>)}</select>
          </div>
          <div className="form-row">
            {moveForm.type !== "RECEIVE" && (
              <select value={moveForm.from} onChange={(e) => setMoveForm((x) => ({ ...x, from: e.target.value }))} title="From location" aria-label="From location">{LOCS.map((l) => <option key={l}>{l}</option>)}</select>
            )}
            {(moveForm.type === "RECEIVE" || moveForm.type === "ISSUE") && (
              <select value={moveForm.to} onChange={(e) => setMoveForm((x) => ({ ...x, to: e.target.value }))} title="To location" aria-label="To location">{LOCS.map((l) => <option key={l}>{l}</option>)}</select>
            )}
            {moveForm.type === "RECEIVE" && (
              <>
                <select value={moveForm.supplierId} onChange={(e) => setMoveForm((x) => ({ ...x, supplierId: e.target.value }))}>
                  <option value="">Supplier (optional)…</option>
                  {suppliers.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
                </select>
                <input type="number" min="0" placeholder="Unit cost (UGX)" value={moveForm.unitCost} onChange={(e) => setMoveForm((x) => ({ ...x, unitCost: e.target.value }))} />
              </>
            )}
            {(moveForm.type === "WASTAGE" || moveForm.type === "ADJUSTMENT") && (
              <input placeholder="Reason (required)" value={moveForm.reason} onChange={(e) => setMoveForm((x) => ({ ...x, reason: e.target.value }))} />
            )}
            <button disabled={busy || !moveForm.itemId || Number(moveForm.quantity) <= 0}
              onClick={() => void run(() => invoke("inv_record_movement", {
                token, itemId: moveForm.itemId, movementType: moveForm.type, quantity: Number(moveForm.quantity), unit: moveForm.unit,
                fromLocation: moveForm.type === "RECEIVE" ? null : moveForm.from,
                toLocation: moveForm.type === "RECEIVE" || moveForm.type === "ISSUE" ? moveForm.to : null,
                unitCostMinor: moveForm.unitCost ? Number(moveForm.unitCost) : null,
                supplierId: moveForm.supplierId || null,
                reason: moveForm.reason.trim() || null,
              }))}>
              Record
            </button>
          </div>
          <p>The ledger enforces balances — issuing or wasting more than available at a location is rejected.</p>
        </div>
      )}

      {section === "counts" && (
        <>
          <div className="card composer">
            <h2>Stock count</h2>
            <div className="form-row">
              <select value={countForm.itemId} onChange={(e) => setCountForm((x) => ({ ...x, itemId: e.target.value }))}>
                <option value="">Select item…</option>
                {items.map((i) => <option key={i.id} value={i.id}>{i.name} · {i.sku}</option>)}
              </select>
              <select value={countForm.location} onChange={(e) => setCountForm((x) => ({ ...x, location: e.target.value }))}>{LOCS.map((l) => <option key={l}>{l}</option>)}</select>
              <input type="number" min="0" step="any" placeholder="Counted quantity" value={countForm.counted} onChange={(e) => setCountForm((x) => ({ ...x, counted: e.target.value }))} />
              <button disabled={busy || !countForm.itemId || countForm.counted === ""}
                onClick={() => void run(() => invoke("inv_count_stock", { token, itemId: countForm.itemId, location: countForm.location, counted: Number(countForm.counted) }))}>
                Finalize count
              </button>
            </div>
            <p>A variance automatically posts a compensating ledger adjustment attributed to you.</p>
          </div>
          <div className="table-card">
            <h2>Count history</h2>
            <table>
              <thead><tr><th>Time</th><th>Item</th><th>Location</th><th>Expected</th><th>Counted</th><th>Variance</th></tr></thead>
              <tbody>
                {counts.length === 0 && <tr><td colSpan={6}>No counts recorded.</td></tr>}
                {counts.map((c) => (
                  <tr key={c.id}>
                    <td>{new Date(c.created_at * 1000).toLocaleString()}</td>
                    <td>{itemName(c.item_id)}</td><td>{c.location}</td><td>{c.expected}</td><td>{c.counted}</td>
                    <td><span className={`pill ${c.variance === 0 ? "pill-ok" : "pill-warn"}`}>{c.variance > 0 ? `+${c.variance}` : c.variance}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {section === "suppliers" && (
        <>
          <div className="card composer">
            <h2>Register supplier</h2>
            <div className="form-row">
              <input placeholder="Supplier name" value={supForm.name} onChange={(e) => setSupForm((x) => ({ ...x, name: e.target.value }))} />
              <input placeholder="Contact person" value={supForm.contact} onChange={(e) => setSupForm((x) => ({ ...x, contact: e.target.value }))} />
              <input placeholder="Phone" value={supForm.phone} onChange={(e) => setSupForm((x) => ({ ...x, phone: e.target.value }))} />
              <button disabled={busy || !supForm.name.trim()}
                onClick={() => void run(() => invoke("inv_upsert_supplier", { token, name: supForm.name.trim(), contact: supForm.contact.trim() || null, phone: supForm.phone.trim() || null }))}>
                Save supplier
              </button>
            </div>
          </div>
          <div className="table-card">
            <h2>Suppliers</h2>
            <table>
              <thead><tr><th>Name</th><th>Contact</th><th>Phone</th><th>Status</th></tr></thead>
              <tbody>
                {suppliers.length === 0 && <tr><td colSpan={4}>No suppliers registered.</td></tr>}
                {suppliers.map((s) => (
                  <tr key={s.id}><td><b>{s.name}</b></td><td>{s.contact ?? ""}</td><td>{s.phone ?? ""}</td><td><span className="pill pill-ok">{s.status}</span></td></tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {section === "expenses" && (
        <>
          <div className="card composer">
            <h2>Submit expense</h2>
            <div className="form-row">
              <input placeholder="Category e.g. Utilities" value={expForm.category} onChange={(e) => setExpForm((x) => ({ ...x, category: e.target.value }))} />
              <input type="number" min="1" placeholder="Amount (UGX)" value={expForm.amount} onChange={(e) => setExpForm((x) => ({ ...x, amount: e.target.value }))} />
              <select value={expForm.method} onChange={(e) => setExpForm((x) => ({ ...x, method: e.target.value }))}>{["CASH", "MTN_MOMO", "AIRTEL_MONEY", "OTHER"].map((m) => <option key={m}>{m}</option>)}</select>
              <input placeholder="Description (required)" value={expForm.description} onChange={(e) => setExpForm((x) => ({ ...x, description: e.target.value }))} />
              <input placeholder="Reference (optional)" value={expForm.reference} onChange={(e) => setExpForm((x) => ({ ...x, reference: e.target.value }))} />
              <button disabled={busy || !expForm.category.trim() || !expForm.description.trim() || !expForm.amount}
                onClick={() => void run(() => invoke("inv_create_expense", { token, category: expForm.category.trim(), amountMinor: Number(expForm.amount), paymentMethod: expForm.method, description: expForm.description.trim(), reference: expForm.reference.trim() || null }))}>
                Submit for approval
              </button>
            </div>
          </div>
          <div className="table-card">
            <h2>Expenses{canApprove ? " — approval queue" : ""}</h2>
            <table>
              <thead><tr><th>Time</th><th>Category</th><th>Amount</th><th>Method</th><th>Description</th><th>Status</th>{canApprove && <th></th>}</tr></thead>
              <tbody>
                {expenses.length === 0 && <tr><td colSpan={7}>No expenses recorded.</td></tr>}
                {expenses.map((e) => (
                  <tr key={e.id}>
                    <td>{new Date(e.created_at * 1000).toLocaleString()}</td>
                    <td>{e.category}</td><td>{money(e.amount_minor)}</td><td>{e.payment_method}</td><td>{e.description}</td>
                    <td><span className={`pill ${e.status === "APPROVED" ? "pill-ok" : e.status === "REJECTED" ? "pill-warn" : "pill-muted"}`}>{e.status}</span></td>
                    {canApprove && (
                      <td className="form-row">
                        {e.status === "SUBMITTED" && (
                          <>
                            <button disabled={busy} onClick={() => void run(() => invoke("inv_approve_expense", { token, expenseId: e.id }))}>Approve</button>
                            <button className="danger" disabled={busy} onClick={() => void run(() => invoke("inv_reject_expense", { token, expenseId: e.id }))}>Reject</button>
                          </>
                        )}
                      </td>
                    )}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  );
}
