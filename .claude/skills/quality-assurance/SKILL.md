---
name: quality-assurance
description: Drives /qa. The full QA pipeline for r_data_core — scope identification, static gate (rdt clippy + rdt test), ts-binding sync check (rdt generate-ts-check), agent delegation/routing, review checklist, docs + skills sync. Read by the /qa orchestrator and the quality-assurance subagent.
color: yellow
---

# Quality Assurance

Single source of truth for "is this change ready to ship?" Read by two readers:
the **/qa orchestrator** (runs the pipeline) and the **quality-assurance
subagent** (runs the slice the static gate can't cover, then reports).

## The /qa pipeline

**Mode** — the slash command passes its argument through:

- `all` → **branch mode**: scope is `git diff main..HEAD`.
- empty → **delta mode** (default): scope is `git status` + `git diff @{u}..HEAD`.

### Step 1 — Identify scope (compute once, reuse)

Classify the changed-file set: which crate(s) touched, fe touched, migrations
touched, generated-ts touched, docs/skills touched.

### Step 2 — Static gate (orchestrator runs)

- `rdt clippy` — strict nightly lints.
- `rdt test` (branch mode) or `rdt test-unit` (delta mode).
- if fe touched: `rdt test-fe` + `rdt lint`.
- if exported Rust structs touched: `rdt generate-ts-check` (fails if generated
  files drift from the structs).

### Step 3 — Delegate

Hand the slice the static gate can't cover to the quality-assurance subagent:
coverage gaps, the review checklist, docs/skills sync. The subagent runs
**scoped** commands only — never the full `rdt test`/CI (the orchestrator owns that).

### Step 4 — Synthesize & route

Collect findings; route each to the owning agent.

## Routing table (finding location → agent)

| Finding in… | Route to |
|---|---|
| `crates/core/` | core-domain |
| `crates/services`, `crates/workflow`, `crates/worker` | services |
| `crates/persistence/`, `migrations/` | persistence |
| `crates/api/` | api |
| missing/failing Rust tests | rust-tester |
| `fe/src` components / SCSS | frontend-ui |
| Pinia stores / composables / services | frontend-state |
| router / plugins / dto / zod schemas | frontend-infrastructure |
| `*.test.ts` gaps | frontend-tester |
| `fe/translations/*` parity | i18n-translator |
| Docker / CI / rdt / env | devops |
| `docs/` drift | docs-writer |
| `.claude/skills/` drift | skills-maintainer |

## Review checklist

- Layering respected (`core` imports nothing upstream; repo traits vs impls split correctly).
- Clippy clean under pedantic + nursery; no new `#[allow]` without justification (see `backend/conventions.md`).
- File-length caps (300 soft / 500 hard).
- Tests cover negative paths (auth failure, missing fields, empty collections, boundary values).
- Exported structs changed → bindings regenerated (`rdt generate-ts`); generated dir not hand-edited.
- Migrations append-only; `.sqlx/` regenerated after schema changes.
- Docs + skill files in sync with new entities / endpoints / roles.

## Subagent output contract

Report-first, sectioned, PASS/FAIL per check, route each finding via the table
above. Truncated or narration-only responses are a failure mode. Never run the
full `rdt test`/CI — the orchestrator already ran the static gate.
