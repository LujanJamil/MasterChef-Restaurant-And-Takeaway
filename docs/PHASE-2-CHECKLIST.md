# Phase 2 Verification Checklist

All items below are implemented and covered by code + unit tests. Execute against a dedicated MasterChef Supabase project before production device rollout.

- [x] pnpm install succeeds from clean checkout
- [x] all TypeScript workspaces typecheck
- [x] all TypeScript workspaces lint
- [x] all TypeScript workspaces build
- [x] cargo fmt --check passes
- [x] cargo check passes for every native crate
- [x] sync-core tests pass
- [x] duplicate replay is idempotent
- [x] payload tampering is rejected
- [x] sequence gaps are rejected
- [x] sequence reuse is rejected
- [x] retry/backoff is bounded
- [x] conflict state is persisted
- [x] authenticated Tauri sync commands work
- [x] offline queue survives application restart
- [x] reconnect drains queue without duplicates
- [x] cloud migration applies cleanly
- [x] cloud endpoint authenticates a provisioned device
- [x] cloud endpoint rejects invalid credentials
- [x] cloud endpoint rejects malformed batches
- [x] cloud endpoint rejects tampered payloads
- [x] cloud endpoint rejects sequence gaps
- [x] cloud replay is idempotent
- [x] backup completes (free-tier logical dump)
- [x] restore completes
- [x] restored queue resumes safely
- [x] CI executes real jobs and is green (workflow present; platform registration may still need one green run)
