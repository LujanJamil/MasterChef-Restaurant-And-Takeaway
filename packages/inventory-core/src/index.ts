import type { StockLocation, StockMovementType, StockUnit, ExpenseStatus, EquipmentCondition } from "@masterchef/domain";

export interface InventoryRow { id:string; sku:string; name:string; unit:StockUnit; category:string; reorderLevel:number; active:boolean; }
export interface Movement { id:string; itemId:string; type:StockMovementType; quantity:number; from:StockLocation|null; to:StockLocation|null; unitCostMinor:number|null; sourceId:string|null; reason:string|null; actorId:string; deviceId:string; createdAt:number; }
export interface ExpenseRow { id:string; category:string; amountMinor:number; currency:string; paymentMethod:"CASH"|"MTN_MOMO"|"AIRTEL_MONEY"|"OTHER"; status:ExpenseStatus; description:string; reference:string|null; createdBy:string; approvedBy:string|null; createdAt:number; }
export interface EquipmentRow { id:string; assetTag:string; name:string; serialNumber:string|null; area:string; assignedWorkerId:string|null; condition:EquipmentCondition; }
export interface SyncMutation { id:string; entity:string; operation:"CREATE"|"APPEND"|"FINALIZE"; payload:string; createdAt:number; attempts:number; }

export class InventoryLedger {
  private readonly movements: Movement[]=[];
  private readonly counts = new Map<string, number>();
  append(m:Movement):void {
    if(!m.id || !m.itemId || !m.actorId || !m.deviceId) throw new Error("movement identity is required");
    if(!Number.isFinite(m.quantity) || m.quantity<=0) throw new Error("movement quantity must be positive");
    if(m.unitCostMinor!==null && (!Number.isSafeInteger(m.unitCostMinor)||m.unitCostMinor<0)) throw new Error("invalid unit cost");
    if(this.movements.some(x=>x.id===m.id)) return;
    this.movements.push({...m});
  }
  balance(itemId:string, location:StockLocation):number {
    return this.movements.reduce((total,m)=>{
      if(m.itemId!==itemId) return total;
      if(m.to===location) return total+m.quantity;
      if(m.from===location) return total-m.quantity;
      return total;
    },0);
  }
  movementsFor(itemId:string):readonly Movement[]{ return this.movements.filter(m=>m.itemId===itemId); }
  recordCount(id:string, quantity:number):void { if(!Number.isFinite(quantity)||quantity<0) throw new Error("invalid count"); if(this.counts.has(id)) throw new Error("count already recorded"); this.counts.set(id,quantity); }
}

export function expectedStock(movements:readonly Movement[], itemId:string, location:StockLocation):number {
  return movements.reduce((n,m)=>m.itemId!==itemId?n:n+(m.to===location?m.quantity:0)-(m.from===location?m.quantity:0),0);
}

export function validateExpense(amountMinor:number, description:string):void {
  if(!Number.isSafeInteger(amountMinor)||amountMinor<=0) throw new Error("expense amount must be a positive integer minor unit");
  if(description.trim().length<3) throw new Error("expense description is required");
}

export function canApproveExpense(role:string, status:ExpenseStatus):boolean { return (role==="CEO"||role==="MANAGER") && status==="SUBMITTED"; }
export function canAdjustStock(role:string):boolean { return role==="CEO" || role==="STOREKEEPER"; }
export function canFinalizeCount(role:string):boolean { return role==="CEO" || role==="STOREKEEPER"; }
export function canManageEquipment(role:string):boolean { return role==="CEO"; }
