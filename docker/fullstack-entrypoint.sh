#!/bin/sh
set -eu

BACKEND_BIN="${BACKEND_BIN:-/usr/local/bin/ls-app}"
RUNTIME_CONFIG_WRITER="${RUNTIME_CONFIG_WRITER:-/usr/local/bin/lumenstream-write-web-runtime-config.sh}"

if [ ! -x "$BACKEND_BIN" ]; then
  echo "backend binary not found: $BACKEND_BIN" >&2
  exit 1
fi

if [ ! -x "$RUNTIME_CONFIG_WRITER" ]; then
  echo "runtime config writer not found: $RUNTIME_CONFIG_WRITER" >&2
  exit 1
fi

if [ -n "${LS_BOOTSTRAP_ADMIN_USER:-}" ] && [ -z "${LS_BOOTSTRAP_ADMIN_PASSWORD:-}" ]; then
  echo "LS_BOOTSTRAP_ADMIN_PASSWORD is required when LS_BOOTSTRAP_ADMIN_USER is set" >&2
  exit 1
fi

if [ -z "${LS_BOOTSTRAP_ADMIN_USER:-}" ] && [ -n "${LS_BOOTSTRAP_ADMIN_PASSWORD:-}" ]; then
  echo "LS_BOOTSTRAP_ADMIN_USER is required when LS_BOOTSTRAP_ADMIN_PASSWORD is set" >&2
  exit 1
fi

"$RUNTIME_CONFIG_WRITER"

"$BACKEND_BIN" &
backend_pid="$!"

nginx -g "daemon off;" &
web_pid="$!"

terminate() {
  kill -TERM "$backend_pid" "$web_pid" 2>/dev/null || true
}

trap terminate INT TERM

exit_code=0
while kill -0 "$backend_pid" 2>/dev/null && kill -0 "$web_pid" 2>/dev/null; do
  sleep 1
done

if ! kill -0 "$backend_pid" 2>/dev/null; then
  wait "$backend_pid" || exit_code="$?"
else
  wait "$web_pid" || exit_code="$?"
fi

terminate
wait "$backend_pid" 2>/dev/null || true
wait "$web_pid" 2>/dev/null || true

exit "$exit_code"
