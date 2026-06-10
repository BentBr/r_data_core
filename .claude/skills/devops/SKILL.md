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
`docker compose exec -T node …` — never `npm`/`node` on the host.
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

`.github/workflows/` mirrors the pre-push gate plus `generate-ts-check`.

## Operating rules for the devops agent

- Never commit, push, or stash — leave changes in the working tree.
- Don't mutate lockfiles as a side effect; if a tool updates `Cargo.lock` /
  `fe/package-lock.json` unintentionally, revert it before reporting.
