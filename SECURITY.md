# MasterChef Security Baseline

## Security boundary

The React frontend is treated as untrusted presentation code. Sensitive authorization, password verification, session validation and identity mutations are enforced in the native Rust security core. The frontend never decides whether a sensitive operation is allowed.

## Phase 1 controls

- Individual user identities.
- Argon2id password hashing with per-password salts.
- Minimum 12-character passwords with upper/lower/numeric requirements.
- Short-lived bearer sessions kept in application memory on the frontend.
- Session records store only a SHA-256 digest of the random session token.
- Eight-hour absolute session lifetime.
- Thirty-minute idle session timeout.
- Five failed logins trigger a fifteen-minute lockout.
- Password changes increment an authentication version and invalidate existing sessions.
- Active/suspended/inactive account states.
- Role-based permissions with explicit per-user grant/revoke overrides.
- CEO authority is enforced in the native core and remains auditable.
- Critical identity/security actions are written to an audit table.
- SQLite foreign keys and WAL mode are enabled for the local security store.
- Tauri command surface is explicitly enumerated at build time.
- Tauri prototype freezing is enabled.
- A restrictive Content Security Policy is configured.
- Tauri is kept on the current supported 2.x line rather than the older initial scaffold version.
- Dependency update automation is enabled through Dependabot.
- CI runs type checking, frontend builds, Rust tests and native application checks.

## Secrets policy

Never commit passwords, session tokens, API keys, database credentials, payment credentials, signing certificates, or private keys. Production secrets belong in the deployment/operating-system secret mechanism appropriate to the environment.

## Audit policy

Do not silently delete financial, inventory, identity or security history. Future business modules should use reversal/adjustment records where corrections are necessary.

## Threats considered in Phase 1

1. Password database compromise — mitigated with Argon2id hashes and unique salts.
2. Credential stuffing — mitigated by login throttling/lockout; stronger device/network controls will be added later.
3. Stolen session token — mitigated by short lifetime, idle timeout and token hashing at rest; OS-level secure storage and device binding are future hardening layers.
4. Frontend privilege escalation — mitigated by native-side authorization.
5. Unauthorized account persistence — mitigated by account status and session-version invalidation.
6. Insider privilege abuse — mitigated by individual identities and audit events.
7. Remote-origin command abuse — mitigated by Tauri security configuration and command manifest restrictions.

## Phase 2 security requirements

- Offline-first transaction security.
- Device enrollment and revocation.
- Cloud authentication and authorization.
- Cryptographically authenticated synchronization.
- Conflict and replay protection.
- Encrypted backup strategy.
- Key management.
- Stronger device binding.
- Recovery and restore controls.
- Centralized audit synchronization.
- Security incident workflow.

Security is a continuous engineering requirement; Phase 1 is a foundation, not a claim that the finished restaurant platform is immune to compromise.
