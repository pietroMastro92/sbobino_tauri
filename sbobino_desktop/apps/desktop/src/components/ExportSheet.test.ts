import { describe, expect, it } from "vitest";

import { buildPreviewContent } from "./ExportSheet";

describe("buildPreviewContent", () => {
  it("includes speaker names in segments preview when requested", () => {
    const preview = buildPreviewContent({
      transcriptText: "Fallback transcript",
      segments: [
        { time: "00:12", line: "Alice opens the meeting.", speakerLabel: "Alice" },
        { time: "00:24", line: "Bob confirms the next step.", speakerLabel: "Bob" },
      ],
      style: "segments",
      format: "txt",
      includeTimestamps: true,
      includeSpeakerNames: true,
      language: "en",
      title: "Meeting",
    });

    expect(preview).toContain("[00:12] Alice: Alice opens the meeting.");
    expect(preview).toContain("[00:24] Bob: Bob confirms the next step.");
  });

  it("adds the speaker column to segments csv preview when requested", () => {
    const preview = buildPreviewContent({
      transcriptText: "Fallback transcript",
      segments: [
        { time: "00:12", line: "Alice opens the meeting.", speakerLabel: "Alice" },
      ],
      style: "segments",
      format: "csv",
      includeTimestamps: true,
      includeSpeakerNames: true,
      language: "en",
      title: "Meeting",
    });

    expect(preview).toContain("Start Timestamp;End Timestamp;Transcript;Speaker");
    expect(preview).toContain("00:12;00:23;\"Alice opens the meeting.\";\"Alice\"");
  });
});
