## Description

<!-- What changed and why? Link related issues with "Closes #123" or "Relates to #123". -->

## Type of Change

- [ ] `feat` — New feature
- [ ] `fix` — Bug fix
- [ ] `refactor` — Code restructuring (no behaviour change)
- [ ] `test` — Adding or updating tests
- [ ] `docs` — Documentation only
- [ ] `chore` — dependencies, formatting, style adherence
- [ ] `ci` — Build, CI, tooling
- [ ] `perf` — Performance improvement

## Pre-Push Checks

The [pre-push hook](.githooks/pre-push) enforces these automatically. Confirm they passed locally:

- [ ] `cargo fmt --all --check` — No formatting issues
- [ ] `rdt clippy` — No clippy warnings
- [ ] `rdt test` — All Rust tests pass
- [ ] `rdt test-fe` — Frontend tests pass
- [ ] `rdt lint` — ESLint + Prettier pass
- [ ] `rdt generate-ts-check` — TS bindings are up to date
- [ ] Commits follow [Conventional Commits](https://www.conventionalcommits.org/)

## Testing

<!-- How was this tested? What scenarios were covered? -->

## Notes for Reviewers

<!-- Anything reviewers should pay attention to, or context that helps the review. -->
