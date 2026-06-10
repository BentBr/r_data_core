---
description: Validate the current change before pushing — runs the QA pipeline defined in .claude/skills/quality-assurance/SKILL.md (static gate, ts-binding check, agent delegation, docs/skills sync).
argument-hint: "[all]"
---

Read `.claude/skills/quality-assurance/SKILL.md` and execute **"The /qa pipeline"**.

Mode argument: `$ARGUMENTS`

- empty → **delta mode** (default): scope is uncommitted changes + commits ahead of upstream.
- `all` → **branch mode**: scope is `git diff main..HEAD`.

The SKILL is the single source of truth for what `/qa` runs and when. Do not duplicate its logic here.
