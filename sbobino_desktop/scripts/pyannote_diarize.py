#!/usr/bin/env python3

import argparse
import json
import sys
from typing import Dict, List


def resolve_device(requested: str):
    import torch

    value = (requested or "cpu").strip().lower()
    if value == "auto":
        if torch.backends.mps.is_available():
            return torch.device("mps")
        if torch.cuda.is_available():
            return torch.device("cuda")
        return torch.device("cpu")
    if value == "mps" and torch.backends.mps.is_available():
        return torch.device("mps")
    if value == "cuda" and torch.cuda.is_available():
        return torch.device("cuda")
    return torch.device("cpu")


def resolve_annotation(diarization):
    if hasattr(diarization, "exclusive_speaker_diarization"):
        annotation = diarization.exclusive_speaker_diarization
        if annotation is not None:
            return annotation

    if hasattr(diarization, "speaker_diarization"):
        annotation = diarization.speaker_diarization
        if annotation is not None:
            return annotation

    return diarization


def main() -> int:
    parser = argparse.ArgumentParser(description="Run speaker diarization with pyannote.audio")
    parser.add_argument("--audio-path", required=True)
    parser.add_argument("--model-path", required=True)
    parser.add_argument("--device", default="cpu")
    args = parser.parse_args()

    try:
        import torch
        from pyannote.audio import Pipeline
    except Exception as error:
        sys.stderr.write(
            "pyannote dependencies are not available. Install torch and pyannote.audio in the configured Python environment.\n"
        )
        sys.stderr.write(f"{error}\n")
        return 1

    try:
        pipeline = Pipeline.from_pretrained(args.model_path)
        pipeline.to(resolve_device(args.device))
        diarization = pipeline(args.audio_path)
    except Exception as error:
        sys.stderr.write(f"pyannote inference failed: {error}\n")
        return 1

    annotation = resolve_annotation(diarization)
    if not hasattr(annotation, "itertracks"):
        sys.stderr.write(
            f"pyannote inference returned unsupported annotation type: {type(annotation).__name__}\n"
        )
        return 1

    speaker_order: Dict[str, int] = {}
    turns: List[dict] = []

    for turn, _, backend_speaker in annotation.itertracks(yield_label=True):
        if backend_speaker not in speaker_order:
            speaker_order[backend_speaker] = len(speaker_order) + 1
        index = speaker_order[backend_speaker]
        turns.append(
            {
                "speaker_id": f"speaker_{index}",
                "speaker_label": f"Speaker {index}",
                "start_seconds": float(turn.start),
                "end_seconds": float(turn.end),
                "backend_speaker": backend_speaker,
            }
        )

    turns.sort(key=lambda item: (item["start_seconds"], item["end_seconds"]))
    sys.stdout.write(json.dumps({"speakers": turns}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
