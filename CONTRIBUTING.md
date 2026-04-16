# Contributing to r_data_core

Thank you for considering a contribution. This document outlines the standards and expectations for all code entering this project.

## Code of Conduct

All contributors must follow our [Code of Conduct](CODE_OF_CONDUCT.md). Be respectful, constructive, and professional.

## Clean Code Standards

This project prioritises **readable, maintainable, and well-structured code**. Every contribution is expected to meet these standards:

- **Meaningful names** -- variables, functions, and types must clearly express their intent. Avoid abbreviations and single-letter names outside short closures or iterators.
- **Small, focused functions** -- each function should do one thing. If you need a comment to explain _what_ a block does, extract it into a named function instead.
- **No dead code** -- do not commit commented-out code, unused imports, or unreachable branches. Remove what is not needed.
- **DRY within reason** -- eliminate true duplication, but do not over-abstract. Three similar lines are better than a premature abstraction.
- **Minimal surface area** -- keep types, fields, and functions private by default. Only make things public when there is a concrete consumer.
- **Follow existing patterns** -- read surrounding code before writing new code. Match the conventions already established in the module you are changing.

### Rust-Specific

- Clippy runs with strict lints (`-D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery`). Your code must pass clippy without suppression unless a documented exception applies.
- Run `cargo fmt --all` before submitting.
- Keep files under 300 lines (soft cap) and never exceed 500 lines (hard cap).

### Frontend-Specific

- Follow the TypeScript and Vue3 conventions documented in `.claude/skills/conventions/frontend-conventions`.
- Linting runs via `rdt lint`. All code must pass ESLint and Prettier checks.

## Test-Driven Development

Tests are not optional. This project follows a test-driven approach:

1. **Bug fixes must include a failing test first.** Before you fix a bug, write a test that reproduces it. The test must fail without your fix and pass with it. A bug fix without a corresponding test will not be accepted.
2. **New features must be covered by tests.** Write tests that validate the expected behaviour before or alongside your implementation.
3. **Do not reduce coverage.** Your contribution should maintain or improve the existing test coverage.
4. **Tests must be deterministic.** No flaky tests, no timing-dependent assertions, no reliance on external services in unit tests.

### Running Tests

This project uses [`rdt` (rusty_dev_tool)](https://github.com/BentBr/rusty_dev_tool) as its task runner. Install it before running any commands below.

```bash
rdt test          # All workspace tests
rdt test-unit     # Unit tests only
rdt test-fe       # Frontend vitest (Docker)
rdt test-e2e      # Playwright E2E tests (Docker)
```

## AI Tools Policy

AI-assisted tools (LLMs, code generators, copilots) **are allowed** for contributions to this project, under the following conditions:

- **You are fully responsible for every line you submit.** AI-generated code is treated identically to human-written code. There is no reduced standard, no special label, and no excuses. If it has your name on the commit, it is your code.
- **Review everything.** Do not blindly accept AI-generated suggestions. Read, understand, and verify every change before committing.
- **AI output must meet all project standards.** This includes clean code, passing clippy, passing tests, and following the conventions documented here. "The AI wrote it" is not a valid justification for substandard code.
- **Security is your responsibility.** Verify that AI-generated code does not introduce vulnerabilities, leak secrets, or bypass safety checks.

## Submitting a Contribution

### Issues

- Check existing issues before opening a new one.
- Bug reports must include steps to reproduce, expected behaviour, and actual behaviour.
- Feature requests should describe the problem being solved, not just the desired solution.

### Pull Requests

1. Fork the repository and create a feature branch from `main`.
2. Make your changes, following the standards above.
3. Ensure all checks pass locally:
   ```bash
   cargo fmt --all --check
   rdt clippy
   rdt test
   rdt generate-ts-check    # if you changed exported Rust structs
   rdt test-fe
   rdt lint
   ```
4. Write a clear PR description explaining _what_ changed and _why_.
5. Keep PRs focused. One logical change per PR. Large refactors should be split into reviewable chunks.

### Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add user role validation endpoint
fix: prevent duplicate entity creation on concurrent requests
refactor: extract shared pagination logic into helper
test: add integration tests for workflow DSL parser
docs: update API reference for admin endpoints
chore: clippy formatting
```

### Review Process

- All PRs require review before merging.
- Commits must be [signed](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits)
- Reviewers will check for correctness, test coverage, clean code, and adherence to project conventions.
- Address review feedback or explain why you disagree. Constructive discussion is welcome.

## Getting Help

- Read the [readme](README.md) and checkout existing [docs](docs)
- Read the project documentation in `.claude/skills/` for detailed architecture and convention guides.
- Open an issue if you are unsure about the right approach for a change.
- Ask questions in PR discussions -- there are no stupid questions.
- Be nice to each other :)
