#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: verify_local_apple_silicon_release.sh <version> [repo-slug]

Builds and verifies the local Apple Silicon release assets before any GitHub
publish. Native updater validation must run after prerelease publication because
Tauri updater endpoints must use HTTPS.

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
  rm -f "$SERVER_LOG"
}
trap cleanup EXIT

need_cmd python3

if [[ "${SBOBINO_SKIP_PREPARE_LOCAL_RELEASE:-0}" != "1" ]]; then
  "$SCRIPT_DIR/prepare_local_release.sh" "$VERSION"
elif [[ ! -d "$RELEASE_DIR" ]]; then
  echo "Cannot skip prepare_local_release.sh because $RELEASE_DIR does not exist." >&2
  exit 1
fi

python3 - <<'PY' "$RELEASE_DIR" "$REPORT_PATH" "$VERSION" "$REPO_SLUG"
import json
import pathlib
import sys
from datetime import datetime, timezone

release_dir = pathlib.Path(sys.argv[1]).resolve()
report_path = pathlib.Path(sys.argv[2]).resolve()
version = sys.argv[3]
repo_slug = sys.argv[4]
tag = f"v{version}"

required = [
    f"Sbobino_{version}_aarch64.dmg",
    "Sbobino.app.tar.gz",
    "Sbobino.app.tar.gz.sig",
    "latest.json",
    "setup-manifest.json",
    "runtime-manifest.json",
    "speech-runtime-macos-aarch64.zip",
    "pyannote-manifest.json",
    "pyannote-runtime-macos-aarch64.zip",
    "pyannote-model-community-1.zip",
    "release-readiness-proof.json",
]
missing = [name for name in required if not (release_dir / name).is_file()]
if missing:
    raise SystemExit("Missing local release assets: " + ", ".join(missing))

proof = json.loads((release_dir / "release-readiness-proof.json").read_text(encoding="utf-8"))
if proof.get("version") != version:
    raise SystemExit("release-readiness-proof.json version mismatch.")
if str(proof.get("status", "")).strip().lower() != "passed":
    raise SystemExit("release-readiness-proof.json is not passed.")
if proof.get("gate") != "release_readiness.sh":
    raise SystemExit("release-readiness-proof.json gate mismatch.")

latest = json.loads((release_dir / "latest.json").read_text(encoding="utf-8"))
if latest.get("version") != version:
    raise SystemExit("latest.json version mismatch.")
platform = latest.get("platforms", {}).get("darwin-aarch64", {})
url = str(platform.get("url", "")).strip()
if not url.endswith(f"/releases/download/{tag}/Sbobino.app.tar.gz"):
    raise SystemExit("latest.json updater URL is not tied to the expected GitHub release tag.")

report = {
    "schema_version": 1,
    "version": version,
    "release_tag": tag,
    "release_url": f"https://github.com/{repo_slug}/releases/tag/{tag}",
    "machine_class": "AS-PRIMARY",
    "status": "passed",
    "tester": "local-prepublish",
    "os_name": "macOS",
    "os_version": "",
    "runner_label": "local,macos,apple-silicon,prepublish",
    "tested_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "notes": "Local Apple Silicon asset preflight passed. Native updater validation must run against the HTTPS GitHub prerelease.",
    "required_scenarios": ["local_release_assets_prepared"],
    "scenario_results": {"local_release_assets_prepared": "passed"},
}
report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
PY

cat <<EOF
Local Apple Silicon prepublish asset validation passed:
  version: $VERSION
  assets:  $RELEASE_DIR
  report:  $REPORT_PATH
EOF
