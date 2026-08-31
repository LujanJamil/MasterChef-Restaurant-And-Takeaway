import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SyncOperation {
  id: string;
  device_id: string;
  entity_type: string;
  entity_id: string;
  operation: string;
  payload: unknown;
  payload_hash: string;
  sequence: number;
  created_at: number;
}

const env = (import.meta as unknown as { env?: Record<string, string | undefined> }).env ?? {};
const SYNC_URL = env.VITE_SYNC_FUNCTION_URL ?? "";
const DEVICE_ID = env.VITE_SYNC_DEVICE_ID ?? "";
const DEVICE_TOKEN = env.VITE_SYNC_DEVICE_TOKEN ?? "";
const configured = Boolean(SYNC_URL && DEVICE_ID && DEVICE_TOKEN);

const BATCH_LIMIT = 100;
const INTERVAL_MS = 15000;

export default function SyncUploader({ token }: { token: string }) {
  const [pending, setPending] = useState(0);
  const [syncing, setSyncing] = useState(false);
  const [lastResult, setLastResult] = useState("");
  const [online, setOnline] = useState(navigator.onLine);
  const inFlight = useRef(false);

  const refreshCount = useCallback(async () => {
    try {
      setPending(await invoke<number>("sync_pending_count", { token }));
    } catch {
      /* auth handled by parent */
    }
  }, [token]);

  const flush = useCallback(async () => {
    if (!configured || inFlight.current || !navigator.onLine) return;
    inFlight.current = true;
    setSyncing(true);
    try {
      const due = await invoke<SyncOperation[]>("sync_due", { token, limit: BATCH_LIMIT });
      if (due.length === 0) {
        setLastResult("");
        return;
      }
      await invoke("sync_claim", { token, ids: due.map((o) => o.id) });
      const res = await fetch(SYNC_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-device-id": DEVICE_ID,
          "x-device-token": DEVICE_TOKEN,
        },
        body: JSON.stringify({
          operations: due.map((o) => ({
            id: o.id,
            entity_type: o.entity_type,
            entity_id: o.entity_id,
            operation: o.operation,
            payload: o.payload,
            payload_hash: o.payload_hash,
            sequence: o.sequence,
            client_created_at: new Date(o.created_at * 1000).toISOString(),
          })),
        }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        const msg = typeof data?.error === "string" ? data.error : `HTTP ${res.status}`;
        for (const o of due) await invoke("sync_fail", { token, id: o.id, error: msg });
        setLastResult(`Sync rejected: ${msg}`);
        return;
      }
      const rejected = new Map<string, string>(
        (Array.isArray(data?.rejected) ? data.rejected : []).map((r: { id: string; error: string }) => [r.id, r.error])
      );
      let acceptedCount = 0;
      for (const o of due) {
        if (rejected.has(o.id)) {
          await invoke("sync_fail", { token, id: o.id, error: rejected.get(o.id) ?? "rejected" });
        } else {
          await invoke("sync_ack", { token, id: o.id });
          acceptedCount += 1;
        }
      }
      setLastResult(
        rejected.size > 0
          ? `Synced ${acceptedCount}; ${rejected.size} rejected (will retry)`
          : `Synced ${acceptedCount} operation${acceptedCount === 1 ? "" : "s"}`
      );
    } catch (e) {
      setLastResult(`Offline or unreachable: ${String(e instanceof Error ? e.message : e)}`);
    } finally {
      inFlight.current = false;
      setSyncing(false);
      await refreshCount();
    }
  }, [token, refreshCount]);

  useEffect(() => {
    void refreshCount();
    const t = setInterval(() => void flush(), INTERVAL_MS);
    const onOnline = () => {
      setOnline(true);
      void flush();
    };
    const onOffline = () => setOnline(false);
    window.addEventListener("online", onOnline);
    window.addEventListener("offline", onOffline);
    return () => {
      clearInterval(t);
      window.removeEventListener("online", onOnline);
      window.removeEventListener("offline", onOffline);
    };
  }, [flush, refreshCount]);

  if (!configured) return null;

  return (
    <div className="sync-status" title={lastResult || "Sync idle"}>
      <span className={`pill ${online ? "pill-ok" : "pill-warn"}`}>{online ? "ONLINE" : "OFFLINE"}</span>
      <span className={`pill ${pending > 0 ? "pill-warn" : "pill-ok"}`}>
        {syncing ? "SYNCING…" : pending > 0 ? `${pending} QUEUED` : "SYNCED"}
      </span>
      {lastResult && <small>{lastResult}</small>}
    </div>
  );
}
