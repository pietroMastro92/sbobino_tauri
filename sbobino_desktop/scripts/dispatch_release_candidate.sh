#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: dispatch_release_candidate.sh <tag-name> [repo-slug]

Requires a passed local Apple Silicon prepublish report, checks the
AS-PRIMARY runner, and dispatches the Release Candidate workflow.
EOF
}

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage
  exit 1
fi

TAG_NAME=$1
REPO_SLUG=${2:-pietroMastro92/Sbobino}
ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
VERSION=${TAG_NAME#v}
LOCAL_RELEASE_DIR="$ROOT_DIR/dist/local-release/v$VERSION"
LOCAL_PREPUBLISH_REPORT="$LOCAL_RELEASE_DIR/AS-PRIMARY.local-prepublish-report.json"

if ! command -v gh >/dev/null 2>&1; then
  echo "Missing required command: gh" >&2
  exit 1
fi

python3 - <<'PY' "$LOCAL_PREPUBLISH_REPORT" "$VERSION"
import json
import pathlib
import sys

report_path = pathlib.Path(sys.argv[1])
version = sys.argv[2]

if not report_path.is_file():
    raise SystemExit(
        "Missing local Apple Silicon prepublish report. Run ./scripts/verify_local_apple_silicon_release.sh first."
    )

report = json.loads(report_path.read_text(encoding="utf-8"))
if report.get("version") != version:
    raise SystemExit("Local Apple Silicon prepublish report version mismatch.")
if report.get("machine_class") != "AS-PRIMARY":
    raise SystemExit("Local Apple Silicon prepublish report machine_class mismatch.")
if str(report.get("status", "")).strip().lower() != "passed":
    raise SystemExit("Local Apple Silicon prepublish report is not passed.")
PY

"$ROOT_DIR/scripts/check_release_runner_matrix.sh" "$REPO_SLUG" AS-PRIMARY

gh workflow run "Release Candidate" \
  --repo "$REPO_SLUG" \
  -f "tag_name=$TAG_NAME"

cat <<EOF
Release candidate workflow dispatched successfully.
  repo: $REPO_SLUG
  tag:  $TAG_NAME
  local-prepublish-proof: $LOCAL_PREPUBLISH_REPORT

Next recommended command:
  gh run list --repo "$REPO_SLUG" --workflow "Release Candidate" --limit 5
EOF
