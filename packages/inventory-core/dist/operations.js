import { assertMoney, assertQuantity } from "@masterchef/domain";
export class OperationsEngine {
    ledger;
    constructor(ledger) {
        this.ledger = ledger;
    }
    receivePurchase(p) {
        if (!p.supplierId || !p.actorId || !p.deviceId || p.lines.length === 0)
            throw new Error("invalid purchase receipt");
        for (const line of p.lines) {
            assertQuantity(line.quantity);
            assertMoney(line.unitCostMinor);
            this.ledger.append({ id: `${p.id}:${line.itemId}`, itemId: line.itemId, type: "PURCHASE_RECEIPT", quantity: line.quantity, unit: line.unit, from: null, to: "STORE", unitCostMinor: line.unitCostMinor, sourceId: p.id, reason: p.reference, actorId: p.actorId, deviceId: p.deviceId, createdAt: p.receivedAt });
        }
    }
    issueStock(s) {
        assertQuantity(s.quantity);
        if (s.from === s.to)
            throw new Error("source and destination must differ");
        const available = this.ledger.balance(s.itemId, s.from);
        if (available < s.quantity)
            throw new Error("insufficient stock");
        this.ledger.append({ id: s.id, itemId: s.itemId, type: "ISSUE", quantity: s.quantity, unit: s.unit, from: s.from, to: s.to, unitCostMinor: null, sourceId: s.id, reason: s.reason, actorId: s.actorId, deviceId: s.deviceId, createdAt: s.createdAt });
    }
    waste(id, itemId, quantity, unit, location, actorId, deviceId, reason, now) {
        assertQuantity(quantity);
        if (reason.trim().length < 3)
            throw new Error("wastage reason required");
        if (this.ledger.balance(itemId, location) < quantity)
            throw new Error("insufficient stock");
        this.ledger.append({ id, itemId, type: "WASTAGE", quantity, unit, from: location, to: null, unitCostMinor: null, sourceId: id, reason, actorId, deviceId, createdAt: now });
    }
}
export function validatePurchaseLine(line) { assertQuantity(line.quantity); assertMoney(line.unitCostMinor); }
export function validateExpenseDraft(e) { assertMoney(e.amountMinor); if (!e.category.trim() || !e.description.trim())
    throw new Error("expense category and description are required"); }
export function validateEquipmentUpdate(e) { if (!e.assetTag.trim() || !e.name.trim())
    throw new Error("asset tag and equipment name are required"); if (e.condition === "LOST" && e.assignedWorkerId === null)
    return; }
