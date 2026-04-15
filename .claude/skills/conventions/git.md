---
name: git
description: Git hooks, conventional commits, branch naming, and hook customization for r_data_core
---

# Git Conventions

## Conventional Commits

Format: `<type>[optional scope][!]: <description>`

Types: `feat`, `fix`, `test`, `docs`, `refactor`, `style`, `perf`, `build`, `ci`, `chore`

## Pre-push Hook

Enabled via:
```bash
git config core.hooksPath .githooks
chmod +x .githooks/pre-push
```

The hook runs automatically before each push:
1. **Docker check** - Ensures required services (postgres, pg-test, redis, node) are running, starts them if not
2. **cargo fmt** - `cargo fmt --check --all`
3. **Clippy** - Strict linting (see `rust-conventions` skill)
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
