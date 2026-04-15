---
name: rust-conventions
description: Clippy policy, allow policy, MSRV, file length limits, and Rust coding standards for r_data_core
---

# Rust Conventions

## MSRV

Rust 1.92.0

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
