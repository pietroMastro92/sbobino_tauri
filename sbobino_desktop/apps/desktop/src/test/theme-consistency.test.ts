import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const cssPath = path.resolve(currentDir, "../styles.css");
const cssSource = fs.readFileSync(cssPath, "utf8");

function extractBlock(source: string, selector: string): string {
  const startToken = `${selector} {`;
  const startIndex = source.indexOf(startToken);
  if (startIndex < 0) {
    return "";
  }

  const blockStart = source.indexOf("{", startIndex);
  let depth = 0;
  for (let index = blockStart; index < source.length; index += 1) {
    const ch = source[index];
    if (ch === "{") depth += 1;
    if (ch === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(blockStart + 1, index);
      }
    }
  }
  return "";
}

describe("theme consistency", () => {
  it("keeps settings/main topbar styling split correctly for light and dark modes", () => {
    expect(cssSource).toContain('[data-theme="light"] .main-topbar');
    expect(cssSource).toContain('[data-theme="light"] .detail-toolbar');
    expect(cssSource).toContain('[data-theme="light"] .settings-window-header');
    expect(cssSource).toContain('[data-theme="dark"] .main-topbar');
    expect(cssSource).toContain('[data-theme="dark"] .detail-toolbar');
    expect(cssSource).toContain('[data-theme="dark"] .settings-window-header');

    const darkThemeVariables = extractBlock(cssSource, '[data-theme="dark"]');
    expect(darkThemeVariables).toContain("--topbar-bg: transparent;");
    expect(darkThemeVariables).not.toContain("--topbar-bg: rgba(248, 251, 255, 0.44);");
  });

  it("uses palette variables for active/selected controls without hardcoded light accents", () => {
    const selectors = [
      ".home-history-item.selected",
      ".history-item.selected",
      ".queue-card-clickable:focus-visible",
      ".queue-progress > div",
      ".inline-progress > div",
      ".icon-button.active",
      ".audio-slider",
    ];
    const forbiddenTokens = [
      "#4ea2e0",
      "#2f7bc0",
      "#57afea",
      "#347dc2",
      "#6D94C5",
      "#5478a8",
      "#3f92d6",
    ];

    for (const selector of selectors) {
      const block = extractBlock(cssSource, selector);
      expect(block, `Missing block for ${selector}`).not.toBe("");
      for (const token of forbiddenTokens) {
        expect(block).not.toContain(token);
      }
    }

    expect(extractBlock(cssSource, ".home-history-item.selected")).toContain("var(--detail-active-border)");
    expect(extractBlock(cssSource, ".history-item.selected")).toContain("var(--detail-active-border)");
    expect(extractBlock(cssSource, ".queue-progress > div")).toContain("var(--accent)");
    expect(extractBlock(cssSource, ".queue-progress > div")).toContain("var(--accent-strong)");
    expect(extractBlock(cssSource, ".audio-slider")).toContain("var(--accent)");
  });
});

