#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: verify_local_apple_silicon_release.sh <version> [repo-slug]

Builds the local Apple Silicon release, serves the exact local candidate assets,
and validates the native in-app update path locally before any GitHub publish.

Outputs:
  dist/local-release/v<version>/AS-PRIMARY.local-prepublish-report.json

Environment:
  SBOBINO_SKIP_PREPARE_LOCAL_RELEASE=1  Reuse existing local release assets.
EOF
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage
  exit 1
fi

VERSION=$1
REPO_SLUG=${2:-pietroMastro92/Sbobino}
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
RELEASE_DIR="$ROOT_DIR/dist/local-release/v$VERSION"
REPORT_PATH="$RELEASE_DIR/AS-PRIMARY.local-prepublish-report.json"
SERVER_LOG=$(mktemp)
PORT_FILE=$(mktemp)
SERVER_PID=""

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

cleanup() {
  if [[ -n "${SERVER_PID// }" ]]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  rm -f "$SERVER_LOG" "$PORT_FILE"
}
trap cleanup EXIT

need_cmd python3

if [[ "${SBOBINO_SKIP_PREPARE_LOCAL_RELEASE:-0}" != "1" ]]; then
  "$SCRIPT_DIR/prepare_local_release.sh" "$VERSION"
elif [[ ! -d "$RELEASE_DIR" ]]; then
  echo "Cannot skip prepare_local_release.sh because $RELEASE_DIR does not exist." >&2
  exit 1
fi

python3 - <<'PY' "$RELEASE_DIR" "$PORT_FILE" >"$SERVER_LOG" 2>&1 &
import functools
import http.server
import pathlib
import socketserver
import sys

directory = pathlib.Path(sys.argv[1]).resolve()
port_file = pathlib.Path(sys.argv[2])

handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(directory))

class ReusableTCPServer(socketserver.TCPServer):
    allow_reuse_address = True

with ReusableTCPServer(("127.0.0.1", 0), handler) as httpd:
    port_file.write_text(str(httpd.server_address[1]), encoding="utf-8")
    httpd.serve_forever()
PY
SERVER_PID=$!

for _ in $(seq 1 50); do
  if [[ -s "$PORT_FILE" ]]; then
    break
  fi
  sleep 0.2
done

if [[ ! -s "$PORT_FILE" ]]; then
  echo "Failed to start local release asset server." >&2
  cat "$SERVER_LOG" >&2 || true
  exit 1
fi

PORT=$(cat "$PORT_FILE")
LOCAL_FEED_URL="http://127.0.0.1:${PORT}/latest.json"
LOCAL_ASSET_BASE_URL="http://127.0.0.1:${PORT}"

SBOBINO_VALIDATION_LOCAL_RELEASE_DIR="$RELEASE_DIR" \
SBOBINO_VALIDATION_FEED_URL="$LOCAL_FEED_URL" \
SBOBINO_VALIDATION_ASSET_BASE_URL="$LOCAL_ASSET_BASE_URL" \
bash "$SCRIPT_DIR/run_release_machine_validation.sh" \
  AS-PRIMARY \
  "$VERSION" \
  "$REPO_SLUG" \
  "$REPORT_PATH"

cat <<EOF
Local Apple Silicon prepublish validation passed:
  version: $VERSION
  assets:  $RELEASE_DIR
  report:  $REPORT_PATH
  feed:    $LOCAL_FEED_URL
EOF
