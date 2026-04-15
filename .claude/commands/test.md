# Run Tests

Run tests for this project. Uses `rdt` (rusty_dev_tool) commands.

## Available test commands

- `rdt test` - Run all workspace tests (recommended for full test suite)
- `rdt test-unit` - Run unit tests only (faster)
- `rdt test-fe` - Run admin frontend tests (vitest via Docker)

## Instructions

1. If the user specifies a test type, run that specific test command
2. If no type specified, ask which tests to run or run `rdt test` for the full suite
3. For running a specific Rust test by name: `cargo test <test_name> --workspace`
4. If tests fail, analyze the output and suggest fixes

## Environment requirements

- Tests require PostgreSQL and Redis running via Docker
- Test database: `pg-test.rdatacore.docker:5433`
- The test framework creates/drops schemas automatically

## Example usage

```bash
# All tests
rdt test

# Unit tests only
rdt test-unit

# Specific test
cargo test test_entity_creation --workspace

# With SQLX offline mode (no db connection needed for compile)
SQLX_OFFLINE=true cargo test --workspace
```
