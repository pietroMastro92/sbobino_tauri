import { describe, expect, it } from "vitest";
import { useAppStore } from "./useAppStore";
import type { TranscriptArtifact } from "../types";

const artifact = (id: string, title: string): TranscriptArtifact => ({
  id,
  job_id: `job-${id}`,
  title,
  kind: "file",
  input_path: `/tmp/${title}.wav`,
  raw_transcript: "raw",
  optimized_transcript: "optimized",
  summary: "",
  faqs: "",
  metadata: {},
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
});

describe("useAppStore", () => {
  it("tracks active job lifecycle", () => {
    const store = useAppStore.getState();
    store.setJobStarted("job-123");
    expect(useAppStore.getState().activeJobId).toBe("job-123");

    store.clearActiveJob();
    expect(useAppStore.getState().activeJobId).toBeNull();
  });

  it("prepends, upserts and removes artifacts", () => {
    const store = useAppStore.getState();
    store.setArtifacts([]);

    const first = artifact("a1", "first");
    const second = artifact("a2", "second");
    store.prependArtifact(first);
    store.prependArtifact(second);
    expect(useAppStore.getState().artifacts.map((item) => item.id)).toEqual(["a2", "a1"]);

    store.upsertArtifact({ ...first, title: "first-updated" });
    expect(useAppStore.getState().artifacts.find((item) => item.id === "a1")?.title).toBe(
      "first-updated",
    );

    store.removeArtifacts(["a2"]);
    expect(useAppStore.getState().artifacts.map((item) => item.id)).toEqual(["a1"]);
  });
});
