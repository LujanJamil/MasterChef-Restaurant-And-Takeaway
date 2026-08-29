import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { APP_NAME, LOCATION_NAME } from "@masterchef/domain";

type User = { id: string; username: string; display_name: string; role: string; status: string; created_at: number };
type Session = { token: string; user: User; expires_at: number };
type Attendance = {
  id: string;
  worker_id: string;
  device_id: string;
  clock_in_at: number;
  clock_out_at: number | null;
  break_minutes: number;
  status: string;
  notes?: string | null;
};

function fmt(ts: number) {
  return new Date(ts * 1000).toLocaleString();
}

export default function App() {
  const [session, setSession] = useState<Session | null>(null);
  const [setup, setSetup] = useState(false);
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [panel, setPanel] = useState<"home" | "attendance">("home");
  const [openAtt, setOpenAtt] = useState<Attendance | null>(null);
  const [history, setHistory] = useState<Attendance[]>([]);
  const [breakMin, setBreakMin] = useState(0);
  const [notes, setNotes] = useState("");

  async function login() {
    setBusy(true);
    setError("");
    try {
      const s = await invoke<Session>("login", { username, password });
      setSession(s);
      setPassword("");
    } catch {
      setError("Unable to sign in. Check your credentials or contact the CEO.");
    } finally {
      setBusy(false);
    }
  }
  async function bootstrap() {
    setBusy(true);
    setError("");
    try {
      await invoke("bootstrap_ceo", { username, displayName, password });
      setSetup(false);
      setDisplayName("");
      setPassword("");
      setError("Initial local administrator created. Sign in to continue.");
    } catch (e) {
      setError(String(e).includes("unauthorized") ? "This device has already been initialized." : String(e));
    } finally {
      setBusy(false);
    }
  }
  async function logout() {
    if (!session) return;
    try {
      await invoke("logout", { token: session.token });
    } finally {
      setSession(null);
      setPanel("home");
    }
  }

  async function refreshAttendance() {
    if (!session) return;
    try {
      const open = await invoke<Attendance | null>("workforce_open_attendance", { token: session.token });
      setOpenAtt(open);
      const list = await invoke<Attendance[]>("workforce_list_attendance", { token: session.token, limit: 20 });
      setHistory(list);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    if (session && panel === "attendance") void refreshAttendance();
  }, [session, panel]);

  async function clockIn() {
    if (!session) return;
    setBusy(true);
    setError("");
    try {
      await invoke("workforce_clock_in", { token: session.token, notes: notes || null });
      setNotes("");
      await refreshAttendance();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function clockOut() {
    if (!session) return;
    setBusy(true);
    setError("");
    try {
      await invoke("workforce_clock_out", {
        token: session.token,
        breakMinutes: breakMin,
        notes: notes || null,
      });
      setNotes("");
      setBreakMin(0);
      await refreshAttendance();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!session)
    return (
      <main className="auth">
        <section className="card login">
          <div className="logo">M</div>
          <div className="eyebrow">MASTER CHEF · RESTAURANT APPLICATION</div>
          <h1>{setup ? "Initialize this device" : "Sign in"}</h1>
          <p>{LOCATION_NAME} · 24/7 operations</p>
          {setup && (
            <>
              <label>
                Administrator name
                <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
              </label>
            </>
          )}
          <label>
            Username
            <input value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" />
          </label>
          <label>
            Password
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete={setup ? "new-password" : "current-password"}
            />
          </label>
          <button disabled={busy} onClick={() => void (setup ? bootstrap() : login())}>
            {busy ? "Please wait…" : setup ? "Initialize securely" : "Sign in"}
          </button>
          <button
            className="link"
            onClick={() => {
              setSetup(!setup);
              setError("");
            }}
          >
            {setup ? "Back to sign in" : "First launch? Initialize device"}
          </button>
          {error && <div className="error">{error}</div>}
          <small>
            Authentication is enforced by the native security core. Sessions are held in memory and critical identity
            events are audited.
          </small>
        </section>
      </main>
    );

  if (panel === "attendance") {
    return (
      <main className="app">
        <header>
          <div>
            <b>{APP_NAME} · Attendance</b>
            <span>{LOCATION_NAME}</span>
          </div>
          <div>
            <button className="secondary" onClick={() => setPanel("home")}>
              Back
            </button>
            <button className="secondary" onClick={() => void logout()}>
              Sign out
            </button>
          </div>
        </header>
        <section className="hero">
          <div>
            <div className="eyebrow">WORKFORCE · PHASE 3</div>
            <h1>Attendance</h1>
            <p>
              Signed in as <b>{session.user.display_name}</b> · {session.user.role}
            </p>
          </div>
          <div className="badge">
            {openAtt ? "CLOCKED IN" : "NOT CLOCKED IN"}
            <br />
            <b>{openAtt ? fmt(openAtt.clock_in_at) : "—"}</b>
          </div>
        </section>
        <section className="card" style={{ margin: "1rem", padding: "1.25rem" }}>
          {openAtt ? (
            <>
              <p>You are currently clocked in. Enter break minutes (if any) and clock out.</p>
              <label>
                Break minutes
                <input
                  type="number"
                  min={0}
                  value={breakMin}
                  onChange={(e) => setBreakMin(Number(e.target.value) || 0)}
                />
              </label>
              <label>
                Notes
                <input value={notes} onChange={(e) => setNotes(e.target.value)} />
              </label>
              <button disabled={busy} onClick={() => void clockOut()}>
                {busy ? "Saving…" : "Clock out"}
              </button>
            </>
          ) : (
            <>
              <p>Clock in to start your shift. This is recorded offline-first and queued for sync.</p>
              <label>
                Notes
                <input value={notes} onChange={(e) => setNotes(e.target.value)} />
              </label>
              <button disabled={busy} onClick={() => void clockIn()}>
                {busy ? "Saving…" : "Clock in"}
              </button>
            </>
          )}
          {error && <div className="error">{error}</div>}
        </section>
        <section className="grid">
          {history.map((a) => (
            <article className="card" key={a.id}>
              <span className="eyebrow">{a.status}</span>
              <h2>{fmt(a.clock_in_at)}</h2>
              <p>
                Out: {a.clock_out_at ? fmt(a.clock_out_at) : "—"} · Break: {a.break_minutes} min
              </p>
            </article>
          ))}
        </section>
        <footer>MasterChef · Najjera II Central Market · Phase 3 Workforce</footer>
      </main>
    );
  }

  const modules = [
    { name: "Orders", soon: true },
    { name: "Kitchen", soon: true },
    { name: "Cashier", soon: true },
    { name: "Store", soon: true },
    { name: "Attendance", soon: false },
    { name: "Delivery", soon: true },
  ];
  return (
    <main className="app">
      <header>
        <div>
          <b>{APP_NAME} · Restaurant</b>
          <span>{LOCATION_NAME}</span>
        </div>
        <div>
          <span className="secure">● SECURE SESSION</span>
          <button className="secondary" onClick={() => void logout()}>
            Sign out
          </button>
        </div>
      </header>
      <section className="hero">
        <div>
          <div className="eyebrow">RESTAURANT OPERATIONS APPLICATION</div>
          <h1>Operations are protected.</h1>
          <p>
            Signed in as <b>{session.user.display_name}</b> · {session.user.role}
          </p>
        </div>
        <div className="badge">
          OFFLINE-FIRST
          <br />
          <b>SECURITY + WORKFORCE ACTIVE</b>
        </div>
      </section>
      <section className="grid">
        {modules.map((m) => (
          <article
            className="card"
            key={m.name}
            style={{ cursor: m.soon ? "default" : "pointer" }}
            onClick={() => {
              if (!m.soon && m.name === "Attendance") setPanel("attendance");
            }}
          >
            <span className="eyebrow">MODULE</span>
            <h2>{m.name}</h2>
            <p>
              {m.soon
                ? "Access is evaluated against the authenticated worker identity before sensitive actions are executed."
                : "Clock in/out and view your attendance history. Offline-first with sync queue."}
            </p>
            <span className="pill">{m.soon ? "COMING SOON" : "PHASE 3 READY"}</span>
          </article>
        ))}
      </section>
      <footer>MasterChef · Najjera II Central Market</footer>
    </main>
  );
}
