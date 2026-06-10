#!/usr/bin/env bash
# Workspace-wide `cargo llvm-cov` wrapper. Single source of truth for what we
# exclude from coverage measurement and the minimum threshold CI enforces.
# Used by `.github/workflows/coverage.yml` and (via `rdt coverage-be`) locally.
#
# Usage:
#   scripts/coverage.sh profile    # run tests with profiling (no report)
#   scripts/coverage.sh lcov       # emit coverage/rust.lcov
#   scripts/coverage.sh json       # emit coverage/rust.json
#   scripts/coverage.sh summary    # print human-readable per-file table
#   scripts/coverage.sh check      # fail if line coverage < threshold
#   scripts/coverage.sh all        # profile + lcov + json + summary + check
#   scripts/coverage.sh threshold  # print the threshold integer
#
# Adding a new "untestable wiring" file: extend the IGNORE regex below and
# document why.
#
# Note: the report steps use the no-subcommand form with `--no-run --workspace`
# rather than the `report` subcommand. This repo's root is BOTH a package and a
# workspace, so `cargo llvm-cov report` defaults to the root package only and
# misses every member crate. `--no-run --workspace` re-uses the profdata from
# the `profile` step and reports across the whole workspace.

set -euo pipefail

cd "$(dirname "$0")/.."

# Exclude binary entry points and runtime wiring that cannot be meaningfully
# unit-tested:
#   - utility/CLI binaries (crates/*/src/bin/*)
#   - application + worker entry points (src/main.rs, crates/worker/src/main.rs)
#   - application bootstrap/wiring (src/bootstrap.rs)
#   - worker scheduler/consumer/outbox runtime loops (crates/worker/src/runtime/):
#     they require a live Postgres + Redis + tick loop and are exercised by the
#     e2e suite, not unit tests.
#   - worker cron/task registrars (crates/worker/src/registrars/): pure wiring
#     that registers tasks against the scheduler; behaviour is covered via the
#     task implementations (crates/worker/src/tasks/, which stay measured).
IGNORE='(/src/bin/|/main\.rs$|(^|/)src/bootstrap\.rs$|crates/worker/src/(runtime|registrars)/)'

# Minimum line-coverage threshold (percent). CI fails the build below this.
MIN_LINES=60

# Integration tests spawn the server binary, whose instrumented runtime drops
# stray `default_*.profraw` files into the repo root (the spawned process's
# cwd). They are orphans — the report reads merged profdata under
# target/llvm-cov-target — so delete them to keep the working tree clean.
cleanup_profraw() {
    find . -maxdepth 1 -name '*.profraw' -delete 2>/dev/null || true
}

cmd="${1:-help}"

case "$cmd" in
    profile)
        cargo llvm-cov --workspace --no-report --ignore-filename-regex "$IGNORE"
        cleanup_profraw
        ;;
    lcov)
        mkdir -p coverage
        cargo llvm-cov --no-run --workspace --lcov --output-path coverage/rust.lcov --ignore-filename-regex "$IGNORE"
        ;;
    json)
        mkdir -p coverage
        cargo llvm-cov --no-run --workspace --json --output-path coverage/rust.json --ignore-filename-regex "$IGNORE"
        ;;
    summary)
        cargo llvm-cov --no-run --workspace --summary-only --ignore-filename-regex "$IGNORE"
        ;;
    check)
        cargo llvm-cov --no-run --workspace --fail-under-lines "$MIN_LINES" --ignore-filename-regex "$IGNORE"
        ;;
    all)
        "$0" profile
        "$0" lcov
        "$0" json
        "$0" summary
        "$0" check
        cleanup_profraw
        ;;
    threshold)
        echo "$MIN_LINES"
        ;;
    *)
        echo "usage: $0 {profile|lcov|json|summary|check|all|threshold}" >&2
        exit 1
        ;;
esac
