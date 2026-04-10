#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: setup_bundled_pyannote.sh [--force] [--python-version VERSION] [--model-repo URL]

Downloads a local offline pyannote model and creates a bundled Python runtime
under apps/desktop/src-tauri/resources/pyannote so Tauri builds can ship a
standalone diarization-capable app without post-install provisioning.

Options:
  --force                 Rebuild runtime and re-download the model.
  --python-version X.Y    Python version for the bundled runtime. Default: 3.11
  --model-repo URL        Git/LFS repository containing an offline pyannote
                          pipeline. Default:
                          https://huggingface.co/pyannote-community/speaker-diarization-community-1
EOF
}

FORCE=0
PYTHON_VERSION=3.11
MODEL_REPO="https://huggingface.co/pyannote-community/speaker-diarization-community-1"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force)
      FORCE=1
      shift
      ;;
    --python-version)
      PYTHON_VERSION=${2:-}
      shift 2
      ;;
    --model-repo)
      MODEL_REPO=${2:-}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

need_cmd git
need_cmd git-lfs
need_cmd rsync
need_cmd install_name_tool
need_cmd codesign
need_cmd otool

python_matches_version() {
  local python_bin=$1
  local requested=$2
  "$python_bin" - <<PY >/dev/null 2>&1
import sys
requested = "$requested".strip()
major_minor = f"{sys.version_info.major}.{sys.version_info.minor}"
raise SystemExit(0 if requested == major_minor else 1)
PY
}

python_is_healthy() {
  local python_bin=$1
  "$python_bin" - <<'PY' >/dev/null 2>&1
import ctypes
import csv
import encodings
print("ok")
PY
}

rewrite_embedded_python_app_launcher() {
  local runtime_dir=$1
  local libpython_target=$2
  local python_app="$runtime_dir/lib/Resources/Python.app/Contents/MacOS/Python"

  if [[ ! -x "$python_app" ]]; then
    return 0
  fi

  local python_app_dep=""
  while IFS= read -r dep; do
    if [[ "$dep" == /*Python.framework/Versions/*/Python ]]; then
      python_app_dep=$dep
      break
    fi
  done < <(otool -L "$python_app" | tail -n +2 | awk '{print $1}')

  if [[ -z "$python_app_dep" ]]; then
    return 0
  fi

  install_name_tool -add_rpath "@executable_path/../../../../" "$python_app" 2>/dev/null || true
  install_name_tool -change "$python_app_dep" "@rpath/$(basename "$libpython_target")" "$python_app"
  codesign --force --sign - "$python_app" >/dev/null 2>&1 || true
}

verify_no_external_python_framework_refs() {
  local runtime_dir=$1
  local candidate

  for candidate in \
    "$runtime_dir/bin/python" \
    "$runtime_dir/bin/python3" \
    "$runtime_dir/lib/Resources/Python.app/Contents/MacOS/Python"; do
    if [[ ! -e "$candidate" ]]; then
      continue
    fi

    if otool -L "$candidate" | tail -n +2 | awk '{print $1}' | grep -E '^/.+Python\.framework/Versions/.+/Python$' >/dev/null; then
      echo "Bundled runtime still references an external Python.framework in $candidate" >&2
      otool -L "$candidate" >&2
      exit 1
    fi
  done
}

resolve_python_executable() {
  local requested=$1
  local candidates=()

  if [[ -x "$requested" ]]; then
    candidates+=("$requested")
  fi

  if command -v "python$requested" >/dev/null 2>&1; then
    candidates+=("$(command -v "python$requested")")
  fi

  if command -v python3 >/dev/null 2>&1; then
    candidates+=("$(command -v python3)")
  fi

  if [[ -x "$HOME/miniconda3/bin/python3" ]]; then
    candidates+=("$HOME/miniconda3/bin/python3")
  fi

  local candidate
  for candidate in "${candidates[@]}"; do
    if python_matches_version "$candidate" "$requested" && python_is_healthy "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  echo "Could not find a healthy Python $requested interpreter on this machine." >&2
  echo "Install one first, then rerun this script. Example: brew install python@$requested" >&2
  exit 1
}

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
RESOURCE_ROOT="$ROOT_DIR/apps/desktop/src-tauri/resources/pyannote"
MODEL_DIR="$RESOURCE_ROOT/model"
PYTHON_ROOT="$RESOURCE_ROOT/python"

