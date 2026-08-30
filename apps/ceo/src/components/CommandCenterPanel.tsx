import { useMemo } from "react";
import type { WorkerRole } from "@masterchef/domain";

type UserStatus = "ACTIVE" | "SUSPENDED" | "INACTIVE";
interface User { id: string; username: string; display_name: string; role: WorkerRole; status: UserStatus; created_at: number; }
interface Audit { id: string; actor_user_id: string | null; action: string; target_type: string | null; target_id: string | null; details: Record<string, unknown>; device_id: string; created_at: number; }

const ROLE_ORDER: WorkerRole[] = ["CEO", "MANAGER", "WAITER", "COOK", "CASHIER", "STOREKEEPER", "DELIVERY_RIDER"];
const ROLE_LABEL: Record<WorkerRole, string> = { CEO: "CEO", MANAGER: "Manager", WAITER: "Waiter", COOK: "Cook", CASHIER: "Cashier", STOREKEEPER: "Storekeeper", DELIVERY_RIDER: "Delivery Rider" };
const SECURITY_PATTERN = /login|lockout|password|session|permission|override/i;
const fmtTime = (t: number) => new Date(t * 1000).toLocaleString();

export default function CommandCenterPanel({ users, audits, onRefresh }: { users: User[]; audits: Audit[]; onRefresh: () => void }) {
  const stats = useMemo(() => {
    const active = users.filter((u) => u.status === "ACTIVE");
    const suspended = users.filter((u) => u.status === "SUSPENDED");
    const nowSec = Math.floor(Date.now() / 1000);
    const last24h = audits.filter((a) => nowSec - a.created_at <= 86400);
    const security = audits.filter((a) => SECURITY_PATTERN.test(a.action));
    const byRole = ROLE_ORDER.map((r) => ({ role: r, count: users.filter((u) => u.role === r).length }));
    const maxRole = Math.max(1, ...byRole.map((b) => b.count));
    const latest = audits.length ? Math.max(...audits.map((a) => a.created_at)) : null;
    return { active, suspended, last24h, security, byRole, maxRole, latest };
  }, [users, audits]);

  const userName = (id: string | null) => (id ? users.find((u) => u.id === id)?.display_name ?? id.slice(0, 8) : "system");

  return (
    <>
      <style>{`
        .cc-tabs{display:flex;gap:.5rem;margin:0 0 1rem}
        .cc-tabs button{padding:.5rem 1rem;border-radius:.5rem;border:1px solid #d0d7de;background:#fff;cursor:pointer;font-weight:600}
        .cc-tabs button.active{background:#0b3d2e;color:#fff;border-color:#0b3d2e}
        .cc-note{background:#fff8e6;border:1px solid #f0d48a;border-radius:.6rem;padding:.7rem 1rem;font-size:.85rem;margin:0 0 1rem}
        .cc-bar-row{display:flex;align-items:center;gap:.6rem;margin:.3rem 0;font-size:.85rem}
        .cc-bar-row b{width:110px;text-align:right;font-weight:600}
        .cc-bar{height:10px;border-radius:6px;background:#1a7f37;min-width:4px}
        .cc-table{width:100%;border-collapse:collapse;font-size:.85rem}
        .cc-table th,.cc-table td{text-align:left;padding:.45rem .6rem;border-bottom:1px solid #e5e7eb;white-space:nowrap}
        .cc-table th{font-size:.68rem;text-transform:uppercase;letter-spacing:.05em;color:#57606a}
        .cc-scroll{overflow:auto;max-height:340px}
      `}</style>

      <div className="cc-note">
        Operational KPIs (today's sales, cash position, low stock, delayed orders) will appear here as Phase 10 aggregates the synced restaurant data. This center currently reflects identity, authorization and audit truth from this device's security core.
      </div>

      <section className="metrics">
        <article><span>Active identities</span><b>{stats.active.length}</b></article>
        <article><span>Suspended</span><b>{stats.suspended.length}</b></article>
        <article><span>Audit events · 24h</span><b>{stats.last24h.length}</b></article>
        <article><span>Security events</span><b>{stats.security.length}</b></article>
      </section>

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2>Workforce composition</h2>
            <p>Identities by role across this installation. Last security activity: {stats.latest ? fmtTime(stats.latest) : "none recorded"}.</p>
          </div>
          <button className="secondary" onClick={onRefresh}>Refresh</button>
        </div>
        {stats.byRole.map((b) => (
          <div className="cc-bar-row" key={b.role}>
            <b>{ROLE_LABEL[b.role]}</b>
            <div className="cc-bar" style={{ width: `${(b.count / stats.maxRole) * 60}%` }} />
            <span>{b.count}</span>
          </div>
        ))}
      </section>

      {stats.suspended.length > 0 && (
        <section className="panel">
          <div className="panel-head">
            <div>
              <h2>Needs attention — suspended identities</h2>
              <p>Suspended accounts cannot sign in. Reactivate them from Identity &amp; access.</p>
            </div>
          </div>
          <div className="cc-scroll">
            <table className="cc-table">
              <thead><tr><th>Name</th><th>Username</th><th>Role</th><th>Created</th></tr></thead>
              <tbody>
                {stats.suspended.map((u) => (
                  <tr key={u.id}><td>{u.display_name}</td><td>{u.username}</td><td>{ROLE_LABEL[u.role]}</td><td>{fmtTime(u.created_at)}</td></tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}

      <section className="panel">
        <div className="panel-head">
          <div>
            <h2>Audit timeline</h2>
            <p>Most recent audited events from the native security core.</p>
          </div>
        </div>
        <div className="cc-scroll">
          <table className="cc-table">
            <thead><tr><th>Time</th><th>Actor</th><th>Action</th><th>Target</th><th>Device</th></tr></thead>
            <tbody>
              {audits.length === 0 && <tr><td colSpan={5}>No audit events loaded.</td></tr>}
              {[...audits].sort((a, b) => b.created_at - a.created_at).slice(0, 25).map((a) => (
                <tr key={a.id}>
                  <td>{fmtTime(a.created_at)}</td>
                  <td>{userName(a.actor_user_id)}</td>
                  <td>{a.action}{SECURITY_PATTERN.test(a.action) ? " 🔐" : ""}</td>
                  <td>{a.target_type ?? "—"}</td>
                  <td><small>{a.device_id.slice(0, 8)}…</small></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </>
  );
}
