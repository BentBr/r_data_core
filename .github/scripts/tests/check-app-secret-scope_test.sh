#!/usr/bin/env bash
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/../check-app-secret-scope.sh"
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT

mkdir -p "$work/ok" "$work/bad"
printf 'private-key: ${{ secrets.RELEASE_APP_PRIVATE_KEY }}\n' > "$work/ok/release-please.yml"
printf 'private-key: ${{ secrets.PR_APP_PRIVATE_KEY }}\n' > "$work/ok/pr-management.yml"
cp "$work/ok/"* "$work/bad/"
printf 'private-key: ${{ secrets.RELEASE_APP_PRIVATE_KEY }}\n' > "$work/bad/pr-management.yml"

pass=0; fail=0
if bash "$script" "$work/ok" >/dev/null 2>&1; then pass=$((pass+1)); echo "ok   - confined secret passes"; else fail=$((fail+1)); echo "FAIL - confined secret should pass"; fi
if bash "$script" "$work/bad" >/dev/null 2>&1; then fail=$((fail+1)); echo "FAIL - leaked secret should fail"; else pass=$((pass+1)); echo "ok   - leaked secret fails"; fi
echo "$pass passed, $fail failed"; [ "$fail" -eq 0 ]
