---
name: code-quality
description: General code quality standards, testing conventions, and quality checks for r_data_core
---

# Code Quality

## Quality Check Commands

This project uses `rdt` (rusty_dev_tool) as the standard task runner. **Always prefer `rdt` commands** over raw `cargo`/`npm` invocations.

```bash
rdt test            # All workspace tests (recommended)
rdt test-unit       # Unit tests only
rdt test-fe         # Admin frontend vitest (runs in Docker)
rdt clippy          # Clippy with strict lints (nightly)
rdt lint            # ESLint + Prettier for admin frontend (runs in Docker)
rdt test-e2e        # Playwright E2E browser tests (runs in Docker)
rdt clean-e2e       # Remove E2E test data from local database
```

**Never run `npm` locally.** All frontend commands run inside the Docker `node` container via `rdt` or `docker compose exec node`.

`cargo test --workspace` also works but `rdt test` is recommended.

## Test Structure

- **Unit tests**: Within `src/` files in each crate (inline `#[cfg(test)]` modules)
- **Integration tests**: `/tests/` directory at workspace root
- Run tests by name without `test_` prefix

## Test Organization (Integration)

```
tests/
├── adapters/          # Adapter pattern tests
├── api/               # API endpoint tests (20+ files)
├── cache/             # Cache layer tests
├── dsl/               # DSL integration tests
├── license/           # License verification tests
├── statistics/        # Statistics tests
├── integration/       # Cross-layer integration tests
└── *.rs               # Individual feature tests
```

## Pre-push Checks

The pre-push hook runs all quality checks automatically. See the `git` skill for details.

## General Standards

- Prefer descriptive names over comments; only comment non-obvious logic
- Write the simplest code that solves the problem
- Validate uncertain types at runtime
- See `rust-conventions` skill for Clippy and file length rules
