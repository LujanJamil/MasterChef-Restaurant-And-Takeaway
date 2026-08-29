import { useState } from "react";
import { APP_NAME, LOCATION_NAME } from "@masterchef/domain";
import { hasPermission } from "@masterchef/permissions";
import type { Permission } from "@masterchef/permissions";

const checks: Permission[] = ["system.manage", "users.manage", "pay.manage", "inventory.adjust", "audit.view"];

export default function App() {
  const [authenticated, setAuthenticated] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [message, setMessage] = useState("");

  function signIn() {
    if (!username.trim() || password.length < 8) {
      setMessage("Enter a username and a password of at least 8 characters.");
      return;
    }
    // Phase 1 establishes the identity/authorization contract. Credential persistence
    // and secure local authentication storage are implemented with the Phase 2 local DB.
    setAuthenticated(true);
    setMessage("");
  }

  if (!authenticated) return (
    <main className="shell login-shell">
      <section className="card login-card">
        <div className="eyebrow">CEO APPLICATION · PHASE 1</div>
        <h1>{APP_NAME}</h1>
        <p className="muted">Identity & Security Core</p>
        <p className="location">{LOCATION_NAME}</p>
        <label>Username<input value={username} onChange={e => setUsername(e.target.value)} autoComplete="username" /></label>
        <label>Password<input type="password" value={password} onChange={e => setPassword(e.target.value)} autoComplete="current-password" /></label>
        <button onClick={signIn}>Continue to CEO Console</button>
        {message && <p className="error">{message}</p>}
      </section>
    </main>
  );

  return (
    <main className="shell">
      <header className="topbar"><div><strong>{APP_NAME}</strong><span>{LOCATION_NAME}</span></div><button className="secondary" onClick={() => setAuthenticated(false)}>Sign out</button></header>
      <section className="hero"><div><div className="eyebrow">CEO COMMAND FOUNDATION</div><h1>Identity & Security Core</h1><p>Phase 1 establishes the permission model that will protect every future MasterChef operation.</p></div><div className="status">SECURITY FOUNDATION<br/><b>ACTIVE</b></div></section>
      <section className="grid">
        <article className="card"><h2>CEO authority</h2><p className="muted">The CEO is the ultimate authority. CEO permission checks always resolve to allowed, while the action remains auditable.</p><div className="pill">CEO OVERRIDE ENABLED</div></article>
        <article className="card"><h2>Capabilities</h2><ul>{checks.map(p => <li key={p}><code>{p}</code><span>{hasPermission({ role: "CEO" }, p) ? "ALLOWED" : "DENIED"}</span></li>)}</ul></article>
        <article className="card"><h2>Phase 1 boundary</h2><p className="muted">Authentication storage, device registration, audit persistence and offline synchronization will connect to the local database in Phase 2.</p></article>
      </section>
    </main>
  );
}
