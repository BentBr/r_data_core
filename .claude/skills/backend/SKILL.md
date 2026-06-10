---
name: backend
description: Rust backend for r_data_core — crate dependency graph (core → services → persistence → api + workflow/worker), layering rules, per-crate maps, conventions, and scoped-check commands. Read by all backend subagents.
color: green
---

# Backend (Rust workspace)

## Crate dependency graph & layering

| Crate | Path | Role | May depend on |
|---|---|---|---|
| core | `crates/core/` | domain models, field types, validation, config, cache | (foundational) |
| services | `crates/services/` | business logic, adapters | core |
| persistence | `crates/persistence/` | SQLx repositories, migration service | core, services traits |
| api | `crates/api/` | Actix endpoints, middleware, auth | all layers |
| workflow | `crates/workflow/` | DSL engine, job queue | core, services |
| worker | `crates/worker/` | background tasks, scheduler | core, services, workflow |
| license | `crates/license/` | license verification | core |
| test-support | `crates/test-support/` | shared test helpers, fixtures (dev-only) | — |

**Layering rule:** `core` imports no other workspace crate. Repository
*traits* live in core/services; *implementations* live in `persistence`.

## Supporting docs (read on demand)

- Per crate: `core.md`, `services.md`, `persistence.md`, `api.md`, `workflow.md`, `worker.md`, `license.md`
- `database.md` — SQLx, migrations, compile-time verification, test DB
- `api-reference.md` — public + admin endpoint tables
- `conventions.md` — clippy policy, MSRV, file-length caps, allow policy
- `quality.md` — testing conventions, review standards

## Scoped-check commands

Subagents verify their own slice — never `rdt test` or full CI:

```bash
SQLX_OFFLINE=true cargo check -p <crate>
cargo +nightly clippy -p <crate> --all-targets -- -D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery
cargo test -p <crate> <module>
```

The `-p` flag takes the **package name** from `Cargo.toml`, not the directory name.
Package names: `r_data_core_core`, `r_data_core_api`, `r_data_core_persistence`,
`r_data_core_services`, `r_data_core_workflow`, `r_data_core_worker`,
`r_data_core_license`, `r_data_core_test_support`.

## TypeScript bindings

`core-domain` owns `#[derive(TS)]` / `#[ts(...)]` on exported structs; `api`
owns the `export_bindings` tests. After changing exported structs run
`rdt generate-ts`. Never hand-edit `fe/src/types/generated/` (a PreToolUse
hook blocks it).
