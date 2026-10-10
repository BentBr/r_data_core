#!/usr/bin/env bash
# Publish a small file to the repo's gh-pages branch.
#
# Shared by the coverage jobs in .github/workflows/coverage.yml (shields.io
# endpoint JSON) and by the MCP release in .github/workflows/mcp-release.yml
# (the download manifest), so the clone/commit/push logic lives in one place.
# Initializes gh-pages on first use if it doesn't exist yet.
#
# Usage: publish-to-pages.sh <dest-filename> <source-file> <label> [message]
#
# Without a message the commit is described as a coverage-badge update, which
# reads the percentage out of the file — the original behaviour, kept so the
# two coverage call sites need no argument they would only repeat.
#
# Requires env: GH_TOKEN (a token with contents:write), REPO (owner/name).
set -euo pipefail

DEST="$1"
SRC="$2"
LABEL="$3"
MESSAGE="${4:-}"
: "${GH_TOKEN:?GH_TOKEN is required}"
: "${REPO:?REPO is required}"

cd /tmp
rm -rf pages
if git clone --branch gh-pages --depth 1 "https://x-access-token:${GH_TOKEN}@github.com/${REPO}.git" pages 2>/dev/null; then
    cd pages
else
    mkdir pages && cd pages
    git init -b gh-pages
    git remote add origin "https://x-access-token:${GH_TOKEN}@github.com/${REPO}.git"
fi

git config user.name 'github-actions[bot]'
git config user.email '41898282+github-actions[bot]@users.noreply.github.com'

cp "$SRC" "$DEST"
git add "$DEST"
if git diff --staged --quiet; then
    echo "${LABEL} unchanged — skipping push"
else
    if [ -z "$MESSAGE" ]; then
        PCT=$(jq -r '.message' "$DEST")
        MESSAGE="chore(coverage): update ${LABEL} badge to ${PCT}"
    fi
    git commit -m "$MESSAGE"
    git push origin gh-pages
fi
