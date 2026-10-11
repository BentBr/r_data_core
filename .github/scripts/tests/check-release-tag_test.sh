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
git config commit.gpgsign false
git config tag.gpgsign false
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
printf '{\n  ".": "0.4.16"\n}\n' > .release-please-manifest.json
git commit -qam 'release 0.4.16'
git tag -a mcp-v0.4.16 -m 'long version'
git tag -a mcp-v0.4.1 -m 'prefix of manifest version'
printf '{\n  ".": "0.5.0"\n}\n' > .release-please-manifest.json
git commit -qam 'release 0.5.0'
git tag mcp-v0.5.0  # lightweight: no tag object to peel

pass=0; fail=0
# The run's own commit defaults to the tag's, as for a push of that tag; a
# fourth argument overrides it. GITHUB_SHA is unset so a CI runner's value
# cannot leak in.
expect() { # <expected-exit> <tag> <label> [expected-sha]
    local want="$1" tag="$2" label="$3" got=0 sha
    sha="${4:-$(git rev-parse --verify --quiet "refs/tags/${tag}^{commit}" || echo none)}"
    env -u GITHUB_SHA BASE_REF=main EXPECTED_SHA="$sha" bash "$script" "$tag" >/dev/null 2>&1 || got=$?
    if [ "$got" -eq "$want" ]; then pass=$((pass+1)); echo "ok   - $label"
    else fail=$((fail+1)); echo "FAIL - $label (exit $got, wanted $want)"; fi
}

expect 0 mcp-v1.2.3 'a tag on main whose manifest matches passes'
expect 1 mcp-v9.9.9 'a tag off main is refused'
expect 1 mcp-v1.2.5 'a tag whose manifest disagrees is refused'
expect 0 mcp-v0.4.16 'a tag equal to the manifest version passes'
expect 1 mcp-v0.4.1 'a tag that is a prefix of the manifest version is refused'
expect 1 mcp-v0.0.1 'a tag that does not exist is refused'
expect 0 mcp-v0.5.0 'a lightweight tag on main whose manifest matches passes'
expect 1 mcp-v1.2.3 'a run started for another commit is refused' "$(git rev-parse main)"
expect 1 'mcp-v1.2.3;id' 'a malformed tag is refused before anything else'

missing=0
env -u GITHUB_SHA -u EXPECTED_SHA BASE_REF=main bash "$script" mcp-v1.2.3 >/dev/null 2>&1 || missing=$?
if [ "$missing" -ne 0 ]; then pass=$((pass+1)); echo "ok   - a run without EXPECTED_SHA is refused"
else fail=$((fail+1)); echo "FAIL - a run without EXPECTED_SHA passed"; fi

want_sha="$(git rev-parse 'mcp-v1.2.3^{commit}')"
out="$(BASE_REF=main EXPECTED_SHA="$want_sha" bash "$script" mcp-v1.2.3 2>&1)"
case "$out" in *"$want_sha"*) pass=$((pass+1)); echo "ok   - success output names the tag's commit";;
    *) fail=$((fail+1)); echo "FAIL - success output lacks $want_sha: $out";; esac

echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
