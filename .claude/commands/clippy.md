# Run Clippy Lints

Run Rust clippy linter with the strict lints configured for this project.

## Command

```bash
rdt clippy
```

This runs clippy with strict settings:
```bash
cargo clippy --workspace --all-targets --all-features -- \
  -D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery
```

## Instructions

1. Run `rdt clippy` to check all code
2. If clippy finds issues, fix them before committing
3. For offline mode (no database): `SQLX_OFFLINE=true rdt clippy`

## Common clippy fixes

- Missing documentation: Add doc comments if public API
- Unnecessary clones: Use references where possible
- Match on Option/Result: Use combinators like `map`, `and_then`
- Redundant closures: Use function references directly
