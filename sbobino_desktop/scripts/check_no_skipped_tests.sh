#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

PATTERNS=(
  '(^|[^[:alnum:]_])(describe|it|test)[.]skip[[:space:]]*[(]'
  '(^|[^[:alnum:]_])(describe|it|test)[.]todo[[:space:]]*[(]'
  '(^|[^[:alnum:]_])(xdescribe|xit|xtest)[[:space:]]*[(]'
  '#[[:space:]]*\[[[:space:]]*ignore[[:space:]]*\]'
)

scan_with_ripgrep() {
  local pattern=$1
  rg \
    --line-number \
    --hidden \
    --glob '!node_modules/**' \
    --glob '!target/**' \
    --glob '!dist/**' \
    --glob '!apps/desktop/src-tauri/resources/**' \
    --glob '!scripts/check_no_skipped_tests.sh' \
    --glob '!Cargo.lock' \
    --glob '!apps/desktop/package-lock.json' \
    "$pattern" \
    "$ROOT_DIR" || true
}

scan_with_grep() {
  local pattern=$1
  find "$ROOT_DIR" \
    \( \
      -path "$ROOT_DIR/target" \
      -o -path "$ROOT_DIR/apps/desktop/node_modules" \
      -o -path "$ROOT_DIR/apps/desktop/dist" \
      -o -path "$ROOT_DIR/apps/desktop/src-tauri/resources" \
    \) -prune \
    -o -type f \
    ! -path "$ROOT_DIR/scripts/check_no_skipped_tests.sh" \
    ! -path "$ROOT_DIR/Cargo.lock" \
    ! -path "$ROOT_DIR/apps/desktop/package-lock.json" \
    -print0 |
    xargs -0 grep -En "$pattern" || true
}

matches=""
for pattern in "${PATTERNS[@]}"; do
  if command -v rg >/dev/null 2>&1; then
    result=$(scan_with_ripgrep "$pattern")
  else
    result=$(scan_with_grep "$pattern")
  fi
  if [[ -n "$result" ]]; then
    matches+=$'\n'"$result"
  fi
done

if [[ -n "$matches" ]]; then
  echo "Skipped or todo test declarations are not allowed in first-party tests:" >&2
  printf '%s\n' "$matches" >&2
  exit 1
fi

echo "No skipped or todo first-party tests found."
