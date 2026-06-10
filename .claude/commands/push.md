---
description: Push the current branch, drive the .githooks/pre-push quality pipeline, route any failures to the right agent, and monitor every PR the hook creates. Use after /commit when ready to publish.
---

Read `.claude/skills/git/SKILL.md`, then push the current branch and handle the pre-push hook:

1. **Preflight** —
   - If `git status --porcelain` is non-empty, warn about uncommitted changes (which won't be pushed) and confirm before continuing.
   - List commits to publish: `git log @{u}..HEAD --oneline` (or `git log main..HEAD --oneline` if no upstream). If nothing to publish, stop and report.

2. **Push** — Run `git push`. The pre-push hook (`.githooks/pre-push`) runs: Docker check → `cargo fmt --check` → clippy → `rdt test` → `rdt test-fe` → eslint → conventional-commit lint.

3. **Handle hook failure** — read the output and route by failure type:

   - **Merge conflict** — delegate to the **conflict-resolver** agent. Pass it `git status --short` and `git log -1 --oneline MERGE_HEAD` (or `REBASE_HEAD`). Present its proposal via `EnterPlanMode` for review before it applies anything. After it reports a clean, staged tree, `git commit` (keep the prepared merge message) and **re-invoke `/push`**. Conventions: `.claude/skills/git/conflicts.md`.

   - **Quality-check failure** — route by location:
     | Failure | Agent |
     |---|---|
     | fmt/clippy in `crates/core` | core-domain |
     | clippy in `crates/services\|workflow\|worker` | services |
     | clippy in `crates/persistence` (or migration issue) | persistence |
     | clippy in `crates/api` | api |
     | Rust test failure | rust-tester (or the owning layer if it's a production bug) |
     | eslint/vue-tsc in `fe/src` components/SCSS | frontend-ui |
     | eslint/vue-tsc in stores/composables/services | frontend-state |
     | eslint/vue-tsc in router/plugins/types/schemas | frontend-infrastructure |
     | `rdt generate-ts-check` drift | api (regenerate via `rdt generate-ts`) |
     | CI/hook/Docker/env step | devops |

     Summarize what failed, propose a fix, wait for confirmation, delegate to the agent, then **re-invoke `/push`**.

4. **Find every PR the hook created** (the hook may sync derived branches and open more than one):
   ```bash
   gh pr list --head <branch> --json number,url,title,baseRefName
   ```

5. **Arm one Monitor per PR** (each PR has its own check set):
   ```
   Monitor:
     description: "PR #<num> (<base>) check completions"
     timeout_ms: 3600000
     persistent: false
     command: |
       prev=""
       while true; do
         s=$(gh pr checks <num> --json name,bucket 2>/dev/null) || { sleep 30; continue; }
         cur=$(jq -r '.[] | select(.bucket!="pending") | "\(.name): \(.bucket)"' <<<"$s" | sort)
         comm -13 <(echo "$prev") <(echo "$cur")
         prev=$cur
         jq -e 'length>0 and all(.[]; .bucket!="pending")' <<<"$s" >/dev/null && break
         sleep 30
       done
       echo "PR #<num> checks complete"
   ```
   Do not poll or `gh pr checks --watch` while a Monitor is armed — events arrive asynchronously. Continue with other work meanwhile.

6. **On a failed check** — summarize (including which PR) and ask whether to fix. If yes, delegate by location using step 3's routing table.

Never pass `GIT_HOOK_SKIP=1` (or set any `GIT_HOOK_RUN_*=0`) unless the user explicitly asks.
