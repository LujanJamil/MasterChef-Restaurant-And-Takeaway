export type StockLocation = "STORE" | "KITCHEN" | "DISPLAY" | "FRIDGE" | "ICE_CREAM" | "CHAPATI" | "LOCAL_FOOD" | "OTHER";
export type StockMovementType = "OPENING" | "PURCHASE_RECEIPT" | "ISSUE" | "RETURN" | "TRANSFER" | "WASTAGE" | "DAMAGE" | "EXPIRY" | "COUNT_ADJUSTMENT" | "REVERSAL";
export type StockUnit = "PCS" | "KG" | "G" | "L" | "ML" | "BOX" | "PACK" | "BOTTLE" | "PLATE";
export type StockStatus = "ACTIVE" | "ARCHIVED";
export type ExpenseStatus = "DRAFT" | "SUBMITTED" | "APPROVED" | "REJECTED" | "REVERSED";
export type EquipmentCondition = "NEW" | "GOOD" | "FAIR" | "DAMAGED" | "LOST" | "UNDER_REPAIR" | "RETIRED";

export interface InventoryItem { id:string; sku:string; name:string; unit:StockUnit; category:string; reorderLevel:number; active:boolean; createdAt:number; updatedAt:number; }
export interface Supplier { id:string; name:string; phone:string|null; address:string|null; active:boolean; createdAt:number; updatedAt:number; }
export interface SupplierPrice { id:string; supplierId:string; itemId:string; unitCostMinor:number; currency:string; effectiveFrom:number; effectiveTo:number|null; }
export interface StockMovement { id:string; itemId:string; type:StockMovementType; quantity:number; unit:StockUnit; fromLocation:StockLocation|null; toLocation:StockLocation|null; unitCostMinor:number|null; sourceId:string|null; reason:string|null; actorId:string; deviceId:string; createdAt:number; }
export interface StockBalance { itemId:string; location:StockLocation; quantity:number; unit:StockUnit; }
export interface StockCount { id:string; location:StockLocation; status:"OPEN"|"FINALIZED"; countedBy:string; createdAt:number; finalizedAt:number|null; }
export interface StockCountLine { id:string; countId:string; itemId:string; expectedQuantity:number; countedQuantity:number; variance:number; }
export interface Expense { id:string; category:string; amountMinor:number; currency:string; paymentMethod:"CASH"|"MTN_MOMO"|"AIRTEL_MONEY"|"OTHER"; status:ExpenseStatus; description:string; reference:string|null; createdBy:string; approvedBy:string|null; createdAt:number; updatedAt:number; }
export interface Equipment { id:string; assetTag:string; name:string; serialNumber:string|null; area:StockLocation|"CUSTOMER_AREA"|"OFFICE"; assignedWorkerId:string|null; condition:EquipmentCondition; createdAt:number; updatedAt:number; }
export interface EquipmentEvent { id:string; equipmentId:string; type:"ASSIGNED"|"UNASSIGNED"|"DAMAGE_REPORTED"|"LOST_REPORTED"|"REPAIR_STARTED"|"REPAIR_COMPLETED"|"REPLACED"|"RETIRED"; actorId:string; workerId:string|null; reason:string|null; createdAt:number; }

export function assertQuantity(quantity:number):number { if(!Number.isFinite(quantity)||quantity<=0) throw new Error("Quantity must be greater than zero"); return quantity; }
export function assertMoney(amountMinor:number):number { if(!Number.isSafeInteger(amountMinor)||amountMinor<0) throw new Error("Money must be a non-negative safe integer minor unit"); return amountMinor; }
export function calculateVariance(expected:number,counted:number):number { if(!Number.isFinite(expected)||!Number.isFinite(counted)) throw new Error("Invalid stock count"); return counted-expected; }
