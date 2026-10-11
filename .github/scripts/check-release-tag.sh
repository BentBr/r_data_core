#!/usr/bin/env bash
# Refuse to release a tag that is not a release commit on main.
#
# mcp-release.yml builds and publishes whatever commit an mcp-v* tag points
# at. Anyone who can push a tag could otherwise ship a branch's code under a
# release version — with valid provenance, since the workflow itself is the
# publisher. Two checks close that: the tagged commit must already be on the
# base branch, and the release-please manifest at that commit must name the
# tag's version. The manifest proves the version is the one released from main
# at that commit; it does not prove the commit is release-please's own.
#
# Usage: check-release-tag.sh <tag>
# Env:   BASE_REF  (default origin/main)
#        MANIFEST  (default .release-please-manifest.json)
set -euo pipefail

tag="${1:?usage: check-release-tag.sh <tag>}"
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
