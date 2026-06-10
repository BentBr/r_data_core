Audit the `.claude/` directory and produce a prioritized improvement report with an inline fix plan in plan mode.

Accept an optional argument: `fast` — if present, skip Phase 0. Example: `/analyze-claude fast`.

---

## Phase 0 — Load Official Docs (skip if `fast`)

Fetch current Claude Code docs via `context7` to ground the audit:
1. `mcp__context7__resolve-library-id` with query `"Claude Code"`
2. In parallel, `query-docs` on: `"CLAUDE.md rules skills difference"`, `"hooks PreToolUse exit codes"`, `"agents commands slash"`, `"token context window optimization"`.

Flag any doc finding that contradicts the current project config.

---

## Phase 1 — Collect

Read in parallel:
- `.claude/CLAUDE.md`
- `.claude/settings.json` and `.claude/settings.local.json`
- Every file under `.claude/hooks/`
- Every file under `.claude/agents/`
- Every file under `.claude/commands/`
- Every `SKILL.md` (and supporting `.md`) under `.claude/skills/<name>/`
- Top-level listing of `crates/` and `fe/`

---

## Phase 2 — Per-File Analysis

### A. Type correctness
- Reusable knowledge/workflow → `skill` at `.claude/skills/<name>/SKILL.md`
- How a subagent operates → `agent` in `.claude/agents/`
- User-invocable workflow → `command` in `.claude/commands/`
- ≤3 lines, or security-critical → inline in `CLAUDE.md` or a hook

### B. Duplication check
Compare each `SKILL.md` against `CLAUDE.md` and other skills: identical → keep most-scoped, delete dup; differ ≤5 lines → merge into canonical; differ >5 → flag for human decision.

### C. Compression
Flag files that could lose >20% of lines without losing distinct info (prose restating headings, redundant examples, stale infra/API descriptions).

### D. Security audit (hooks)

**`protect_secrets.js` checklist** — each failed item is Critical:
- [ ] file-path check uses `/\.env\b/` (word boundary, not bare `/\.env/`)
- [ ] a broader `sensitiveBashPatterns` array (incl. `/\.en[?*[]/`, `/\.e[?*[]/`) is used for Bash + Glob/Grep
- [ ] `Glob` (and `Grep`) is in the `settings.json` matcher; hook reads `tool_input.pattern`
- [ ] exit codes: `0` allow, `1` error, `2` block

**`protect_generated.cjs` checklist:**
- [ ] denies Write/Edit under `fe/src/types/generated/`, allows elsewhere
- [ ] wired in `settings.json` PreToolUse `Write|Edit` matcher

**Format hooks (`rustfmt`/`prettier`/`eslint`):** skip silently when the tool/Docker is unavailable; eslint blocks (exit 2) on real lint errors; prettier/rustfmt never block.

### E. Agent validation
Every `.claude/agents/*.md`: has `model`, has a real `description`, and is referenced by name in `CLAUDE.md` (else flag as orphan).

### F. Missing coverage
- Every `crates/<x>` is represented in `backend/SKILL.md`'s crate table.
- Every agent name in `CLAUDE.md` has a file in `.claude/agents/`.
- Every skill referenced in `CLAUDE.md`/agents exists under `.claude/skills/`.
- No command/agent/CLAUDE.md references a stale flat skill path (`skills/crates/`, `skills/conventions/`).

---

## Before Phase 3 — Enter Plan Mode

Call `EnterPlanMode` now so no files are written until the user reviews. Present the Phase 3 report inside plan mode — it IS the plan.

---

## Phase 3 — Report + Fix Plan

```
🟠 Claude Configuration Audit — {today's date}

### 🔴 Critical
### 🟡 Duplication
### 🟠 Type Mismatches
### 🟢 Compression Opportunities
### 🔵 Missing Coverage
### ⚪ Suggestions

---
🟠 Summary: X critical | Y duplicates | Z mismatches | W compression candidates

### Fix Plan
**Safe fixes** (non-destructive): per fix — File / Action / Reason.
**Conflicts requiring human decision:** per conflict — File / Issue / Options A|B.
```

Each finding: **File** / **Issue** (one sentence) / **Fix** (one sentence).
