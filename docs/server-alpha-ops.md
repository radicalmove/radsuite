# RADsuite Alpha Server Operations

## Host Assumptions

The internal alpha server will run on the existing Ubuntu Mac Mini.

Baseline assumptions:

- Ubuntu Server host
- `nginx` reverse proxy
- `systemd` process manager
- RADsuite server binary listening on `127.0.0.1:8088`
- Local asset storage under `RADSUITE_ASSET_ROOT`
- GitHub is the source of truth for code

## Database

The earliest alpha can use SQLite through `RADSUITE_DATABASE_URL`.

Before serious multi-user testing, reassess whether the server should move to PostgreSQL. The Rust data layer already includes PostgreSQL support through SQLx, but the first server skeleton only exercises SQLite.

## Assets

Assets can start on local disk behind:

```bash
RADSUITE_ASSET_ROOT=/home/fldadmin/radsuite/data/assets
```

Backups must include both:

- Server database
- Asset root

## Initial Verification

Run on the Ubuntu Mac Mini after deployment:

```bash
systemctl status radsuite --no-pager
curl -fsS http://127.0.0.1:8088/healthz
```

Expected health response:

```json
{"status":"ok"}
```

## Reverse Proxy

Nginx should proxy internal alpha traffic to:

```text
127.0.0.1:8088
```

TLS and stable public ingress can be decided after the internal alpha server is reachable and backed up.

## Deployment Boundary

This document is not yet a production runbook. Before external release, add:

- Dedicated deployment user and directory layout
- `systemd` service file
- Nginx site config
- Backup script and restore test
- Log rotation
- Admin bootstrap process
- Upgrade and rollback procedure
