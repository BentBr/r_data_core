#!/usr/bin/env bash
# SQL-boundary guard.
#
# All SQL query issuance must live in the persistence layer
# (`crates/persistence/src/`). Other production crates call repository methods
# instead of issuing queries directly, keeping the data layer swappable and the
# layering enforceable. `crates/test-support/src/` is exempt: it sets up and
# tears down test schemas and fixtures.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Matches `sqlx::query*` calls and the `query!`/`query_as!`/`query_scalar!`/
# `query_file!` macros (with or without a leading identifier char). No `\b` so
# it works under both GNU and BSD grep.
pattern='sqlx::query|(^|[^A-Za-z0-9_])(query|query_as|query_scalar|query_file)!'

violations="$(grep -rnE "$pattern" crates src --include='*.rs' 2>/dev/null \
    | grep -vE 'crates/persistence/src/|crates/test-support/src/' || true)"

if [[ -n "$violations" ]]; then
    echo "::error::SQL query issuance found outside the persistence layer:"
    echo "$violations"
    echo
    echo "All SQL belongs in crates/persistence/src/. Call a repository method instead."
    exit 1
fi

echo "SQL boundary OK: no query issuance outside the persistence layer."
