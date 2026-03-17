#!/bin/sh
set -eu

RUNTIME_CONFIG_WRITER="${RUNTIME_CONFIG_WRITER:-/usr/local/bin/lumenstream-write-web-runtime-config.sh}"

if [ ! -x "$RUNTIME_CONFIG_WRITER" ]; then
  echo "runtime config writer not found: $RUNTIME_CONFIG_WRITER" >&2
  exit 1
fi

"$RUNTIME_CONFIG_WRITER"

exec nginx -g "daemon off;"
