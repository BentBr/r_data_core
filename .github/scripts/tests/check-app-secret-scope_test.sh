#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/../check-app-secret-scope.sh"
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT

mkdir -p "$work/ok" "$work/bad" "$work/noenv" "$work/otherjob"

# A job that declares the environment and reads the key, after one that
# does neither.
cat > "$work/ok/release-please.yml" <<'EOF'
jobs:
    release-please:
        runs-on: ubuntu-latest
        environment: something-else
    mcp-tag:
        runs-on: ubuntu-latest
        environment: release-tag
        steps:
            -   with:
                    private-key: ${{ secrets.RELEASE_APP_PRIVATE_KEY }}
EOF
printf 'private-key: ${{ secrets.PR_APP_PRIVATE_KEY }}\n' > "$work/ok/pr-management.yml"

cp "$work/ok/"* "$work/bad/"
printf 'private-key: ${{ secrets.RELEASE_APP_PRIVATE_KEY }}\n' > "$work/bad/pr-management.yml"

# The allowed file, but the job reading the key has no environment.
cp "$work/ok/"* "$work/noenv/"
cat > "$work/noenv/release-please.yml" <<'EOF'
jobs:
    mcp-tag:
        runs-on: ubuntu-latest
        steps:
            -   with:
                    private-key: ${{ secrets.RELEASE_APP_PRIVATE_KEY }}
EOF

# The environment is declared on a different job than the one reading the key.
cp "$work/ok/"* "$work/otherjob/"
cat > "$work/otherjob/release-please.yml" <<'EOF'
jobs:
    mcp-tag:
        runs-on: ubuntu-latest
        environment: release-tag
    leaky:
        runs-on: ubuntu-latest
        steps:
            -   with:
                    private-key: ${{ secrets.RELEASE_APP_PRIVATE_KEY }}
EOF

pass=0; fail=0
expect() { # <expected-exit> <dir> <label>
    local want="$1" dir="$2" label="$3" got=0
    bash "$script" "$dir" >/dev/null 2>&1 || got=$?
    if [ "$got" -eq "$want" ]; then pass=$((pass+1)); echo "ok   - $label"
    else fail=$((fail+1)); echo "FAIL - $label (exit $got, wanted $want)"; fi
}

expect 0 "$work/ok" 'confined secret read from the release-tag environment passes'
expect 1 "$work/bad" 'leaked secret fails'
expect 1 "$work/noenv" 'secret read by a job without the environment fails'
expect 1 "$work/otherjob" 'environment on another job does not cover the reader'

echo "$pass passed, $fail failed"; [ "$fail" -eq 0 ]
