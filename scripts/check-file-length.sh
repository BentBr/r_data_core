#!/usr/bin/env bash
# File-length policy guard.
#
# The cap applies to PRODUCTION code only: 500-line hard cap (error), 300-line
# soft cap (warning), counting lines BEFORE the first `#[cfg(test)]` module
# (tests live at the bottom of a file and don't count). Pure-test files
# (`tests.rs`, `*_tests.rs`, anything under a `tests/`/`*_tests/` module) and
# the `test-support` crate are test infrastructure and are exempt entirely.
set -uo pipefail

SOFT=300
HARD=500
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Production line count = lines before the first `#[cfg(test)]` attribute, or
# the whole file if there is none.
prod_lines() {
    awk '
        /^[[:space:]]*#\[cfg\(test\)\]/ { print NR - 1; found = 1; exit }
        END { if (!found) print NR }
    ' "$1"
}

hard=0
soft=0
while IFS= read -r file; do
    rel="${file#./}"

    # Skip test infrastructure entirely.
    case "$rel" in
        crates/test-support/*) continue ;;
        */tests.rs | *_tests.rs) continue ;;
        */tests/* | *_tests/*) continue ;;
    esac

    lines="$(prod_lines "$file")"

    if (( lines > HARD )); then
        echo "::error::$rel has $lines lines of non-test code (hard cap $HARD). Split it into focused modules."
        hard=$((hard + 1))
    elif (( lines > SOFT )); then
        echo "::warning::$rel has $lines lines of non-test code (soft cap $SOFT)."
        soft=$((soft + 1))
    fi
done < <(find crates src -name '*.rs' -type f 2>/dev/null | sort)

echo "File-length check: $soft over soft cap, $hard hard-cap violation(s)."
(( hard == 0 )) || exit 1
