#!/bin/sh
# Seeds /app/config on first start, then replaces this shell with the proxy so
# it receives SIGTERM directly from tini.
set -eu

CONFIG_DIR=/app/config
CONFIG_FILE="$CONFIG_DIR/config.json"
CREDENTIALS_FILE="$CONFIG_DIR/credentials.json"

json_escape() {
  printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'
}

mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_FILE" ]; then
  host=$(json_escape "${HOST:-0.0.0.0}")
  admin_psw=$(json_escape "${ADMIN_PSW:-${ADMIN_API_KEY:-}}")
  printf '{"host":"%s","port":%s,"adminPsw":"%s"}\n' \
    "$host" "${PORT:-5678}" "$admin_psw" > "$CONFIG_FILE"
fi

if [ ! -f "$CREDENTIALS_FILE" ]; then
  echo '[]' > "$CREDENTIALS_FILE"
fi

exec /app/kiro2cc-proxy --config "$CONFIG_FILE" --credentials "$CREDENTIALS_FILE" "$@"
