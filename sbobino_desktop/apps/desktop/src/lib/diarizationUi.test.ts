import { describe, expect, it } from "vitest";

import type { TranscriptArtifact } from "../types";
import {
  getArtifactDiarizationUiState,
  normalizeJobFailureMessage,
} from "./diarizationUi";

function artifact(metadata: Record<string, string> = {}): TranscriptArtifact {
  return {
    id: "artifact-1",
    job_id: "job-1",
    title: "Example",
    kind: "file",
    input_path: "/tmp/example.wav",
    raw_transcript: "raw",
    optimized_transcript: "",
    summary: "",
    faqs: "",
    metadata,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  };
}

describe("normalizeJobFailureMessage", () => {
  it("preserves backend failure text when present", () => {
    expect(
      normalizeJobFailureMessage("input file not found: /tmp/missing.wav"),
    ).toBe("input file not found: /tmp/missing.wav");
  });

  it("falls back for empty failure text", () => {
    expect(normalizeJobFailureMessage("   ", "Fallback")).toBe("Fallback");
  });
});

describe("getArtifactDiarizationUiState", () => {
  it("detects labeled speakers from timeline-derived labels", () => {
    expect(
      getArtifactDiarizationUiState(artifact(), ["Speaker 2", "Speaker 1", "Speaker 2"]),
    ).toEqual({
      kind: "speakers_detected",
      speakerCount: 2,
      speakerLabels: ["Speaker 2", "Speaker 1"],
      error: null,
    });
  });

  it("reports failed diarization from artifact metadata", () => {
    expect(
      getArtifactDiarizationUiState(
        artifact({
          speaker_diarization_status: "failed",
          speaker_diarization_error: "pyannote helper failed",
        }),
        [],
      ),
    ).toEqual({
      kind: "failed",
      speakerCount: 0,
      speakerLabels: [],
      error: "pyannote helper failed",
    });
  });

  it("reports missing speaker labels as not requested when no metadata exists", () => {
    expect(getArtifactDiarizationUiState(artifact(), [])).toEqual({
      kind: "not_requested",
      speakerCount: 0,
      speakerLabels: [],
      error: null,
    });
  });
});
