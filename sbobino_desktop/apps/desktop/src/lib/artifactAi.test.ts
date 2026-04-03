import { describe, expect, it } from "vitest";

import {
  aiActionsAvailable,
  buildChatArtifactPayload,
  buildEmotionAnalysisPayload,
  buildSummaryArtifactPayload,
  defaultEmotionControls,
  defaultSummaryControls,
  normalizeEmotionNarrative,
  parsePersistedEmotionAnalysis,
  shouldAutostartSummary,
} from "./artifactAi";

describe("artifactAi helpers", () => {
  it("builds summary payloads from inspector controls", () => {
    expect(buildSummaryArtifactPayload({
      id: "artifact-1",
      language: "it",
      includeTimestamps: true,
      includeSpeakers: false,
      sections: true,
      bulletPoints: true,
      actionItems: false,
      keyPointsOnly: true,
      customPrompt: "  Focus on decisions only.  ",
    })).toEqual({
      id: "artifact-1",
      language: "it",
      include_timestamps: true,
      include_speakers: false,
      sections: true,
      bullet_points: true,
      action_items: false,
      key_points_only: true,
      custom_prompt: "Focus on decisions only.",
    });
  });

  it("normalizes empty custom summary prompts to null", () => {
    expect(buildSummaryArtifactPayload({
      id: "artifact-1",
      language: "en",
      includeTimestamps: false,
      includeSpeakers: true,
      sections: false,
      bulletPoints: false,
      actionItems: true,
      keyPointsOnly: false,
      customPrompt: "   ",
    }).custom_prompt).toBeNull();
  });

  it("exposes detailed summary defaults", () => {
    expect(defaultSummaryControls).toEqual({
      includeTimestamps: false,
      includeSpeakers: false,
      sections: true,
      bulletPoints: false,
      actionItems: true,
      keyPointsOnly: false,
      language: "en",
    });
  });

  it("builds chat payloads that preserve context toggles", () => {
    expect(buildChatArtifactPayload({
      id: "artifact-2",
      prompt: "  What were the next steps?  ",
      includeTimestamps: false,
      includeSpeakers: true,
    })).toEqual({
      id: "artifact-2",
      prompt: "What were the next steps?",
      include_timestamps: false,
      include_speakers: true,
    });
  });

  it("builds emotion-analysis payloads from inspector controls", () => {
    expect(buildEmotionAnalysisPayload({
      id: "artifact-4",
      language: "it",
      includeTimestamps: true,
      includeSpeakers: true,
      speakerDynamics: false,
    })).toEqual({
      id: "artifact-4",
      language: "it",
      include_timestamps: true,
      include_speakers: true,
      speaker_dynamics: false,
    });
  });

  it("exposes emotion-analysis defaults", () => {
    expect(defaultEmotionControls).toEqual({
      includeTimestamps: true,
      includeSpeakers: false,
      speakerDynamics: true,
      language: "en",
    });
  });

  it("parses persisted emotion analysis from artifact metadata", () => {
    const result = parsePersistedEmotionAnalysis({
      id: "artifact-5",
      job_id: "job-1",
      title: "Emotion test",
      kind: "file",
      source_label: "/tmp/demo.wav",
      source_origin: "imported",
      audio_available: true,
      audio_backfill_status: "imported",
      revision: 1,
      raw_transcript: "raw",
      optimized_transcript: "",
      summary: "",
      faqs: "",
      metadata: {
        emotion_analysis_v1: JSON.stringify({
          overview: {
            primary_emotions: ["joy"],
            emotional_arc: "Starts worried, ends calmer.",
          },
          timeline: [],
          semantic_map: { nodes: [], edges: [], clusters: [] },
          bridges: [],
          reflection_prompts: ["What shifted?"],
          narrative_markdown: "Narrative",
        }),
      },
      parent_artifact_id: null,
      processing_engine: "whisper_cpp",
      processing_model: "base",
      processing_language: "en",
      audio_duration_seconds: 12,
      audio_byte_size: 1024,
      created_at: "",
      updated_at: "",
    });

    expect(result?.overview.primary_emotions).toEqual(["joy"]);
    expect(result?.narrative_markdown).toBe("Narrative");
  });

  it("drops emotion narratives that look like serialized payloads", () => {
    expect(
      normalizeEmotionNarrative(`{
        "overview": {"primary_emotions":["joy"]},
        "timeline": [],
        "semantic_map": {"nodes": [], "edges": [], "clusters": []},
        "bridges": [],
        "reflection_prompts": [],
        "narrative_markdown": "Narrative"
      }`),
    ).toBe("");
  });

  it("keeps readable emotion narratives", () => {
    expect(
      normalizeEmotionNarrative("## Emotional reading\n\nThe conversation starts tense and softens near the end."),
    ).toBe("## Emotional reading\n\nThe conversation starts tense and softens near the end.");
  });

  it("autostarts only once for empty summaries on ready artifacts", () => {
    expect(shouldAutostartSummary({
      enabled: true,
      artifactId: "artifact-3",
      persistedSummary: "",
      draftSummary: "",
      hasActiveJob: false,
      isGeneratingSummary: false,
      triggeredArtifactIds: new Set<string>(),
    })).toBe(true);

    expect(shouldAutostartSummary({
      enabled: true,
      artifactId: "artifact-3",
      persistedSummary: "",
      draftSummary: "",
      hasActiveJob: false,
      isGeneratingSummary: false,
      triggeredArtifactIds: new Set(["artifact-3"]),
    })).toBe(false);

    expect(shouldAutostartSummary({
      enabled: true,
      artifactId: "artifact-3",
      persistedSummary: "Existing summary",
      draftSummary: "",
      hasActiveJob: false,
      isGeneratingSummary: false,
      triggeredArtifactIds: new Set<string>(),
    })).toBe(false);
  });

  it("reports AI actions availability from backend capability status", () => {
    expect(aiActionsAvailable(null)).toBe(false);
    expect(aiActionsAvailable({
      available: false,
      fallback_available: false,
      unavailable_reason: "No provider",
    })).toBe(false);
    expect(aiActionsAvailable({
      available: true,
      fallback_available: true,
      unavailable_reason: null,
    })).toBe(true);
  });
});
