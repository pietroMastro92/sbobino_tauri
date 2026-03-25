import type { CSSProperties } from "react";

const ANSI_SEQUENCE_PATTERN =
  /(?:\\u001b\[|\\x1b\[|[\u001b\u009b\uFFFD]*\[)([0-9;]*)m/gi;

function ansiIndexedColorToCss(code: number): string {
  const baseColors = [
    "#1f2328",
    "#cc3f33",
    "#2f8f4e",
    "#b98917",
    "#4c7dd9",
    "#8e59d1",
    "#1591a6",
    "#c7d0d9",
    "#6b7280",
    "#ff6b5b",
    "#4ac26b",
    "#f5c542",
    "#79a6ff",
    "#b27cff",
    "#3dc1d5",
    "#f4f7fb",
  ];

  if (code >= 0 && code < baseColors.length) {
    return baseColors[code];
  }

  if (code >= 16 && code <= 231) {
    const index = code - 16;
    const red = Math.floor(index / 36);
    const green = Math.floor((index % 36) / 6);
    const blue = index % 6;
    const channelToHex = (value: number) => {
      const normalized = value === 0 ? 0 : 55 + value * 40;
      return normalized.toString(16).padStart(2, "0");
    };
    return `#${channelToHex(red)}${channelToHex(green)}${channelToHex(blue)}`;
  }

  if (code >= 232 && code <= 255) {
    const shade = 8 + (code - 232) * 10;
    const hex = shade.toString(16).padStart(2, "0");
    return `#${hex}${hex}${hex}`;
  }

  return "var(--text)";
}

export function stripAnsi(value: string): string {
  return value.replace(new RegExp(ANSI_SEQUENCE_PATTERN), "");
}

export function parseAnsiTextSegments(
  value: string,
): Array<{ text: string; style: CSSProperties }> {
  const segments: Array<{ text: string; style: CSSProperties }> = [];
  const ansiPattern = new RegExp(ANSI_SEQUENCE_PATTERN);
  let currentStyle: CSSProperties = {};
  let lastIndex = 0;

  const pushChunk = (chunk: string) => {
    if (!chunk) {
      return;
    }
    segments.push({ text: chunk, style: { ...currentStyle } });
  };

  for (let match = ansiPattern.exec(value); match; match = ansiPattern.exec(value)) {
    pushChunk(value.slice(lastIndex, match.index));

    const codes = (match[1] || "0")
      .split(";")
      .map((token) => Number(token || "0"))
      .filter((code) => Number.isFinite(code));

    for (let index = 0; index < codes.length; index += 1) {
      const code = codes[index];

      if (code === 0) {
        currentStyle = {};
        continue;
      }
      if (code === 1) {
        currentStyle.fontWeight = 600;
        continue;
      }
      if (code === 2) {
        currentStyle.opacity = 0.72;
        continue;
      }
      if (code === 4) {
        currentStyle.textDecoration = "underline";
        continue;
      }
      if (code === 22) {
        delete currentStyle.fontWeight;
        delete currentStyle.opacity;
        continue;
      }
      if (code === 24) {
        delete currentStyle.textDecoration;
        continue;
      }
      if (code === 39) {
        delete currentStyle.color;
        continue;
      }
      if (code === 49) {
        delete currentStyle.backgroundColor;
        continue;
      }
      if (code >= 30 && code <= 37) {
        currentStyle.color = ansiIndexedColorToCss(code - 30);
        continue;
      }
      if (code >= 90 && code <= 97) {
        currentStyle.color = ansiIndexedColorToCss(code - 90 + 8);
        continue;
      }
      if (code >= 40 && code <= 47) {
        currentStyle.backgroundColor = ansiIndexedColorToCss(code - 40);
        continue;
      }
      if (code >= 100 && code <= 107) {
        currentStyle.backgroundColor = ansiIndexedColorToCss(code - 100 + 8);
        continue;
      }
      if (code === 38 && codes[index + 1] === 5 && codes[index + 2] !== undefined) {
        currentStyle.color = ansiIndexedColorToCss(codes[index + 2]);
        index += 2;
        continue;
      }
      if (code === 48 && codes[index + 1] === 5 && codes[index + 2] !== undefined) {
        currentStyle.backgroundColor = ansiIndexedColorToCss(codes[index + 2]);
        index += 2;
      }
    }

    lastIndex = match.index + match[0].length;
  }

  pushChunk(value.slice(lastIndex));
  return segments;
}
