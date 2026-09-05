---
name: backend-conventions
description: Clippy policy, allow policy, MSRV, file length limits, and Rust coding standards for r_data_core (supporting doc of the backend skill)
---

# Rust Conventions

## MSRV

Rust 1.96.0

## Clippy Policy

Enforced strictly:
```bash
cargo clippy --workspace --all-targets --all-features -- \
  -D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery
```

Run via: `rdt clippy`

### Allow Policy

- **Never** add `#[allow(clippy::...)]` without a justification comment explaining why the lint cannot be fixed.
- Fix the underlying code first; only suppress genuine false positives.

### Justified Exceptions (no comment needed)

| Lint | Context |
|------|---------|
| `future_not_send` | Actix handlers (take `HttpRequest`/`Multipart` which are `!Send`) |
| `float_cmp` | Exact zero-comparison division guards |
| `unused_async` | Trait impl methods and Actix handler signatures |
| `missing_const_for_fn` | Functions with non-const parameter types |

### Common Lint Fixes

| Lint | Fix |
|------|-----|
| `implicit_hasher` | Keep allow where generics are inappropriate (Actix extractors, internal fns) |
| `future_not_send` | Module-level allows in test files using `setup_test_app()` |
| `format_push_string` | `use std::fmt::Write; let _ = write!(query, ...)` |
| `cast_possible_truncation` | `i32::try_from(v).ok()` or `.unwrap_or(default)` |
| `cast_possible_wrap` | `i64::try_from(v).unwrap_or(0)` |
| `write_with_newline` | `writeln!` instead of `write!` with `\n` |
| `module_inception` | Rename inner `mod foo_tests` to `mod tests` |

## No Panics in Production Code

Every production crate root carries:

```rust
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]
```

So `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!` are compile errors outside
tests. Return a `Result`, use `unwrap_or`/`map_or_else`/`let ... else`, or — when
the call is genuinely infallible — add a narrowly scoped `#[allow(...)]` **with a
comment saying why**. `crates/test-support` is exempt (dev-only).

## Structural CI Guards

Four checks run in CI beyond clippy/fmt; run them locally before pushing:

| Guard | Command | Rule |
|-------|---------|------|
| SQL boundary | `./scripts/check-sql-boundary.sh` | `sqlx::query*` only in `crates/persistence/src` (and `test-support`) |
| File length | `./scripts/check-file-length.sh` | 300-line soft cap, 500-line hard cap |
| Layering | `cargo test --test architecture` | cargo-metadata dependency allowlist |
| MSRV | `cargo +1.96.0 check --workspace` | no post-1.96 APIs |

## File Length Limits

- **300 lines**: Soft cap — prefer splitting files that exceed this.
- **500 lines**: Hard cap — files must not exceed this without explicit justification.
- Applies to both production and test files.
- When splitting, ask the user before creating new modules.

## Formatting

```bash
cargo fmt --all          # Format
cargo fmt --check --all  # Check only
```
