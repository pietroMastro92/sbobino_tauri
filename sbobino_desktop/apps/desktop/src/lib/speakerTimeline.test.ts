import { describe, expect, it } from "vitest";

import { renameSpeakerInTimeline } from "./speakerTimeline";

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
