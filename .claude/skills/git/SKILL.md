---
name: git
description: r_data_core git conventions — conventional commits + type/scope decision matrix, branch naming, the .githooks/pre-push quality pipeline and its GIT_HOOK_* toggles, commit-message style (NO Co-Authored-By). Read by /commit, /push, and conflict-resolver. See conflicts.md for merge-conflict classification.
color: yellow
---

# Git Conventions

## Conventional Commits

Format: `<type>[optional scope][!]: <description>`

Types: `feat`, `fix`, `test`, `docs`, `refactor`, `style`, `perf`, `build`, `ci`, `chore`

### Commit Type Decision Matrix

| What changed | Type |
|---|---|
| New feature/capability | `feat:` |
| Bug fix | `fix:` |
| Tests only | `test:` |
| Docs only | `docs:` |
| Refactor, no behavior change | `refactor:` |
| Formatting/lint only | `style:` |
| Performance | `perf:` |
| Build system / dependencies | `build:` |
| CI/CD config | `ci:` |
| Maintenance | `chore:` |
| Breaking change (any type) | `feat!:` / `BREAKING CHANGE:` |

### Commit messages

Imperative subject, under 72 chars. **Do not add `Co-Authored-By` lines.**
The summary states the *why*; the diff shows the *what*. No marketing prose
("comprehensive", "robust"), no enumerating every change, no session metrics.

## Pre-push Hook

Enabled via:
```bash
git config core.hooksPath .githooks
chmod +x .githooks/pre-push
```

The hook runs automatically before each push:
1. **Docker check** - Ensures required services (postgres, pg-test, redis, node) are running, starts them if not
2. **cargo fmt** - `cargo fmt --check --all`
3. **Clippy** - Strict linting (see the backend skill's `conventions.md`)
4. **Rust tests** - `rdt test`
5. **Frontend tests** - `rdt test-fe`
6. **ESLint** - `rdt lint`
7. **Conventional commits** - Validates commit message format

### Skipping

```bash
GIT_HOOK_SKIP=1 git push
```

### Disabling Individual Checks

Set in `.env.local`:
```bash
GIT_HOOK_RUN_FMT=0         # Disable cargo fmt check
GIT_HOOK_RUN_CLIPPY=0       # Disable Clippy
GIT_HOOK_RUN_TEST=0         # Disable Rust tests
GIT_HOOK_RUN_TEST_FE=0      # Disable frontend tests
GIT_HOOK_RUN_LINT=0         # Disable ESLint
GIT_HOOK_RUN_COMMIT_LINT=0  # Disable conventional commits check
```

## Security Hooks

`.claude/settings.json` blocks access to:
- `.env` files
- `.pem` and `.key` files
- `credentials` and `secrets` directories