HOST_ARCH=$(uname -m)
case "$HOST_ARCH" in
  arm64)
    TARGET_TRIPLE="aarch64-apple-darwin"
    ;;
  x86_64)
    TARGET_TRIPLE="x86_64-apple-darwin"
    ;;
  *)
    echo "Unsupported macOS architecture: $HOST_ARCH" >&2
    exit 1
    ;;
esac

RUNTIME_DIR="$PYTHON_ROOT/$TARGET_TRIPLE"

if [[ $FORCE -eq 0 && -f "$MODEL_DIR/config.yaml" && -x "$RUNTIME_DIR/bin/python3" ]]; then
  echo "Bundled pyannote assets already present in $RESOURCE_ROOT"
  exit 0
fi

mkdir -p "$RESOURCE_ROOT" "$PYTHON_ROOT"
git lfs install >/dev/null

STAGE_DIR=$(mktemp -d)
trap 'rm -rf "$STAGE_DIR"' EXIT

STAGE_MODEL_DIR="$STAGE_DIR/model"
STAGE_RUNTIME_DIR="$STAGE_DIR/python/$TARGET_TRIPLE"

echo "Cloning offline pyannote model from $MODEL_REPO"
GIT_LFS_SKIP_SMUDGE=0 git clone --depth 1 "$MODEL_REPO" "$STAGE_MODEL_DIR"
(
  cd "$STAGE_MODEL_DIR"
  git lfs pull
)
rm -rf "$STAGE_MODEL_DIR/.git"
rm -f \
  "$STAGE_MODEL_DIR/.gitattributes" \
  "$STAGE_MODEL_DIR/README.md" \
  "$STAGE_MODEL_DIR/diarization.gif"

if [[ ! -f "$STAGE_MODEL_DIR/config.yaml" ]]; then
  echo "Downloaded model is missing config.yaml: $STAGE_MODEL_DIR" >&2
  exit 1
fi

echo "Creating bundled Python runtime for $TARGET_TRIPLE with Python $PYTHON_VERSION"
PYTHON_EXECUTABLE=$(resolve_python_executable "$PYTHON_VERSION")
echo "Using Python executable: $PYTHON_EXECUTABLE"
"$PYTHON_EXECUTABLE" -m venv --copies "$STAGE_RUNTIME_DIR"
"$STAGE_RUNTIME_DIR/bin/python" -m pip install --upgrade pip setuptools wheel >/dev/null
"$STAGE_RUNTIME_DIR/bin/python" -m pip install "pyannote.audio==4.0.4"

echo "Embedding Python standard library into the bundled runtime"
VERSION_DIR_NAME=$("$STAGE_RUNTIME_DIR/bin/python" - <<'PY'
import sys
print(f"python{sys.version_info.major}.{sys.version_info.minor}")
PY
)
SOURCE_STDLIB=$("$PYTHON_EXECUTABLE" - <<'PY'
import sysconfig
print(sysconfig.get_path("stdlib"))
PY
)
mkdir -p "$STAGE_RUNTIME_DIR/lib/$VERSION_DIR_NAME"
rsync -a --exclude 'site-packages' "$SOURCE_STDLIB/" "$STAGE_RUNTIME_DIR/lib/$VERSION_DIR_NAME/"
SOURCE_RESOURCES_DIR=$("$PYTHON_EXECUTABLE" - <<'PY'
import sys
from pathlib import Path

base = Path(sys.base_prefix)
candidates = [
    base / "Resources",
    base / "Frameworks" / "Python.framework" / "Versions" / f"{sys.version_info.major}.{sys.version_info.minor}" / "Resources",
]
for candidate in candidates:
    if candidate.is_dir():
        print(candidate)
        raise SystemExit(0)
PY
)
if [[ -n "$SOURCE_RESOURCES_DIR" ]]; then
  rsync -a "$SOURCE_RESOURCES_DIR/" "$STAGE_RUNTIME_DIR/lib/Resources/"
fi

