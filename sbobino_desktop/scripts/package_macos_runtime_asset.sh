#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <output_zip>" >&2
  exit 1
fi

OUTPUT_ZIP=$1
ROOT_NAME="runtime"
STAGE_DIR=$(mktemp -d)
TARGET_ROOT="$STAGE_DIR/$ROOT_NAME"
TARGET_BIN="$TARGET_ROOT/bin"
TARGET_LIB="$TARGET_ROOT/lib"

cleanup() {
  rm -rf "$STAGE_DIR"
}
trap cleanup EXIT

mkdir -p "$TARGET_BIN" "$TARGET_LIB"
mkdir -p "$(dirname "$OUTPUT_ZIP")"
rm -f "$OUTPUT_ZIP"

WHISPER_CPP_PREFIX=$(brew --prefix whisper-cpp)
FFMPEG_PREFIX=$(brew --prefix ffmpeg)
SDL2_PREFIX=$(brew --prefix sdl2)

SEARCH_DIRS=(
  "$WHISPER_CPP_PREFIX/bin"
  "$WHISPER_CPP_PREFIX/lib"
  "$WHISPER_CPP_PREFIX/libexec/bin"
  "$WHISPER_CPP_PREFIX/libexec/lib"
  "$FFMPEG_PREFIX/bin"
  "$FFMPEG_PREFIX/lib"
  "$SDL2_PREFIX/lib"
)

PRIMARY_BINARIES=(
  "$WHISPER_CPP_PREFIX/bin/whisper-cli"
  "$WHISPER_CPP_PREFIX/bin/whisper-stream"
  "$FFMPEG_PREFIX/bin/ffmpeg"
)

declare -a PENDING_TARGETS=()
LAST_COPIED_TARGET=""
COPIED_PATHS_FILE="$STAGE_DIR/copied_paths.tsv"
touch "$COPIED_PATHS_FILE"

canonical_path() {
  realpath "$1"
}

should_skip_dependency() {
  local dep=$1
  [[ "$dep" == /System/* || "$dep" == /usr/lib/* ]]
}

resolve_dependency_path() {
  local dep=$1
  local source_dir=$2

  if [[ "$dep" == /* ]]; then
    if [[ -e "$dep" ]]; then
      canonical_path "$dep"
      return 0
    fi
    return 1
  fi

  local relative=${dep#@rpath/}
  if [[ "$dep" == @loader_path/* ]]; then
    relative=${dep#@loader_path/}
    if [[ -e "$source_dir/$relative" ]]; then
      canonical_path "$source_dir/$relative"
      return 0
    fi
  fi

  if [[ "$dep" == @executable_path/* ]]; then
    relative=${dep#@executable_path/}
    if [[ -e "$source_dir/$relative" ]]; then
      canonical_path "$source_dir/$relative"
      return 0
    fi
  fi

  local base_name
  base_name=$(basename "$relative")
  for dir in "$source_dir" "${SEARCH_DIRS[@]}"; do
    if [[ -e "$dir/$base_name" ]]; then
      canonical_path "$dir/$base_name"
      return 0
    fi
  done

  return 1
}

copy_and_queue() {
  local source=$1
  local target_dir=$2
  local canonical
  canonical=$(canonical_path "$source")
  local base_name
  base_name=$(basename "$canonical")
  local target="$target_dir/$base_name"
  local existing_target

  existing_target=$(awk -F $'\t' -v canonical="$canonical" -v target_dir="$target_dir" '$1 == canonical && index($2, target_dir "/") == 1 { print $2; exit }' "$COPIED_PATHS_FILE")
  if [[ -n "$existing_target" ]]; then
    LAST_COPIED_TARGET="$existing_target"
    return 0
  fi

  cp -L "$canonical" "$target"
  chmod u+w "$target"
  printf '%s\t%s\n' "$canonical" "$target" >> "$COPIED_PATHS_FILE"
  PENDING_TARGETS+=("$target::$canonical")
  LAST_COPIED_TARGET="$target"
}

ensure_alias_link() {
  local requested_base_name=$1
  local target_path=$2
  local target_dir
  target_dir=$(dirname "$target_path")
  local actual_base_name
  actual_base_name=$(basename "$target_path")
  local alias_path="$target_dir/$requested_base_name"

  if [[ "$requested_base_name" == "$actual_base_name" || -e "$alias_path" ]]; then
    return 0
  fi

  ln -sf "$actual_base_name" "$alias_path"
}

for binary in "${PRIMARY_BINARIES[@]}"; do
  if [[ ! -x "$binary" ]]; then
    echo "Missing runtime binary: $binary" >&2
    exit 1
  fi
  copy_and_queue "$binary" "$TARGET_BIN" >/dev/null
done

while [[ ${#PENDING_TARGETS[@]} -gt 0 ]]; do
  entry=${PENDING_TARGETS[0]}
  PENDING_TARGETS=("${PENDING_TARGETS[@]:1}")

  target=${entry%%::*}
  source=${entry##*::}
  source_dir=$(dirname "$source")

  while IFS= read -r dep; do
    if [[ -z "$dep" ]] || should_skip_dependency "$dep"; then
      continue
    fi

    dep_base_name=$(basename "${dep#@rpath/}")
    if ! resolved=$(resolve_dependency_path "$dep" "$source_dir"); then
      echo "Unable to resolve dependency '$dep' for '$source'." >&2
      exit 1
    fi

    copy_and_queue "$resolved" "$TARGET_LIB"
    copied_target="$LAST_COPIED_TARGET"
    ensure_alias_link "$dep_base_name" "$copied_target"

    if [[ "$target" == "$TARGET_BIN/"* ]]; then
      install_name_tool -change "$dep" "@executable_path/../lib/$dep_base_name" "$target"
    else
      install_name_tool -change "$dep" "@loader_path/$dep_base_name" "$target"
    fi
  done < <(otool -L "$source" | tail -n +2 | awk '{print $1}')

  if [[ "$target" == *.dylib ]]; then
    install_name_tool -id "@rpath/$(basename "$target")" "$target"
  else
    install_name_tool -add_rpath "@executable_path/../lib" "$target" 2>/dev/null || true
  fi
done

while IFS= read -r file; do
  if file "$file" | grep -q "Mach-O"; then
    codesign --force --sign - "$file" >/dev/null 2>&1 || true
  fi
done < <(find "$TARGET_ROOT" -type f | sort)

ditto -c -k --sequesterRsrc --keepParent "$TARGET_ROOT" "$OUTPUT_ZIP"
echo "Created $OUTPUT_ZIP"
