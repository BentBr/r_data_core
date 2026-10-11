#!/usr/bin/env bash
# The release App's key may be read by exactly one workflow, from one
# environment.
#
# RELEASE_APP_PRIVATE_KEY mints a token with contents:write for the App the
# mcp-v* tag ruleset trusts. A workflow that runs on branch pushes and could
# read that secret would let anyone with push access mint the same token from
# their branch — which is how the PR-management App was wired before the two
# were split. Two checks keep that from creeping back:
#   - only release-please.yml names the secret, and
#   - every job there that names it declares `environment: release-tag`, the
#     environment that holds the secret and allows deployments from main only.
# The second is what makes the secret unreadable from a branch; it only holds
# if the operator stored the values as environment secrets, not repository
# ones (docs/MCP.md, "Releasing").
#
# A drift detector, not a boundary: it greps for the name, so it cannot see
# `secrets: inherit` or `toJSON(secrets)`. The environment is the boundary.
#
# Usage: check-app-secret-scope.sh [workflows-dir]
set -euo pipefail

dir="${1:-.github/workflows}"
secret='RELEASE_APP_PRIVATE_KEY'
allowed='release-please.yml'
environment='release-tag'
status=0

while IFS= read -r file; do
    grep -q "$secret" "$file" || continue
    if [ "$(basename "$file")" != "$allowed" ]; then
        echo "::error file=$file::$secret may only be read by $allowed" >&2
        status=1
        continue
    fi
    # Walk the jobs: remember the current job header and whether it declared
    # the environment, and report any job that names the secret without it.
    # Job headers are the lines at the indentation of the first key under
    # `jobs:`.
    if ! offenders="$(awk -v secret="$secret" -v env="$environment" '
        /^jobs:[[:space:]]*$/ { injobs = 1; next }
        injobs && indent == "" && /^[[:space:]]+[^[:space:]#]/ {
            match($0, /^[[:space:]]+/); indent = RLENGTH
        }
        injobs && indent != "" {
            match($0, /^[[:space:]]*/)
            if (RLENGTH == indent && $0 ~ /^[[:space:]]*[A-Za-z0-9_-]+:[[:space:]]*$/) {
                job = $0; sub(/^[[:space:]]+/, "", job); sub(/:.*/, "", job); declared = 0
            }
        }
        $0 ~ "^[[:space:]]+environment:[[:space:]]*[\"'\'']?" env "[\"'\'']?[[:space:]]*(#.*)?$" { declared = 1 }
        index($0, secret) && !declared { print job; bad = 1 }
        END { exit bad }
    ' "$file")"; then
        for job in $(printf '%s\n' "$offenders" | sort -u); do
            echo "::error file=$file::job '$job' reads $secret without declaring environment: $environment" >&2
        done
        status=1
    fi
done < <(find "$dir" -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) | sort)

[ "$status" -eq 0 ] && echo "$secret is confined to $allowed, environment $environment"
exit "$status"
