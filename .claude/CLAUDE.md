# r_data_core

A Rust-based Master Data Management (MDM) system. Modular workspace with Actix-web API, SQLx persistence, DSL-based workflows, and a Vue3 admin frontend.

## Quick Reference

This project uses `rdt` (rusty_dev_tool) as the standard task runner. **Always prefer `rdt` commands** over raw `cargo`/`npm` invocations. **Never run `npm` locally** — all frontend commands run inside the Docker `node` container.

```bash
rdt test            # Run all workspace tests
rdt test-unit       # Unit tests only
rdt test-fe         # Admin frontend vitest (Docker)
rdt clippy          # Clippy with strict lints (nightly)
rdt lint            # ESLint + Prettier for frontend (Docker)
rdt test-e2e        # Playwright E2E tests (Docker)
rdt clean-e2e       # Remove E2E test data from DB
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

## Workspace Crates

| Crate | Path | Role | Skill |
|-------|------|------|-------|
| core | `crates/core/` | Domain models, config, cache, field types | `crates/core` |
| api | `crates/api/` | Actix-web HTTP endpoints, middleware, auth | `crates/api` |
| persistence | `crates/persistence/` | SQLx repositories | `crates/persistence` |
| services | `crates/services/` | Business logic, adapters | `crates/services` |
| workflow | `crates/workflow/` | DSL engine, job queue | `crates/workflow` |
| worker | `crates/worker/` | Background tasks, scheduler | `crates/worker` |
| license | `crates/license/` | License verification | `crates/license` |
| test-support | `crates/test-support/` | Test helpers (dev-only) | `crates/test-support` |

Frontend: `fe/` — Vue3 + TypeScript + Vuetify admin dashboard. See `crates/frontend` skill.

## Skills Reference

Detailed documentation is organized into skills under `.claude/skills/`:

### Conventions
- **`conventions/git`** — Git hooks, conventional commits, hook customization
- **`conventions/rust-conventions`** — Clippy policy, allow policy, MSRV, file length limits
- **`conventions/frontend-conventions`** — Vue3, TypeScript, Vite, Vuetify patterns
- **`conventions/code-quality`** — Quality standards, testing conventions, pre-push checks

### Architecture
- **`architecture/workspace`** — Crate dependency graph, layering rules, key concepts
- **`architecture/api-reference`** — Full public and admin API endpoint tables
- **`architecture/database`** — SQLx, migrations, compile-time verification, test DB

### Per-Crate Documentation
Each crate has a dedicated skill under `crates/` with module breakdowns, key exports, and patterns.

## Code Quality

- Clippy enforced strictly: `-D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery`
- MSRV: 1.92.0
- File length: 300 lines soft cap, 500 lines hard cap
- See `conventions/rust-conventions` skill for full policy

## Security Hooks

`.claude/settings.json` blocks access to `.env`, `.pem`, `.key`, `credentials/`, `secrets/` directories.
