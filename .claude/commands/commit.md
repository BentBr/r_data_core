---
description: Review the working tree, propose a conventional commit message, and commit (no push). Use when the user wants to record changes locally before deciding when to push.
argument-hint: "[all]"
---

Read `.claude/skills/git/SKILL.md`, then perform the commit workflow (no push).

**Mode** — argument: `$ARGUMENTS`
- `all` → **commit-all mode**: stage everything in the working tree, skip step 2.
- otherwise → **default mode**: separate the commit set from unrelated WIP per step 2.

---

1. **Inspect** — Run `git status` and `git diff HEAD`. If the tree is clean, stop and report "nothing to commit".

2. **Separate the commit set from unrelated WIP** *(skipped in commit-all mode)* — Working trees often mix the current work with stale edits, generated lockfiles (`Cargo.lock`, `fe/pnpm-lock.yaml`), regenerated `.sqlx/` or `fe/src/types/generated/`, or experiments. Identify each modified file's purpose. List anything unrelated and ask whether to include it. Default: stage only files matching the current intent; leave unrelated WIP unstaged.

3. **Review (conditional)** — For non-trivial diffs, delegate a code review to the **quality-assurance** agent with the staged diff and ask it to flag issues before committing. **Skip** for trivial commits (pure docs/comments, single-line typo, formatting-only, dependency bumps). If issues are found, present them and ask whether to fix first.

4. **Type + scope** — Pick the conventional `type(scope):` from the staged changes (decision matrix in the git skill). If the staged set spans multiple types, pause and propose splitting into N atomic commits — let the user choose.

5. **Stage** — commit-all mode: `git add -A`. Default mode: `git add` only the files chosen in step 2.

6. **Propose the commits as a numbered Markdown block**, then wait for confirmation. No `---` separators — each commit starts with a bold number prefix.

   Each commit block has three parts:
   1. **Numbered subject line** — `**N.**` then a space, then the conventional-commit subject in inline backticks, e.g. `` **1.** `feat(api): add entity export endpoint` ``.
   2. **Files** — blank line, `**Files:**`, then one file per line indented two spaces, prefixed with the status letter (M/A/D/R). Collapse with `…and K more` past 6 entries.
   3. **Summary** — blank line, `**Summary:**`, then concise prose explaining the **why**, hard-wrapped at ~65 chars, max ~6 lines. No marketing prose, no enumerating every change, no session metrics. **No `Co-Authored-By` line.**

   End with: **Confirm to commit, or edit?**

7. **Commit** to the current branch. **Do not push.** After a successful commit, suggest `/push` to publish — never push automatically.
