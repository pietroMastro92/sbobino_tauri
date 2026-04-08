#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <version> [app-path]" >&2
  exit 1
fi

VERSION=$1
APP_PATH=${2:-}

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
DESKTOP_DIR="$ROOT_DIR/apps/desktop"
SCRIPTS_DIR="$ROOT_DIR/scripts"
TEMP_DIR=$(mktemp -d)
ASSET_DIR="$TEMP_DIR/release-assets"
PYANNOTE_RUNTIME_ZIP="$ASSET_DIR/pyannote-runtime-macos-aarch64.zip"
PYANNOTE_MODEL_ZIP="$ASSET_DIR/pyannote-model-community-1.zip"
PYANNOTE_MANIFEST="$ASSET_DIR/pyannote-manifest.json"
RUNTIME_ZIP="$ASSET_DIR/speech-runtime-macos-aarch64.zip"
RUNTIME_MANIFEST="$ASSET_DIR/runtime-manifest.json"

cleanup() {
  rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

resolve_bundled_pyannote_root() {
  local app_path=$1
  local candidates=(
    "$app_path/Contents/Resources/pyannote"
    "$app_path/Contents/Resources/resources/pyannote"
  )

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -d "$candidate/python" && -d "$candidate/model" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

assert_bundle_contains_no_local_user_data() {
  local app_path=$1
  local bundled_pyannote_root=${2:-}
  local hits=()

  local file_find_root=("$app_path/Contents")
  local dir_find_root=("$app_path/Contents")
  if [[ -n "$bundled_pyannote_root" ]]; then
    file_find_root=(
      "$app_path/Contents"
      "(" -path "$bundled_pyannote_root" -o -path "$bundled_pyannote_root/*" ")" -prune -o
    )
    dir_find_root=(
      "$app_path/Contents"
      "(" -path "$bundled_pyannote_root" -o -path "$bundled_pyannote_root/*" ")" -prune -o
    )
  fi

  while IFS= read -r match; do
    [[ -n "$match" ]] && hits+=("$match")
  done < <(
    find "${file_find_root[@]}" \
      \( \
        -iname 'settings.json' -o \
        -iname 'setup-report.json' -o \
        -iname 'artifacts.db' -o \
        -iname 'artifacts.db-*' -o \
        -iname '*.sqlite' -o \
        -iname '*.sqlite3' -o \
        -iname '*.wav' -o \
        -iname '*.mp3' -o \
        -iname '*.m4a' -o \
        -iname '*.aac' -o \
        -iname '*.ogg' -o \
        -iname '*.opus' -o \
        -iname '*.flac' -o \
        -iname '*.srt' -o \
        -iname '*.vtt' -o \
        -iname '*.docx' -o \
        -iname '*.pdf' \
      \) -print
  )

  while IFS= read -r match; do
    [[ -n "$match" ]] && hits+=("$match")
  done < <(
    find "${dir_find_root[@]}" -type d \
      \( \
        -iname 'audio-vault' -o \
        -iname 'artifacts' -o \
        -iname 'backups' -o \
        -iname 'deleted' \
      \) -print
  )

  if (( ${#hits[@]} > 0 )); then
    echo "Release bundle contains local user data or user-generated artifacts:" >&2
    printf ' - %s\n' "${hits[@]}" >&2
    exit 1
  fi
}

mkdir -p "$ASSET_DIR"

"$SCRIPTS_DIR/check_release_versions.sh" "$VERSION"

"$SCRIPTS_DIR/package_macos_runtime_asset.sh" "$RUNTIME_ZIP"
"$SCRIPTS_DIR/package_pyannote_asset.sh" \
  "$DESKTOP_DIR/src-tauri/resources/pyannote/python/aarch64-apple-darwin" \
  python \
  "$PYANNOTE_RUNTIME_ZIP"
"$SCRIPTS_DIR/package_pyannote_asset.sh" \
  "$DESKTOP_DIR/src-tauri/resources/pyannote/model" \
  model \
  "$PYANNOTE_MODEL_ZIP"

RUNTIME_SHA=$(shasum -a 256 "$RUNTIME_ZIP" | awk '{print $1}')
PYANNOTE_RUNTIME_SHA=$(shasum -a 256 "$PYANNOTE_RUNTIME_ZIP" | awk '{print $1}')
PYANNOTE_MODEL_SHA=$(shasum -a 256 "$PYANNOTE_MODEL_ZIP" | awk '{print $1}')

cat >"$RUNTIME_MANIFEST" <<JSON
{
  "app_version": "$VERSION",
  "assets": [
    {
      "kind": "speech_runtime_macos_aarch64",
      "name": "speech-runtime-macos-aarch64.zip",
      "sha256": "$RUNTIME_SHA"
    }
  ]
}
JSON

cat >"$PYANNOTE_MANIFEST" <<JSON
{
  "app_version": "$VERSION",
  "assets": [
    {
      "kind": "pyannote_runtime_macos_aarch64",
      "name": "pyannote-runtime-macos-aarch64.zip",
      "sha256": "$PYANNOTE_RUNTIME_SHA"
    },
    {
      "kind": "pyannote_model",
      "name": "pyannote-model-community-1.zip",
      "sha256": "$PYANNOTE_MODEL_SHA"
    }
  ]
}
JSON

export SBOBINO_LOCAL_RELEASE_ASSETS_DIR="$ASSET_DIR"

pushd "$DESKTOP_DIR" >/dev/null
npm test -- initialSetup provisioningUi appBootstrap
popd >/dev/null

pushd "$ROOT_DIR" >/dev/null
cargo test -p sbobino-infrastructure runtime_health_reports_version_mismatch_as_repair_required
cargo test -p sbobino-infrastructure runtime_health_reports_install_incomplete_when_python_stdlib_is_missing
cargo test -p sbobino-infrastructure runtime_health_self_heals_missing_manifest_and_status_from_bundled_override
cargo test -p sbobino-desktop install_pyannote_archive_extracts_expected_root
cargo test -p sbobino-desktop verify_file_sha256_rejects_wrong_checksum
popd >/dev/null

if [[ -n "$APP_PATH" ]]; then
  if [[ ! -d "$APP_PATH" ]]; then
    echo "Built app not found at '$APP_PATH'." >&2
    exit 1
  fi

  APP_EXECUTABLE_NAME=$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$APP_PATH/Contents/Info.plist")
  APP_EXEC="$APP_PATH/Contents/MacOS/$APP_EXECUTABLE_NAME"
  if [[ ! -x "$APP_EXEC" ]]; then
    echo "App executable missing at '$APP_EXEC'." >&2
    exit 1
  fi

  for binary in whisper-cli whisper-stream ffmpeg; do
    if [[ ! -x "$APP_PATH/Contents/MacOS/$binary" ]]; then
      echo "Bundled binary missing: $APP_PATH/Contents/MacOS/$binary" >&2
      exit 1
    fi
  done

  PYANNOTE_ROOT=$(resolve_bundled_pyannote_root "$APP_PATH") || {
    echo "Bundled pyannote resources missing from '$APP_PATH/Contents/Resources'." >&2
    exit 1
  }

  assert_bundle_contains_no_local_user_data "$APP_PATH" "$PYANNOTE_ROOT"
fi

echo "Release readiness checks passed for version $VERSION"
