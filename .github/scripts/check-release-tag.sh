#!/usr/bin/env bash
# Refuse to release a tag that is not a release commit on main.
#
# mcp-release.yml builds and publishes whatever commit an mcp-v* tag points
# at. This check stops a mistaken or misdirected tag — one pushed to a branch
# commit, a version main never released, or a run started from a ref other
# than the tag — from shipping under a release version:
#   - the tagged commit must already be on the base branch,
#   - the release-please manifest at that commit must name the tag's version,
#   - the tagged commit must be the commit the workflow run was started for.
# The manifest proves the version is the one released from main at that
# commit; it does not prove the commit is release-please's own.
#
# It is not the boundary against a malicious tag: it constrains which commit
# is published, not who may start a release. That boundary is the tag ruleset (only the release App may create mcp-v* tags), the App key
# being an environment secret readable only from main, and the `release`
# environment that the publish jobs run in being restricted to mcp-v* tags
# with required reviewers.
#
# Usage: check-release-tag.sh <tag>
# Env:   EXPECTED_SHA  (required) the commit the run was started for —
#                      github.sha in the workflow
#        BASE_REF      (default origin/main)
#        MANIFEST      (default .release-please-manifest.json)
set -euo pipefail

tag="${1:?usage: check-release-tag.sh <tag>}"
expected_sha="${EXPECTED_SHA:?EXPECTED_SHA must name the commit this run was started for}"
base_ref="${BASE_REF:-origin/main}"
manifest="${MANIFEST:-.release-please-manifest.json}"

if ! printf '%s' "$tag" | grep -Eq '^mcp-v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'; then
    echo "::error::refusing '$tag': expected mcp-vX.Y.Z" >&2
    exit 1
fi

if ! tag_sha="$(git rev-parse --verify --quiet "refs/tags/${tag}^{commit}")"; then
    echo "::error::tag '$tag' does not exist in this checkout" >&2
    exit 1
fi

# A dispatch started from a branch, or a tag moved between the push and this
# checkout, would otherwise build one commit while the run's provenance names
# another.
if [ "$tag_sha" != "$expected_sha" ]; then
    echo "::error::refusing '$tag': it points at $tag_sha but this run was started for $expected_sha. Dispatch from the tag ref itself." >&2
    exit 1
fi

if ! git merge-base --is-ancestor "$tag_sha" "$base_ref"; then
    echo "::error::refusing '$tag': commit $tag_sha is not on $base_ref" >&2
    exit 1
fi

manifest_version="$(git show "${tag_sha}:${manifest}" | python3 -c 'import json,sys; print(json.load(sys.stdin)["."])')"
if [ "$manifest_version" != "${tag#mcp-v}" ]; then
    echo "::error::refusing '$tag': $manifest at $tag_sha says $manifest_version, not ${tag#mcp-v}" >&2
    exit 1
fi

echo "$tag -> $tag_sha is a release commit on $base_ref (manifest $manifest_version)"
