#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
. "$ROOT_DIR/scripts/repo_split_common.sh"

FORCE=0

while [ "${1:-}" != "" ]; do
  case "$1" in
    --force)
      FORCE=1
      shift
      ;;
    -*)
      echo "unknown option: $1" >&2
      exit 1
      ;;
    *)
      break
      ;;
  esac
done

if [ "${1:-}" = "" ]; then
  echo "usage: $0 [--force] <target-dir>" >&2
  exit 1
fi

TARGET_DIR="$1"

repo_split_require_clean_worktree "$ROOT_DIR"
repo_split_prepare_target "$TARGET_DIR" "$FORCE"

SOURCE_BRANCH="$(repo_split_current_branch "$ROOT_DIR")"
SOURCE_COMMIT="$(repo_split_current_commit "$ROOT_DIR")"
SOURCE_REPO="$(basename "$ROOT_DIR")"

repo_split_clone_repo "$ROOT_DIR" "$TARGET_DIR" "$SOURCE_BRANCH"

git -C "$TARGET_DIR" remote remove origin >/dev/null 2>&1 || true

repo_split_write_role_manifest "$TARGET_DIR" "ce" "$SOURCE_REPO" "$SOURCE_BRANCH" "$SOURCE_COMMIT"
repo_split_write_ce_docs "$TARGET_DIR" "$SOURCE_REPO" "$SOURCE_BRANCH" "$SOURCE_COMMIT"
repo_split_commit_overlay "$TARGET_DIR" "chore(repo): mark CE upstream skeleton"

echo "CE upstream repository exported to: $TARGET_DIR"
echo "source branch: $SOURCE_BRANCH"
echo "source commit: $SOURCE_COMMIT"
