import type { StockLocation, StockMovementType, StockUnit, ExpenseStatus, EquipmentCondition } from "@masterchef/domain";
export interface InventoryRow {
    id: string;
    sku: string;
    name: string;
    unit: StockUnit;
    category: string;
    reorderLevel: number;
    active: boolean;
}
export interface Movement {
    id: string;
    itemId: string;
    type: StockMovementType;
    quantity: number;
    unit: StockUnit;
    from: StockLocation | null;
    to: StockLocation | null;
    unitCostMinor: number | null;
    sourceId: string | null;
    reason: string | null;
    actorId: string;
    deviceId: string;
    createdAt: number;
}
export interface ExpenseRow {
    id: string;
    category: string;
    amountMinor: number;
    currency: string;
    paymentMethod: "CASH" | "MTN_MOMO" | "AIRTEL_MONEY" | "OTHER";
    status: ExpenseStatus;
    description: string;
    reference: string | null;
    createdBy: string;
    approvedBy: string | null;
    createdAt: number;
}
export interface EquipmentRow {
    id: string;
    assetTag: string;
    name: string;
    serialNumber: string | null;
    area: string;
    assignedWorkerId: string | null;
    condition: EquipmentCondition;
}
export interface SyncMutation {
    id: string;
    entity: string;
    operation: "CREATE" | "APPEND" | "FINALIZE";
    payload: string;
    createdAt: number;
    attempts: number;
}
export declare class InventoryLedger {
    private readonly movements;
    private readonly counts;
    append(m: Movement): void;
    balance(itemId: string, location: StockLocation): number;
    movementsFor(itemId: string): readonly Movement[];
    recordCount(id: string, quantity: number): void;
}
export declare function expectedStock(movements: readonly Movement[], itemId: string, location: StockLocation): number;
export declare function validateExpense(amountMinor: number, description: string): void;
export declare function canApproveExpense(role: string, status: ExpenseStatus): boolean;
export declare function canAdjustStock(role: string): boolean;
export declare function canFinalizeCount(role: string): boolean;
export declare function canManageEquipment(role: string): boolean;
