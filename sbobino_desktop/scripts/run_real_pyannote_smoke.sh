#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <input-audio> [seconds]" >&2
  exit 1
fi

INPUT_AUDIO=$1
CLIP_SECONDS=${2:-180}

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
FFMPEG_BIN="$ROOT_DIR/apps/desktop/src-tauri/binaries/ffmpeg-aarch64-apple-darwin"
WHISPER_BIN="$ROOT_DIR/apps/desktop/src-tauri/binaries/whisper-cli-aarch64-apple-darwin"
WHISPER_MODEL="$HOME/Library/Application Support/com.sbobino.desktop/models/ggml-base.bin"
PYANNOTE_PYTHON="$ROOT_DIR/apps/desktop/src-tauri/resources/pyannote/python/aarch64-apple-darwin/bin/python3"
PYANNOTE_SCRIPT="$ROOT_DIR/scripts/pyannote_diarize.py"
PYANNOTE_MODEL="$ROOT_DIR/apps/desktop/src-tauri/resources/pyannote/model"

for path in "$INPUT_AUDIO" "$FFMPEG_BIN" "$WHISPER_BIN" "$WHISPER_MODEL" "$PYANNOTE_PYTHON" "$PYANNOTE_SCRIPT" "$PYANNOTE_MODEL"; do
  if [[ ! -e "$path" ]]; then
    echo "Missing required path: $path" >&2
    exit 1
  fi
done

RUN_DIR="/tmp/sbobino-real-pyannote-$(date +%s)"
mkdir -p "$RUN_DIR"

python3 - <<PY
import json
import os
import subprocess
from pathlib import Path

input_audio = Path(r"""$INPUT_AUDIO""")
clip_seconds = int(r"""$CLIP_SECONDS""")
ffmpeg = Path(r"""$FFMPEG_BIN""")
whisper = Path(r"""$WHISPER_BIN""")
model = Path(r"""$WHISPER_MODEL""")
py_python = Path(r"""$PYANNOTE_PYTHON""")
py_script = Path(r"""$PYANNOTE_SCRIPT""")
py_model = Path(r"""$PYANNOTE_MODEL""")
run_dir = Path(r"""$RUN_DIR""")

clip = run_dir / "clip.wav"
whisper_base = run_dir / "whisper"
whisper_txt = whisper_base.with_suffix(".txt")
whisper_json = whisper_base.with_suffix(".json")
py_json = run_dir / "pyannote.json"
mapped_json = run_dir / "mapped_segments.json"

print(f"run_dir={run_dir}")
print("step=extract_audio")
subprocess.run(
    [
        str(ffmpeg),
        "-y",
        "-i",
        str(input_audio),
        "-t",
        str(clip_seconds),
        "-ac",
        "1",
        "-ar",
        "16000",
        str(clip),
    ],
    check=True,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)

print("step=whisper_transcribe")
env = os.environ.copy()
bin_dir = str(whisper.parent)
env["DYLD_LIBRARY_PATH"] = bin_dir + (
    ":" + env["DYLD_LIBRARY_PATH"] if env.get("DYLD_LIBRARY_PATH") else ""
)
subprocess.run(
    [
        str(whisper),
        "-m",
        str(model),
        "-f",
        str(clip),
        "-l",
        "auto",
        "-t",
        "8",
        "-p",
        "1",
        "-otxt",
        "-ojf",
        "-pp",
        "-pc",
        "-of",
        str(whisper_base),
    ],
    env=env,
    check=True,
)

print("step=pyannote_diarize")
env2 = os.environ.copy()
env2["PATH"] = str(ffmpeg.parent) + os.pathsep + env2.get("PATH", "")
result = subprocess.run(
    [
        str(py_python),
        str(py_script),
        "--audio-path",
        str(clip),
        "--model-path",
        str(py_model),
        "--device",
        "cpu",
    ],
    env=env2,
    capture_output=True,
    text=True,
    check=True,
)
py_payload = json.loads(result.stdout)
py_json.write_text(json.dumps(py_payload, indent=2), encoding="utf-8")

print("step=map_segments")
whisper_payload = json.loads(whisper_json.read_text(encoding="utf-8"))
segments = []
for seg in whisper_payload.get("transcription", []):
    text = (seg.get("text") or "").strip()
    if not text:
        continue
    offsets = seg.get("offsets") or {}
    start = offsets.get("from")
    end = offsets.get("to")
    start_s = None if start is None or start < 0 else start / 1000.0
    end_s = None if end is None or end < 0 else end / 1000.0
    best = None
    best_overlap = -1.0
    if start_s is not None and end_s is not None:
        for turn in py_payload.get("speakers", []):
            overlap = max(
                0.0,
                min(end_s, turn["end_seconds"]) - max(start_s, turn["start_seconds"]),
            )
            if overlap > best_overlap:
                best_overlap = overlap
                best = turn
    segments.append(
        {
            "start_seconds": start_s,
            "end_seconds": end_s,
            "text": text,
            "speaker_id": best.get("speaker_id") if best else None,
            "speaker_label": best.get("speaker_label") if best else None,
            "overlap_seconds": best_overlap if best_overlap > 0 else 0.0,
        }
    )

mapped_json.write_text(json.dumps(segments, indent=2), encoding="utf-8")

print("step=summary")
print(f"input_audio={input_audio}")
print(f"clip_seconds={clip_seconds}")
print(f"whisper_segments={len(segments)}")
print(f"pyannote_turns={len(py_payload.get('speakers', []))}")
unique_speakers = sorted({turn["speaker_label"] for turn in py_payload.get("speakers", [])})
print("speakers=" + ", ".join(unique_speakers))
print("transcript_preview=")
for seg in segments[:12]:
    start = 0.0 if seg["start_seconds"] is None else seg["start_seconds"]
    end = 0.0 if seg["end_seconds"] is None else seg["end_seconds"]
    label = seg["speaker_label"] or "Unknown"
    text = seg["text"].replace("\n", " ").strip()
    print(f"[{start:7.2f}-{end:7.2f}] {label}: {text}")

print("artifacts=")
for path in [clip, whisper_txt, whisper_json, py_json, mapped_json]:
    print(path)

if result.stderr.strip():
    print("pyannote_stderr=")
    print(result.stderr[:2000])
PY
