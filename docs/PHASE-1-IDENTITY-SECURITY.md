# Phase 1 — Identity & Security Core

## Objective
Build the security foundation before financial, stock, ordering or workforce transactions are introduced.

## Implemented
- Dedicated Tauri 2 CEO application.
- Dedicated Tauri 2 Restaurant application security foundation.
- Shared native Rust security core.
- Persistent local SQLite security database.
- Individual worker identities.
- One-time CEO bootstrap.
- Argon2id password hashing with per-password salts.
- Password policy and validation.
- Failed-login lockout.
- Eight-hour absolute sessions and thirty-minute idle timeout.
- Session-token hashing at rest.
- Password-change session invalidation.
- Active, suspended and inactive account states.
- Role-based authorization.
- Per-user permission grants/revokes.
- Native-side CEO ultimate authority and override behavior.
- Security and identity audit events.
- Stable per-device identity.
- Tauri command manifest restrictions.
- Restrictive CSP and prototype freezing.
- Current Tauri 2.x security baseline.
- Dependabot and automated verification workflow.
- Security threat model and baseline documentation.

## Security rules
1. Every worker receives an individual identity.
2. Sensitive authorization is enforced in native code, not only by React UI logic.
3. Passwords are never stored in plaintext.
4. Session tokens are never persisted in plaintext.
5. CEO is the ultimate authority and can override configured restrictions.
6. CEO overrides and security mutations are auditable.
7. Failed authentication is throttled through lockout.
8. Password changes invalidate existing sessions.
9. Critical records will not be silently deleted.
10. No production secrets belong in the repository.

## Phase boundary
Phase 1 provides durable local identity/security storage. Phase 2 will connect this identity foundation to the broader offline-first transaction database, device enrollment/revocation, cloud authentication, cryptographically authenticated synchronization, conflict/replay protection and backup/recovery controls.

## Validation
The repository includes TypeScript verification, frontend builds, native Rust unit tests and native application checks. Hosted GitHub Actions is currently returning synthetic `BuildFailed` / `startup_failure` runs with zero jobs in this repository; this is an external GitHub Actions workflow-registration/platform issue and is not a test result. Production verification remains blocked until GitHub Actions can execute the verification workflow.
