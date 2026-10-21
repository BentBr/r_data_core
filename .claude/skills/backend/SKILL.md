---
name: backend
description: Rust backend for r_data_core — crate dependency graph (core → services → persistence → api + workflow/worker), layering rules, per-crate maps, conventions, and scoped-check commands. Read by all backend subagents.
color: green
---

# Backend (Rust workspace)

## Crate dependency graph & layering

| Crate | Path | Role | May depend on (per `tests/architecture.rs`) |
|---|---|---|---|
| core | `crates/core/` | domain models, field types, validation, config, cache, OIDC token validation, shared DTOs | (leaf) |
| license | `crates/license/` | license verification | core |
| oidc-http | `crates/oidc-http/` | OIDC discovery + JWKS fetching over HTTP | core |
| workflow | `crates/workflow/` | DSL engine, job queue | core |
| persistence | `crates/persistence/` | SQLx repositories, migration service | core, workflow |
| services | `crates/services/` | business logic, adapters, OIDC flow/provisioning/runtime, dry-run | core, license, oidc-http, persistence, workflow |
| worker | `crates/worker/` | background tasks, scheduler | core, persistence, services, workflow |
| mcp | `crates/mcp/` | MCP server, an HTTP client of the admin API | core, oidc-http, workflow |
| api | `crates/api/` | Actix endpoints, middleware, auth | core, license, persistence, services, workflow |
| test-support | `crates/test-support/` | shared test helpers, fixtures (dev-only) | core, persistence, services, workflow |

**Layering rule:** `core` imports no other workspace crate; nothing imports
`api` or `worker`; `mcp` never imports `api`, `persistence` or `services`.
Repository *traits* live in core/services; *implementations* in `persistence`.
`tests/architecture.rs` is the source of truth; update it and this table
together.

## Supporting docs (read on demand)

- Per crate: `core.md`, `services.md`, `persistence.md`, `api.md`, `workflow.md`, `worker.md`, `license.md`, `mcp.md`
- `database.md` — SQLx, migrations, compile-time verification, test DB
- `api-reference.md` — public + admin endpoint tables
- `conventions.md` — clippy policy, MSRV, file-length caps, allow policy
- `quality.md` — testing conventions, review standards

## Scoped-check commands

Subagents verify their own slice — never `rdt test` or full CI (CI clippy runs with `--all-features`; `rdt clippy` does not):

```bash
SQLX_OFFLINE=true cargo check -p <crate>
cargo +nightly clippy -p <crate> --all-targets --all-features -- -D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery
cargo test -p <crate> <module>
```

The `-p` flag takes the **package name** from `Cargo.toml`, not the directory name.
Package names: `r_data_core_core`, `r_data_core_api`, `r_data_core_persistence`,
`r_data_core_services`, `r_data_core_workflow`, `r_data_core_worker`,
`r_data_core_license`, `r_data_core_oidc_http`, `r_data_core_mcp`,
`r_data_core_test_support`.

## TypeScript bindings

`core-domain` owns `#[derive(TS)]` / `#[ts(...)]` on exported structs; `api`
owns the `export_bindings` tests. After changing exported structs run
`rdt generate-ts`. Never hand-edit `fe/src/types/generated/` (a PreToolUse
hook blocks it).
