#!/bin/sh
set -eu

WEB_STATIC_DIR="${WEB_STATIC_DIR:-/usr/share/nginx/html}"
RUNTIME_CONFIG_PATH="${RUNTIME_CONFIG_PATH:-$WEB_STATIC_DIR/runtime-config.js}"
API_URL="${LS_API_BASE_URL:-}"
API_URL="${API_URL%/}"

mkdir -p "$(dirname "$RUNTIME_CONFIG_PATH")"

if [ -n "$API_URL" ]; then
  printf 'window.__LS_CONFIG__=Object.assign({},window.__LS_CONFIG__,{"apiBaseUrl":"%s"});\n' "$API_URL"
else
  printf 'window.__LS_CONFIG__=Object.assign({},window.__LS_CONFIG__,{});\n'
fi > "$RUNTIME_CONFIG_PATH"
