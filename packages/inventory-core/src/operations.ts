import type { EquipmentCondition, StockLocation, StockMovementType, StockUnit } from "@masterchef/domain";
import { assertMoney, assertQuantity } from "@masterchef/domain";
import { InventoryLedger, type Movement } from "./index.js";

export interface PurchaseLine { itemId:string; quantity:number; unit:StockUnit; unitCostMinor:number; }
export interface PurchaseReceipt { id:string; supplierId:string; lines:PurchaseLine[]; actorId:string; deviceId:string; receivedAt:number; reference:string|null; }
export interface StockIssue { id:string; itemId:string; quantity:number; unit:StockUnit; from:StockLocation; to:StockLocation; actorId:string; deviceId:string; reason:string; createdAt:number; }
export interface ExpenseDraft { id:string; category:string; amountMinor:number; currency:string; paymentMethod:"CASH"|"MTN_MOMO"|"AIRTEL_MONEY"|"OTHER"; description:string; reference:string|null; createdBy:string; createdAt:number; }
export interface EquipmentRecord { id:string; assetTag:string; name:string; serialNumber:string|null; area:string; assignedWorkerId:string|null; condition:EquipmentCondition; }

export class OperationsEngine {
  constructor(readonly ledger:InventoryLedger) {}
  receivePurchase(p:PurchaseReceipt):void {
    if(!p.supplierId||!p.actorId||!p.deviceId||p.lines.length===0) throw new Error("invalid purchase receipt");
    for(const line of p.lines){ assertQuantity(line.quantity); assertMoney(line.unitCostMinor); this.ledger.append({id:`${p.id}:${line.itemId}`,itemId:line.itemId,type:"PURCHASE_RECEIPT",quantity:line.quantity,unit:line.unit,from:null,to:"STORE",unitCostMinor:line.unitCostMinor,sourceId:p.id,reason:p.reference,actorId:p.actorId,deviceId:p.deviceId,createdAt:p.receivedAt}); }
  }
  issueStock(s:StockIssue):void {
    assertQuantity(s.quantity); if(s.from===s.to) throw new Error("source and destination must differ");
    const available=this.ledger.balance(s.itemId,s.from); if(available<s.quantity) throw new Error("insufficient stock");
    this.ledger.append({id:s.id,itemId:s.itemId,type:"ISSUE",quantity:s.quantity,unit:s.unit,from:s.from,to:s.to,unitCostMinor:null,sourceId:s.id,reason:s.reason,actorId:s.actorId,deviceId:s.deviceId,createdAt:s.createdAt});
  }
  waste(id:string,itemId:string,quantity:number,unit:StockUnit,location:StockLocation,actorId:string,deviceId:string,reason:string,now:number):void {
    assertQuantity(quantity); if(reason.trim().length<3) throw new Error("wastage reason required"); if(this.ledger.balance(itemId,location)<quantity) throw new Error("insufficient stock");
    this.ledger.append({id,itemId,type:"WASTAGE",quantity,unit,from:location,to:null,unitCostMinor:null,sourceId:id,reason,actorId,deviceId,createdAt:now});
  }
}

export function validatePurchaseLine(line:PurchaseLine):void { assertQuantity(line.quantity); assertMoney(line.unitCostMinor); }
export function validateExpenseDraft(e:ExpenseDraft):void { assertMoney(e.amountMinor); if(!e.category.trim()||!e.description.trim()) throw new Error("expense category and description are required"); }
export function validateEquipmentUpdate(e:EquipmentRecord):void { if(!e.assetTag.trim()||!e.name.trim()) throw new Error("asset tag and equipment name are required"); if(e.condition==="LOST"&&e.assignedWorkerId===null) return; }
