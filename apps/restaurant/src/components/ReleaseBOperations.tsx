import { useMemo, useState } from "react";
import type { StockLocation } from "@masterchef/domain";

const locations:StockLocation[]=["STORE","KITCHEN","DISPLAY","FRIDGE","ICE_CREAM","CHAPATI","LOCAL_FOOD","OTHER"];
const money=(n:number)=>`UGX ${n.toLocaleString()}`;
export interface ReleaseBOperationsProps { role:string; onError?:(message:string)=>void; }

export default function ReleaseBOperations({role,onError}:ReleaseBOperationsProps){
 const [section,setSection]=useState<"stock"|"purchases"|"expenses"|"equipment">("stock");
 const [search,setSearch]=useState("");
 const [location,setLocation]=useState<StockLocation>("STORE");
 const [notice,setNotice]=useState("Local operational ledger ready");
 const [items]=useState([
  {sku:"FLR-001",name:"Wheat Flour",unit:"KG",qty:42,reorder:10,cost:4200},
  {sku:"CHK-001",name:"Chicken",unit:"KG",qty:18,reorder:8,cost:14500},
  {sku:"OIL-001",name:"Cooking Oil",unit:"L",qty:25,reorder:7,cost:6500}
 ]);
 const visible=useMemo(()=>items.filter(i=>`${i.sku} ${i.name}`.toLowerCase().includes(search.toLowerCase())),[items,search]);
 const action=(message:string)=>{setNotice(message);onError?.("");};
 return <section className="release-b">
  <div className="section-head"><div><div className="eyebrow">RELEASE B · OPERATIONS</div><h1>Store & Operations</h1><p>Inventory, purchasing, expenses and equipment with accountable movements.</p></div><span className="badge">● LOCAL LEDGER</span></div>
  <div className="ops-tabs">{(["stock","purchases","expenses","equipment"] as const).map(x=><button key={x} className={section===x?"active":""} onClick={()=>setSection(x)}>{x[0].toUpperCase()+x.slice(1)}</button>)}</div>
  <div className="ops-banner"><b>{notice}</b><span>Offline writes are designed to synchronize by stable event ID.</span></div>
  {section==="stock"&&<><div className="toolbar"><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search SKU or stock item…"/><select value={location} onChange={e=>setLocation(e.target.value as StockLocation)}>{locations.map(l=><option key={l}>{l}</option>)}</select>{(role==="CEO"||role==="STOREKEEPER")&&<button onClick={()=>action("Stock adjustment queued for authorization")}>Record adjustment</button>}</div><div className="table-card"><table><thead><tr><th>Item</th><th>Unit</th><th>On hand</th><th>Reorder</th><th>Cost</th><th>State</th></tr></thead><tbody>{visible.map(i=><tr key={i.sku}><td><b>{i.name}</b><small>{i.sku}</small></td><td>{i.unit}</td><td>{i.qty}</td><td>{i.reorder}</td><td>{money(i.cost)}</td><td><span className={i.qty<=i.reorder?"warning":"ok"}>{i.qty<=i.reorder?"REORDER":"HEALTHY"}</span></td></tr>)}</tbody></table></div></>}
  {section==="purchases"&&<div className="ops-grid"><article className="card"><div className="eyebrow">RECEIVING</div><h2>Receive supplier stock</h2><p>Every receipt creates an immutable stock movement linked to supplier, actor, device and reference.</p><button onClick={()=>action("Purchase receiving workflow ready")}>New receiving record</button></article><article className="card"><div className="eyebrow">COST HISTORY</div><h2>Supplier prices</h2><p>Keep historical unit costs instead of overwriting what the restaurant paid previously.</p><button onClick={()=>action("Supplier cost history opened")}>View cost history</button></article></div>}
  {section==="expenses"&&<div className="ops-grid"><article className="card"><div className="eyebrow">EXPENSE CONTROL</div><h2>Record expense</h2><p>Cash, MTN Mobile Money and Airtel Money expenses require category, amount, description and accountable actor.</p><button onClick={()=>action("Expense draft created locally")}>Create expense</button></article><article className="card"><div className="eyebrow">APPROVAL</div><h2>Approval queue</h2><p>Submitted expenses remain pending until an authorized CEO or manager approves them.</p><button disabled={role!=="CEO"&&role!=="MANAGER"} onClick={()=>action("Approval queue opened")}>Open approvals</button></article></div>}
  {section==="equipment"&&<div className="ops-grid"><article className="card"><div className="eyebrow">ASSET ACCOUNTABILITY</div><h2>Equipment register</h2><p>Track asset tag, location, assigned worker, condition, damage, loss, repair and retirement history.</p><button disabled={role!=="CEO"} onClick={()=>action("Equipment register opened")}>Manage equipment</button></article><article className="card"><div className="eyebrow">AUDIT</div><h2>Condition events</h2><p>Equipment changes are compensating events, never silent edits to history.</p><button onClick={()=>action("Equipment history opened")}>View history</button></article></div>}
 </section>;
}
