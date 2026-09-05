---
name: devops
description: r_data_core infra & tooling — rdt task runner, Docker compose services (postgres, pg-test, redis, node), .githooks/pre-push, .github/workflows CI, .rusty_dev_tool/config.toml, env files, SQLx migration infra. Read by the devops subagent.
color: purple
---

# DevOps & tooling

## rdt task runner (prefer over raw cargo/npm)

Defined in `.rusty_dev_tool/config.toml`:

```
rdt test | test-unit | test-fe | clippy | lint | test-e2e | test-e2e-report | clean-e2e | generate-ts | generate-ts-check
```

## Docker services

`postgres`, `pg-test`, `redis`, `node` (frontend). All frontend commands run via
`docker compose exec -T node …` — never `npm`/`pnpm`/`node` on the host.
`docker compose restart` does NOT apply env-var or network-alias changes — use
`docker compose up -d <service>` to recreate.

## Pre-push hook (`.githooks/pre-push`)

Enable: `git config core.hooksPath .githooks`. Runs:
fmt → clippy → `rdt test` → `rdt test-fe` → eslint → commit-lint.

Toggles in `.env.local`:
`GIT_HOOK_RUN_{FMT,CLIPPY,TEST,TEST_FE,LINT,COMMIT_LINT}=0`. Skip everything with
`GIT_HOOK_SKIP=1` — never set unless the user explicitly asks.

## SQLx

After schema changes: `cargo sqlx prepare --workspace -- --all-targets`, then
commit `.sqlx/`. Run migrations: `cargo sqlx migrate run`. Migrations are
append-only — never edit an existing migration.

## Env files are hook-blocked

`.env*`, `*.pem`, `*.key`, `credentials/`, `secrets/` are blocked by
`protect_secrets.js` for Read/Edit/Write/Bash/Glob. Surface needed values as
blockers with the exact lines to add — never attempt a bypass.

## CI

`.github/workflows/` mirrors the pre-push gate plus `generate-ts-check` and the
structural guards: `check-sql-boundary.sh`, `check-file-length.sh`, the
`architecture` layering test, `cargo-deny`, `cargo-machete`, and MSRV.

## Security env vars

Set on the `app` service (see `compose.yaml` and `docs/DEVELOPMENT.md`):

| Var | Purpose |
|-----|---------|
| `TRUSTED_PROXIES` | IPs/CIDRs whose `X-Forwarded-For` is believed. **Required behind a proxy** or the per-IP login limit becomes global. |
| `LOGIN_MAX_FAILED_ATTEMPTS` / `LOGIN_LOCKOUT_DURATION_SECS` | Account lockout threshold and expiry (`0` = permanent) |
| `LOGIN_RATE_LIMIT_MAX_ATTEMPTS` / `LOGIN_RATE_LIMIT_WINDOW_SECS` | Per-IP throttle on `/auth/login` and `/auth/register` |
| `SSRF_ALLOWED_HOSTS` | Hosts the workflow HTTP adapters may reach past the SSRF guard |
| `CORS_ORIGINS` | Must be explicit and non-wildcard in a hardened environment or the server refuses to start |

`APP_ENV` fails closed: only `development`/`dev`/`local`/`test` relax CORS and
the SSRF guard — staging and an unset value are hardened.

Unlock a locked admin: `cargo run --bin user_actions -- --username <n> --action unlock`,
or `PUT /admin/api/v1/users/{uuid}` with `{"status": "active"}`.

## Operating rules for the devops agent

- Never commit, push, or stash — leave changes in the working tree.
- Don't mutate lockfiles as a side effect; if a tool updates `Cargo.lock` /
  `fe/pnpm-lock.yaml` unintentionally, revert it before reporting.
