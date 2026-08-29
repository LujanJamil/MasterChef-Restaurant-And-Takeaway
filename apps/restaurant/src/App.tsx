import { APP_NAME, LOCATION_NAME } from "@masterchef/domain";
import { hasPermission } from "@masterchef/permissions";

export default function App() {
  const modules = [
    ["Orders", "orders.create"], ["Kitchen", "orders.view"], ["Cashier", "cash.open"],
    ["Store", "inventory.issue"], ["Attendance", "attendance.manage"], ["Delivery", "delivery.manage"]
  ] as const;
  return <main className="shell">
    <header className="topbar"><div><strong>{APP_NAME} · Restaurant</strong><span>{LOCATION_NAME} · OFFLINE-FIRST</span></div><div className="pill">PHASE 1 FOUNDATION</div></header>
    <section className="hero"><div><div className="eyebrow">RESTAURANT OPERATIONS APPLICATION</div><h1>Identity first.<br/>Operations next.</h1><p>Every worker will use an individual identity. Permissions will control sensitive actions while the CEO retains ultimate authority.</p></div></section>
    <section className="grid">{modules.map(([name, permission]) => <article className="card" key={name}><h2>{name}</h2><p className="muted">Required capability: <code>{permission}</code></p><strong>{hasPermission({role: "MANAGER"}, permission) ? "MANAGER ACCESS" : "RESTRICTED"}</strong></article>)}</section>
  </main>;
}
