import { describe, expect, it } from "vitest";

import { parseAnsiTextSegments, stripAnsi } from "./ansiText";

describe("ansiText helpers", () => {
  it("strips standard ANSI color markers", () => {
    const input = "\u001b[38;5;71mhello\u001b[0m \u001b[38;5;114mworld\u001b[0m";

    expect(stripAnsi(input)).toBe("hello world");
  });

  it("strips degraded ANSI markers like the live preview screenshot", () => {
    const input = "\uFFFD[38;5;71mciao\uFFFD[0m [38;5;114mmondo[0m";

    expect(stripAnsi(input)).toBe("ciao mondo");
  });

  it("parses degraded ANSI markers into styled segments", () => {
    const input = "[38;5;71mverde[0m normale \uFFFD[38;5;166marancio\uFFFD[0m";
    const segments = parseAnsiTextSegments(input);

    expect(segments.map((segment) => segment.text).join("")).toBe("verde normale arancio");
    expect(segments[0]?.style.color).toBeTruthy();
    expect(segments[1]?.text).toBe(" normale ");
    expect(segments[2]?.style.color).toBeTruthy();
  });
});
