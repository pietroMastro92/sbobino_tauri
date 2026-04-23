#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: prune_local_releases.sh [release-root] [keep-count] [current-tag]

Deletes older local dist/local-release/v* directories, keeping only the newest
versions plus an optional current tag that must never be removed.
EOF
}

if [[ $# -gt 3 ]]; then
  usage
  exit 1
fi

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RELEASE_ROOT=${1:-"$SCRIPT_DIR/../dist/local-release"}
KEEP_COUNT=${2:-2}
CURRENT_TAG=${3:-}

if [[ ! "$KEEP_COUNT" =~ ^[0-9]+$ ]] || [[ "$KEEP_COUNT" -lt 1 ]]; then
  echo "keep-count must be a positive integer." >&2
  exit 1
fi

DIRS_TO_DELETE=$(python3 - <<'PY' "$RELEASE_ROOT" "$KEEP_COUNT" "$CURRENT_TAG"
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
keep_count = int(sys.argv[2])
current_tag = str(sys.argv[3]).strip()
version_pattern = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")

if not root.is_dir():
    raise SystemExit(0)

def version_key(path: pathlib.Path) -> tuple[int, int, int]:
    match = version_pattern.fullmatch(path.name)
    if not match:
        return (-1, -1, -1)
    return tuple(int(part) for part in match.groups())

release_dirs = [
    path
    for path in root.iterdir()
    if path.is_dir() and version_pattern.fullmatch(path.name)
]
release_dirs.sort(key=version_key, reverse=True)

keep_names = {path.name for path in release_dirs[:keep_count]}
if current_tag:
    keep_names.add(current_tag)

for path in release_dirs:
    if path.name not in keep_names:
        print(path)
PY
)

REMOVED=0
if [[ -n "${DIRS_TO_DELETE// }" ]]; then
  while IFS= read -r release_dir; do
    [[ -z "$release_dir" ]] && continue
    rm -rf "$release_dir"
    echo "Removed stale local release artifacts: $release_dir"
    REMOVED=$((REMOVED + 1))
  done <<<"$DIRS_TO_DELETE"
fi

cat <<EOF
Local release pruning completed:
  root:    $RELEASE_ROOT
  kept:    $KEEP_COUNT
  removed: $REMOVED
EOF
