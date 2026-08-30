import test from "node:test";
import assert from "node:assert/strict";
import { InventoryLedger, expectedStock, validateExpense, canApproveExpense, canManageEquipment } from "../dist/index.js";

test("ledger is idempotent for replayed movement IDs",()=>{
  const l=new InventoryLedger();
  const m={id:"m1",itemId:"flour",type:"PURCHASE_RECEIPT",quantity:10,from:null,to:"STORE",unitCostMinor:5000,sourceId:"po1",reason:null,actorId:"ceo",deviceId:"pc",createdAt:1};
  l.append(m); l.append(m); assert.equal(l.balance("flour","STORE"),10); assert.equal(l.movementsFor("flour").length,1);
});

test("stock balance reflects issues and receipts",()=>{
  const ms=[{id:"1",itemId:"flour",type:"PURCHASE_RECEIPT",quantity:20,from:null,to:"STORE",unitCostMinor:1,sourceId:null,reason:null,actorId:"a",deviceId:"d",createdAt:1},{id:"2",itemId:"flour",type:"ISSUE",quantity:6,from:"STORE",to:"KITCHEN",unitCostMinor:null,sourceId:"issue",reason:"production",actorId:"a",deviceId:"d",createdAt:2}];
  assert.equal(expectedStock(ms,"flour","STORE"),14); assert.equal(expectedStock(ms,"flour","KITCHEN"),6);
});

test("financial and authorization invariants",()=>{
  assert.throws(()=>validateExpense(0,"x")); assert.doesNotThrow(()=>validateExpense(15000,"Gas refill"));
  assert.equal(canApproveExpense("CEO","SUBMITTED"),true); assert.equal(canApproveExpense("WAITER","SUBMITTED"),false); assert.equal(canManageEquipment("CEO"),true); assert.equal(canManageEquipment("MANAGER"),false);
});
