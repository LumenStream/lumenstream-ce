#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

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
  echo "usage: $0 [--force] <base-dir> [ce-dir-name] [commercial-dir-name]" >&2
  exit 1
fi

BASE_DIR="$1"
CE_DIR_NAME="${2:-lumenstream-ce}"
COMMERCIAL_DIR_NAME="${3:-lumenstream-commercial}"

CE_TARGET="$BASE_DIR/$CE_DIR_NAME"
COMMERCIAL_TARGET="$BASE_DIR/$COMMERCIAL_DIR_NAME"

FORCE_FLAG=""
if [ "$FORCE" = "1" ]; then
  FORCE_FLAG="--force"
fi

bash "$ROOT_DIR/scripts/export_ce_upstream.sh" $FORCE_FLAG "$CE_TARGET"
bash "$ROOT_DIR/scripts/init_commercial_downstream.sh" $FORCE_FLAG "$COMMERCIAL_TARGET" "$CE_TARGET"

echo
echo "split repositories created:"
echo "- CE upstream:        $CE_TARGET"
echo "- Commercial overlay: $COMMERCIAL_TARGET"
echo
echo "commercial upstream remote:"
git -C "$COMMERCIAL_TARGET" remote -v | grep '^upstream' || true
