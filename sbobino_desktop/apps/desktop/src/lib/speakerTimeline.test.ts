import { describe, expect, it } from "vitest";

import {
  mergeSpeakerInTimeline,
  removeSpeakerFromTimeline,
  renameSpeakerInTimeline,
} from "./speakerTimeline";

describe("renameSpeakerInTimeline", () => {
  it("renames every segment that shares the selected speaker id", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "Hi", speaker_id: "speaker_1", speaker_label: "Speaker 1" },
        { text: "Hello", speaker_id: "speaker_2", speaker_label: "Speaker 2" },
        { text: "Bye", speaker_id: "speaker_1", speaker_label: "Speaker 1" },
      ],
    });

    const result = renameSpeakerInTimeline(timeline, 0, "Pietro");

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.previousSpeakerLabel).toBe("Speaker 1");
    expect(result.renamedCount).toBe(2);
    expect(result.timeline.segments[0].speaker_label).toBe("Pietro");
    expect(result.timeline.segments[0].speaker_id).toBe("pietro");
    expect(result.timeline.segments[2].speaker_label).toBe("Pietro");
    expect(result.timeline.segments[2].speaker_id).toBe("pietro");
    expect(result.timeline.segments[1].speaker_label).toBe("Speaker 2");
  });

  it("falls back to speaker label when speaker id is missing", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "A", speaker_label: "Speaker 1" },
        { text: "B", speaker_label: "Speaker 1" },
        { text: "C", speaker_label: "Speaker 2" },
      ],
    });

    const result = renameSpeakerInTimeline(timeline, 1, "Luna");

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.renamedCount).toBe(2);
    expect(result.timeline.segments[0].speaker_label).toBe("Luna");
    expect(result.timeline.segments[0].speaker_id).toBe("luna");
    expect(result.timeline.segments[1].speaker_label).toBe("Luna");
    expect(result.timeline.segments[2].speaker_label).toBe("Speaker 2");
  });

  it("rejects rename when the selected segment has no speaker yet", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "No label yet" },
      ],
    });

    const result = renameSpeakerInTimeline(timeline, 0, "Pietro");

    expect(result).toEqual({
      ok: false,
      reason: "speaker_missing",
    });
  });
});

describe("mergeSpeakerInTimeline", () => {
  it("reassigns every segment from one speaker to another", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "Hi", speaker_id: "speaker_1", speaker_label: "Speaker 1" },
        { text: "Hello", speaker_id: "speaker_2", speaker_label: "Speaker 2" },
        { text: "Bye", speaker_id: "speaker_1", speaker_label: "Speaker 1" },
      ],
    });

    const result = mergeSpeakerInTimeline(
      timeline,
      "speaker_1",
      "Speaker 1",
      "speaker_2",
      "Speaker 2",
    );

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.mergedCount).toBe(2);
    expect(result.timeline.segments[0].speaker_label).toBe("Speaker 2");
    expect(result.timeline.segments[0].speaker_id).toBe("speaker_2");
    expect(result.timeline.segments[2].speaker_label).toBe("Speaker 2");
    expect(result.timeline.segments[2].speaker_id).toBe("speaker_2");
  });

  it("falls back to speaker labels when source segments have no speaker id", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "A", speaker_label: "Speaker 1" },
        { text: "B", speaker_label: "Speaker 3" },
        { text: "C", speaker_label: "Speaker 1" },
      ],
    });

    const result = mergeSpeakerInTimeline(
      timeline,
      "speaker_1",
      "Speaker 1",
      "speaker_3",
      "Speaker 3",
    );

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.mergedCount).toBe(2);
    expect(result.timeline.segments[0].speaker_label).toBe("Speaker 3");
    expect(result.timeline.segments[0].speaker_id).toBe("speaker_3");
    expect(result.timeline.segments[2].speaker_label).toBe("Speaker 3");
    expect(result.timeline.segments[2].speaker_id).toBe("speaker_3");
  });

  it("rejects merging a speaker into itself", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "A", speaker_id: "speaker_1", speaker_label: "Speaker 1" },
      ],
    });

    const result = mergeSpeakerInTimeline(
      timeline,
      "speaker_1",
      "Speaker 1",
      "speaker_1",
      "Speaker 1",
    );

    expect(result).toEqual({
      ok: false,
      reason: "same_speaker",
    });
  });
});

describe("removeSpeakerFromTimeline", () => {
  it("removes the selected speaker from all matching segments", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "Hi", speaker_id: "speaker_1", speaker_label: "Speaker 1" },
        { text: "Hello", speaker_id: "speaker_2", speaker_label: "Speaker 2" },
        { text: "Bye", speaker_id: "speaker_1", speaker_label: "Speaker 1" },
      ],
    });

    const result = removeSpeakerFromTimeline(
      timeline,
      "speaker_1",
      "Speaker 1",
    );

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.removedCount).toBe(2);
    expect(result.timeline.segments[0]).not.toHaveProperty("speaker_id");
    expect(result.timeline.segments[0]).not.toHaveProperty("speaker_label");
    expect(result.timeline.segments[2]).not.toHaveProperty("speaker_id");
    expect(result.timeline.segments[2]).not.toHaveProperty("speaker_label");
    expect(result.timeline.segments[1].speaker_label).toBe("Speaker 2");
  });

  it("falls back to speaker labels when ids are missing", () => {
    const timeline = JSON.stringify({
      version: 2,
      segments: [
        { text: "A", speaker_label: "Speaker 1" },
        { text: "B", speaker_label: "Speaker 2" },
        { text: "C", speaker_label: "Speaker 1" },
      ],
    });

    const result = removeSpeakerFromTimeline(
      timeline,
      "speaker_1",
      "Speaker 1",
    );

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.removedCount).toBe(2);
    expect(result.timeline.segments[0]).not.toHaveProperty("speaker_label");
    expect(result.timeline.segments[2]).not.toHaveProperty("speaker_label");
    expect(result.timeline.segments[1].speaker_label).toBe("Speaker 2");
  });
});
