#!/usr/bin/env bash
# Exercises check-release-tag.sh against a throwaway repository.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/../check-release-tag.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cd "$work"
git init -q -b main
git config user.email t@example.com
git config user.name t
printf '{\n  ".": "1.2.3"\n}\n' > .release-please-manifest.json
git add . && git commit -qm 'release 1.2.3'
git tag -a mcp-v1.2.3 -m 'on main'

git checkout -q -b side
printf '{\n  ".": "9.9.9"\n}\n' > .release-please-manifest.json
git commit -qam 'side branch'
git tag -a mcp-v9.9.9 -m 'off main'
git checkout -q main
printf '{\n  ".": "1.2.4"\n}\n' > .release-please-manifest.json
git commit -qam 'bump manifest only'
git tag -a mcp-v1.2.5 -m 'manifest mismatch'

pass=0; fail=0
expect() { # <expected-exit> <tag> <label>
    local want="$1" tag="$2" label="$3" got=0
    BASE_REF=main bash "$script" "$tag" >/dev/null 2>&1 || got=$?
    if [ "$got" -eq "$want" ]; then pass=$((pass+1)); echo "ok   - $label"
    else fail=$((fail+1)); echo "FAIL - $label (exit $got, wanted $want)"; fi
}

expect 0 mcp-v1.2.3 'a tag on main whose manifest matches passes'
expect 1 mcp-v9.9.9 'a tag off main is refused'
expect 1 mcp-v1.2.5 'a tag whose manifest disagrees is refused'
expect 1 mcp-v0.0.1 'a tag that does not exist is refused'

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