echo "Embedding libpython into the bundled runtime"
LIBPY_SOURCE=$("$PYTHON_EXECUTABLE" - <<'PY'
import sys
import sysconfig
from pathlib import Path

version = f"{sys.version_info.major}.{sys.version_info.minor}"
ldlibrary = Path(sysconfig.get_config_var("LDLIBRARY") or "")
libdir = Path(sysconfig.get_config_var("LIBDIR") or "")
names = [f"libpython{version}.dylib"]
if ldlibrary.name and ldlibrary.name not in names:
    names.append(ldlibrary.name)

roots = [
    Path(sys.base_prefix),
    Path(sys.base_prefix).parent,
    Path(sys.base_prefix).parent.parent,
    libdir,
    libdir.parent,
]

seen = set()
for root in roots:
    if not root.exists():
        continue
    root = root.resolve()
    if root in seen:
        continue
    seen.add(root)
    for name in names:
        for match in root.rglob(name):
            if match.is_file():
                print(match)
                raise SystemExit(0)

raise SystemExit(
    f"Could not locate libpython via sysconfig (base_prefix={sys.base_prefix}, LIBDIR={libdir}, LDLIBRARY={ldlibrary})"
)
PY
)
LIBPY_TARGET="$STAGE_RUNTIME_DIR/lib/libpython${PYTHON_VERSION}.dylib"
cp "$LIBPY_SOURCE" "$LIBPY_TARGET"
install_name_tool -id "@rpath/$(basename "$LIBPY_TARGET")" "$LIBPY_TARGET"
PYTHON_BINARY_DEP=$(/usr/bin/otool -L "$STAGE_RUNTIME_DIR/bin/python3" | awk 'NR==2 {print $1}')
for python_bin in "$STAGE_RUNTIME_DIR"/bin/python*; do
  if [[ -x "$python_bin" ]]; then
    install_name_tool -add_rpath "@executable_path/../lib" "$python_bin" 2>/dev/null || true
    install_name_tool -change "$PYTHON_BINARY_DEP" "@rpath/$(basename "$LIBPY_TARGET")" "$python_bin" 2>/dev/null || true
    codesign --force --sign - "$python_bin" >/dev/null 2>&1 || true
  fi
done
rewrite_embedded_python_app_launcher "$STAGE_RUNTIME_DIR" "$LIBPY_TARGET"
codesign --force --sign - "$LIBPY_TARGET" >/dev/null 2>&1 || true
verify_no_external_python_framework_refs "$STAGE_RUNTIME_DIR"
echo "embedded $LIBPY_SOURCE -> $LIBPY_TARGET"

echo "Repairing Python config symlinks for standalone bundling"
STAGE_RUNTIME_DIR="$STAGE_RUNTIME_DIR" "$PYTHON_EXECUTABLE" - <<'PY'
import os
from pathlib import Path

runtime_root = Path(os.environ["STAGE_RUNTIME_DIR"])
lib_root = runtime_root / "lib"

for config_dir in lib_root.glob("python*/config-*-darwin"):
    if not config_dir.is_dir():
        continue
    for dylib_link in config_dir.glob("libpython*.dylib"):
        version = dylib_link.name.removeprefix("libpython").removesuffix(".dylib")
        target = Path("..") / ".." / f"libpython{version}.dylib"
        if dylib_link.exists() or dylib_link.is_symlink():
            dylib_link.unlink()
        dylib_link.symlink_to(target)
    for archive_link in config_dir.glob("libpython*.a"):
        if archive_link.exists() or archive_link.is_symlink():
            archive_link.unlink()
PY

echo "Removing pyvenv.cfg to keep the packaged runtime relocatable"
STAGE_RUNTIME_DIR="$STAGE_RUNTIME_DIR" "$PYTHON_EXECUTABLE" - <<'PY'
import os
from pathlib import Path

cfg_path = Path(os.environ["STAGE_RUNTIME_DIR"]) / "pyvenv.cfg"
if cfg_path.exists():
    cfg_path.unlink()
PY

echo "Verifying bundled pyannote runtime"
PYTHONHOME="$STAGE_RUNTIME_DIR" "$STAGE_RUNTIME_DIR/bin/python3" - <<PY
import ctypes
import csv
import encodings
from pyannote.audio import Pipeline
Pipeline.from_pretrained(r"$STAGE_MODEL_DIR")
print("pyannote pipeline loaded successfully")
PY

rm -rf "$RUNTIME_DIR"
mkdir -p "$(dirname "$RUNTIME_DIR")" "$MODEL_DIR"
rsync -a "$STAGE_RUNTIME_DIR/" "$RUNTIME_DIR/"
rsync -a --delete --exclude '.gitkeep' --exclude 'README.md' "$STAGE_MODEL_DIR/" "$MODEL_DIR/"

echo "Bundled pyannote runtime installed:"
echo "  runtime: $RUNTIME_DIR"
echo "  model:   $MODEL_DIR"
echo
echo "Next steps:"
echo "  1. Build the desktop app with 'cd apps/desktop && npm run tauri:build'"
echo "  2. On first launch, the app will auto-install these bundled assets into app data."
