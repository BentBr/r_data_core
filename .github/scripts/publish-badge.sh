#!/usr/bin/env bash
# Publish a shields.io endpoint JSON to the repo's gh-pages branch.
# Shared by the backend-coverage and frontend-coverage jobs in
# .github/workflows/coverage.yml — keeps the clone/commit/push logic in one
# place. Initializes gh-pages on first use if it doesn't exist yet.
#
# Usage: publish-badge.sh <dest-filename-in-gh-pages> <source-json> <label>
# Requires env: GH_TOKEN (a token with contents:write), REPO (owner/name).
set -euo pipefail

DEST="$1"
SRC="$2"
LABEL="$3"
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
    echo "${LABEL} coverage unchanged — skipping push"
else
    PCT=$(jq -r '.message' "$DEST")
    git commit -m "chore(coverage): update ${LABEL} badge to ${PCT}"
    git push origin gh-pages
fi
