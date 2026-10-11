#!/usr/bin/env bash
# The release App's key may be read by exactly one workflow.
#
# RELEASE_APP_PRIVATE_KEY mints a token with contents:write, which is what
# pushes the mcp-v* tag that starts a release. A workflow that runs on branch
# pushes and reads that secret would let anyone with push access mint the
# same token from their branch — which is how the PR-management App was
# wired before the two were split. This keeps the split honest.
#
# Usage: check-app-secret-scope.sh [workflows-dir]
set -euo pipefail

dir="${1:-.github/workflows}"
secret='RELEASE_APP_PRIVATE_KEY'
allowed='release-please.yml'
status=0

while IFS= read -r file; do
    if grep -q "$secret" "$file" && [ "$(basename "$file")" != "$allowed" ]; then
        echo "::error file=$file::$secret may only be read by $allowed" >&2
        status=1
    fi
done < <(find "$dir" -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) | sort)

[ "$status" -eq 0 ] && echo "$secret is confined to $allowed"
exit "$status"
