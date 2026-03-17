#!/usr/bin/env bash
set -euo pipefail

UPSTREAM_REMOTE="${1:-upstream}"
UPSTREAM_BRANCH="${2:-main}"

git fetch "$UPSTREAM_REMOTE" "$UPSTREAM_BRANCH"
git merge --no-ff "$UPSTREAM_REMOTE/$UPSTREAM_BRANCH"
