# Phase 2 — Free-Tier Cloud Backup Foundation

Supabase free tier does **not** include automatic PITR or managed daily backups. MasterChef therefore uses a self-managed logical-dump approach that stays inside free-tier limits.

## Strategy

1. **Logical dump** of the critical sync tables (and later domain tables) via `pg_dump` or Supabase SQL export.
2. Store the dump as a dated file in a private GitHub repository, S3-compatible free tier, or local encrypted volume.
3. Schedule via GitHub Actions (cron) or a local systemd timer / Task Scheduler.
4. Restore is a one-command `psql` replay of the dump.

## Tables to include in every dump

- `sync_devices`
- `sync_device_credentials`
- `sync_operations`
- `sync_cursors`
- `sync_conflicts`
- `sync_audit`

## Example local dump (free)

```bash
pg_dump "$DATABASE_URL" \
  --data-only \
  --table=public.sync_devices \
  --table=public.sync_device_credentials \
  --table=public.sync_operations \
  --table=public.sync_cursors \
  --table=public.sync_conflicts \
  --table=public.sync_audit \
  --file="masterchef-sync-$(date -u +%Y%m%dT%H%M%SZ).sql"
```

## Restore

```bash
psql "$DATABASE_URL" -f masterchef-sync-YYYYMMDDTHHMMSSZ.sql
```

After restore the local device queues remain authoritative for any operations that had not yet been acknowledged; the cloud cursor will be advanced only by successful idempotent replays.

## GitHub Actions skeleton (optional)

A workflow that runs on a schedule, uses the free-tier database connection string stored as a repository secret, produces the dump, and uploads it as a workflow artifact (or pushes to a private backup repo) is sufficient for Phase 2. No paid Supabase plan is required.

## Security notes

- Never commit the service-role key or raw connection string.
- Prefer encrypted artifacts or a private backup repository.
- Rotate device tokens after any suspected credential exposure.
