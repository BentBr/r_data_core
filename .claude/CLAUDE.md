# r_data_core

A Rust-based Master Data Management (MDM) system. Modular workspace with Actix-web API, SQLx persistence, DSL-based workflows, and a Vue3 admin frontend.

## Quick Reference

This project uses `rdt` (rusty_dev_tool) as the standard task runner. **Always prefer `rdt` commands** over raw `cargo`/`npm`/`pnpm` invocations. **Never run `npm`/`pnpm`/`node` on the host** — all frontend commands run inside the Docker `node` container.

```bash
rdt test            # Run all workspace tests
rdt test-unit       # Unit tests only
rdt test-fe         # Admin frontend vitest (Docker)
rdt clippy          # Clippy with strict lints (nightly)
rdt lint            # ESLint + Prettier for frontend (Docker)
rdt test-e2e        # Playwright E2E tests (Docker)
rdt clean-e2e       # Remove E2E test data from DB
rdt generate-ts     # Generate TS types + validation constants from Rust structs
rdt generate-ts-check # Same + fail if generated files differ from committed
cargo fmt --all     # Format Rust code
```

### Running Services

```bash
cargo run --bin r_data_core              # Main server
cargo run --bin r_data_core_worker       # Workflow worker
cargo run --bin r_data_core_maintenance  # Maintenance worker
```

### Database

```bash
cargo sqlx prepare --workspace -- --all-targets   # After schema changes
cargo sqlx migrate run                             # Run migrations
```

### TypeScript Bindings

Rust API structs generate TypeScript type definitions via `ts-rs`.

- Add `#[derive(TS)]` + `#[ts(export)]` to Rust structs that should be exported
- Use `#[ts(type = "string")]` for `Uuid` and `OffsetDateTime` fields
- Use `#[ts(type = "unknown")]` for `serde_json::Value` fields
- Run `rdt generate-ts` after changing exported Rust structs
- Generated files live in `fe/src/types/generated/` — committed to git, never hand-edited
- Validation constants defined in `crates/core/src/validation/` are exported to `validation.ts`
- Frontend form schemas use `satisfies z.ZodType<GeneratedType>` for type safety
- CI and pre-push hook verify bindings are up to date

## Workspace Crates

| Crate | Path | Role | Skill doc |
|-------|------|------|-------|
| core | `crates/core/` | Domain models, config, cache, field types | `backend/core.md` |
| api | `crates/api/` | Actix-web HTTP endpoints, middleware, auth | `backend/api.md` |
| persistence | `crates/persistence/` | SQLx repositories | `backend/persistence.md` |
| services | `crates/services/` | Business logic, adapters | `backend/services.md` |
| workflow | `crates/workflow/` | DSL engine, job queue | `backend/workflow.md` |
| worker | `crates/worker/` | Background tasks, scheduler | `backend/worker.md` |
| license | `crates/license/` | License verification | `backend/license.md` |
| test-support | `crates/test-support/` | Test helpers (dev-only) | `backend/SKILL.md` |

Frontend: `fe/` — Vue3 + TypeScript + Vuetify admin dashboard. See the `frontend` skill.

## Skills Reference

Documentation is organized into grouped skills under `.claude/skills/<name>/SKILL.md`:

- **`backend`** — crate dependency graph, layering rules, scoped-check commands, TS bindings. Supporting docs: `core.md`, `services.md`, `persistence.md`, `api.md`, `workflow.md`, `worker.md`, `license.md`, `database.md`, `api-reference.md`, `conventions.md` (clippy/MSRV/file-length), `quality.md` (testing/review).
- **`frontend`** — Vue3/TS/Vite/Vuetify/Pinia, generated-types boundary, EN/DE i18n. Supporting: `conventions.md`.
- **`architecture`** — system view: crate dependency graph, layering, key concepts.
- **`git`** — conventional commits + decision matrix, pre-push pipeline, `GIT_HOOK_*` toggles. Supporting: `conflicts.md` (merge-conflict classification).
- **`quality-assurance`** — the `/qa` pipeline (static gate → ts-binding check → agent delegation → review/sync).
- **`devops`** — rdt, Docker compose, `.githooks`, CI, env, SQLx migration infra.

## Code Quality

- Clippy enforced strictly: `-D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery`
- MSRV: 1.96.0
- File length: 300 lines soft cap, 500 lines hard cap
- See the `backend` skill's `conventions.md` for full policy

## Agents

Layer-specialist subagents under `.claude/agents/`. The orchestrator delegates scoped work; each has a fixed scope + delegation table, runs scoped checks only (never full `rdt test`/CI), reports back, and never commits.

- **Backend:** `core-domain` (crates/core), `services` (services + workflow + worker), `persistence` (persistence + migrations), `api` (Actix/auth/DTOs/ts-bindings), `rust-tester` (tests only).
- **Frontend:** `frontend-ui` (components/views/SCSS), `frontend-state` (Pinia/composables/services), `frontend-infrastructure` (router/plugins/types/zod), `frontend-tester` (vitest).
- **Cross-cutting:** `devops`, `conflict-resolver`, `quality-assurance` (read-only), `skills-maintainer`, `docs-writer`, `i18n-translator` (`fe/translations/{en,de}.json` parity).

## Meta-commands

- **`/commit`** — propose conventional commits and commit (no push, no `Co-Authored-By`).
- **`/push`** — push, drive the `.githooks/pre-push` pipeline, route failures to agents, monitor PRs.
- **`/qa`** — run the QA pipeline (`.claude/skills/quality-assurance/SKILL.md`).
- **`/analyze-claude`** — audit the `.claude/` setup and propose fixes in plan mode.

Task-runner commands also exist: `/build`, `/clippy`, `/test`, `/lint`, `/migrate`, `/run`, `/docker`, `/entity`, `/workflow`, `/prepare-sqlx`, `/reset-db`, `/review`.

## Workflow Rules

- **Be precise and brief.** Answer directly, no preamble or filler. Lead with the result; keep explanations to what's needed. Say what you actually did, did not do, or are unsure of — never pad.
- **Never commit unless explicitly asked.** Do not auto-commit, stage, or create commits after completing work. The user reviews all changes before committing.

## Security Hooks

- `protect_secrets.js` (PreToolUse) blocks access to `.env*`, `.pem`, `.key`, `credentials/`, `secrets/` for Read/Edit/Write/Bash/Glob/Grep.
- `protect_generated.cjs` (PreToolUse) blocks hand-edits to `fe/src/types/generated/` — run `rdt generate-ts` instead.
- PostToolUse format/lint hooks: `rustfmt` (`.rs`), `prettier` + `eslint` (`fe/` files).
