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

**Long gate vs. push timeout.** Git opens the connection to the remote *before*
running `pre-push` (that is how it fills in the remote SHAs it passes to the
hook), so the connection sits idle for as long as the gate takes. When the gate
outlasts the server's idle timeout, the push dies with exit 141 (SIGPIPE)
*after* printing "All checks passed", having pushed nothing — and `git push`
emits no output of its own, which makes it look like the hook failed.

Fix it per clone, without touching `~/.ssh/config`:

```bash
git config core.sshCommand 'ssh -o ServerAliveInterval=30 -o ServerAliveCountMax=20'
```

For a one-off push, `GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30' git push`
does the same thing. Never reach for `GIT_HOOK_SKIP=1` to get around it — that
skips the gate, which is a different problem.

The hook also drains its stdin in `parse_push_stdin` before any check runs. Git
streams the ref list into the hook, and an unread pipe is a second way a slow
hook can wedge a push.

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

## Single sign-on env vars

Off unless `RDC_OIDC_ISSUER` is set. Full operator guide, including worked
Keycloak / Auth0 / Entra setups and the reasoning behind each default, is in
`docs/SSO.md` — point operators there rather than restating it.

| Var | Purpose |
|-----|---------|
| `RDC_OIDC_ISSUER` | Provider issuer URL. **Presence enables the feature.** |
| `RDC_OIDC_AUDIENCE` | Required with the issuer; startup fails without it, because otherwise any token from that provider is accepted |
| `RDC_OIDC_ROLE_MAP` / `RDC_OIDC_ROLES_CLAIM` | `idp-group:rdc-role,…`, and which claim carries groups (default `groups`). A malformed map entry fails startup |
| `RDC_OIDC_DEFAULT_ROLE` | Unset means **reject** an unmapped user, not admit them with nothing |
| `RDC_OIDC_LINK_BY_EMAIL` | Off by default; account-takeover surface when the provider's addresses are unverified |
| `RDC_OIDC_CLIENT_ID` / `RDC_OIDC_REDIRECT_URI` | Enable the browser flow. A client id without a redirect URI fails startup |
| `RDC_OIDC_CLIENT_SECRET` | Optional (public client + PKCE). Redacted from debug output |
| `RDC_OIDC_POST_LOGIN_PATH` | Landing path after sign-in; must be a path inside the app |
| `RDC_OIDC_JWKS_TTL_SECS` / `RDC_OIDC_RESOLUTION_CACHE_SECS` | Key-set cache (3600) and identity cache (60). The latter is how long a revocation takes to bite |

Misconfiguration fails startup rather than silently disabling SSO: an operator
who sets the issuer and gets a server that ignores it has no way to tell until
someone cannot log in.

Unlock a locked admin: `cargo run --bin user-actions -- --username <n> --action unlock`,
or `PUT /admin/api/v1/users/{uuid}` with `{"status": "active"}`.

## Operating rules for the devops agent

- Never commit, push, or stash — leave changes in the working tree.
- Don't mutate lockfiles as a side effect; if a tool updates `Cargo.lock` /
  `fe/pnpm-lock.yaml` unintentionally, revert it before reporting.
