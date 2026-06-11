---
name: git-conflicts
description: Conflict-classification table for r_data_core — Cargo.lock, migrations, generated TS, .sqlx, pnpm-lock, and code-file resolution strategies. Read by conflict-resolver and /push.
---

# Merge-conflict classification (r_data_core)

Read both sides before applying any strategy — never blind `--ours`/`--theirs`
on code files. Resolve manifests + code first, lockfiles last.

| Pattern | Strategy |
|---|---|
| `Cargo.lock`, `fe/pnpm-lock.yaml` | Never hand-merge. Resolve the manifest first, take one side's lock, then `cargo update -p <pkg>` / `pnpm install <pkg>` for **only the changed packages**. |
| `Cargo.toml`, `fe/package.json` | Union the dependency entries; surface real version-constraint conflicts — never silently pick one. |
| `migrations/*.sql` | Keep both sides' migrations; rename on timestamp collision; never hand-merge a migration body. |
| `fe/src/types/generated/**` | Never hand-merge — take one side + `rdt generate-ts`. |
| `.sqlx/**` | Never hand-merge — take one side + `cargo sqlx prepare --workspace -- --all-targets`. |
| `.env*` | Combine variables from both sides; preserve comments/order. Hook-blocked for Edit/Write — surface to user to apply. |
| `*.rs`, `*.vue`, `*.ts` | Semantic merge — read both intents, produce a resolution preserving both. Surface colliding intents with a concrete proposal. |
| `.claude/skills/**`, docs | Union + re-sort; delegate to skills-maintainer / docs-writer if reconciliation needs domain knowledge. |
| Binary (`*.png`, `*.pdf`) | Cannot textually merge — surface with sizes/hashes + a recommendation. |

## Verify before handing back

```bash
git diff --check                                   # no remaining markers
git status --short                                 # no unmerged paths
SQLX_OFFLINE=true cargo check --workspace          # if any .rs touched
docker compose exec -T node pnpm exec vue-tsc --noEmit   # if any .ts/.vue touched
```

Never commit, push, abort, or continue the merge/rebase on your own — leave the
resolved, staged tree for the orchestrator.
