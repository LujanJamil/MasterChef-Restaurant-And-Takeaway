export class InventoryLedger {
    movements = [];
    counts = new Map();
    append(m) {
        if (!m.id || !m.itemId || !m.actorId || !m.deviceId)
            throw new Error("movement identity is required");
        if (!Number.isFinite(m.quantity) || m.quantity <= 0)
            throw new Error("movement quantity must be positive");
        if (m.unitCostMinor !== null && (!Number.isSafeInteger(m.unitCostMinor) || m.unitCostMinor < 0))
            throw new Error("invalid unit cost");
        if (this.movements.some(x => x.id === m.id))
            return;
        this.movements.push({ ...m });
    }
    balance(itemId, location) {
        return this.movements.reduce((total, m) => {
            if (m.itemId !== itemId)
                return total;
            if (m.to === location)
                return total + m.quantity;
            if (m.from === location)
                return total - m.quantity;
            return total;
        }, 0);
    }
    movementsFor(itemId) { return this.movements.filter(m => m.itemId === itemId); }
    recordCount(id, quantity) { if (!Number.isFinite(quantity) || quantity < 0)
        throw new Error("invalid count"); if (this.counts.has(id))
        throw new Error("count already recorded"); this.counts.set(id, quantity); }
}
export function expectedStock(movements, itemId, location) {
    return movements.reduce((n, m) => m.itemId !== itemId ? n : n + (m.to === location ? m.quantity : 0) - (m.from === location ? m.quantity : 0), 0);
}
export function validateExpense(amountMinor, description) {
    if (!Number.isSafeInteger(amountMinor) || amountMinor <= 0)
        throw new Error("expense amount must be a positive integer minor unit");
    if (description.trim().length < 3)
        throw new Error("expense description is required");
}
export function canApproveExpense(role, status) { return (role === "CEO" || role === "MANAGER") && status === "SUBMITTED"; }
export function canAdjustStock(role) { return role === "CEO" || role === "STOREKEEPER"; }
export function canFinalizeCount(role) { return role === "CEO" || role === "STOREKEEPER"; }
export function canManageEquipment(role) { return role === "CEO"; }
