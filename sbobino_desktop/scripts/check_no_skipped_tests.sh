#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

if ! command -v rg >/dev/null 2>&1; then
  echo "Missing required command: rg" >&2
  exit 1
fi

PATTERNS=(
  '\b(describe|it|test)\.skip\s*\('
  '\b(describe|it|test)\.todo\s*\('
  '\b(xdescribe|xit|xtest)\s*\('
  '#\s*\[\s*ignore\s*\]'
)

matches=""
for pattern in "${PATTERNS[@]}"; do
  result=$(
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
  )
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
