import type { EquipmentCondition, StockLocation, StockUnit } from "@masterchef/domain";
import { InventoryLedger } from "./index.js";
export interface PurchaseLine {
    itemId: string;
    quantity: number;
    unit: StockUnit;
    unitCostMinor: number;
}
export interface PurchaseReceipt {
    id: string;
    supplierId: string;
    lines: PurchaseLine[];
    actorId: string;
    deviceId: string;
    receivedAt: number;
    reference: string | null;
}
export interface StockIssue {
    id: string;
    itemId: string;
    quantity: number;
    unit: StockUnit;
    from: StockLocation;
    to: StockLocation;
    actorId: string;
    deviceId: string;
    reason: string;
    createdAt: number;
}
export interface ExpenseDraft {
    id: string;
    category: string;
    amountMinor: number;
    currency: string;
    paymentMethod: "CASH" | "MTN_MOMO" | "AIRTEL_MONEY" | "OTHER";
    description: string;
    reference: string | null;
    createdBy: string;
    createdAt: number;
}
export interface EquipmentRecord {
    id: string;
    assetTag: string;
    name: string;
    serialNumber: string | null;
    area: string;
    assignedWorkerId: string | null;
    condition: EquipmentCondition;
}
export declare class OperationsEngine {
    readonly ledger: InventoryLedger;
    constructor(ledger: InventoryLedger);
    receivePurchase(p: PurchaseReceipt): void;
    issueStock(s: StockIssue): void;
    waste(id: string, itemId: string, quantity: number, unit: StockUnit, location: StockLocation, actorId: string, deviceId: string, reason: string, now: number): void;
}
export declare function validatePurchaseLine(line: PurchaseLine): void;
export declare function validateExpenseDraft(e: ExpenseDraft): void;
export declare function validateEquipmentUpdate(e: EquipmentRecord): void;
