import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const appSource = fs.readFileSync(path.resolve(currentDir, "../App.tsx"), "utf8");

function extractFunction(source: string, name: string): string {
  const start = source.indexOf(`function ${name}`);
  if (start < 0) return "";
  const bodyStart = source.indexOf("{", start);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") depth += 1;
    if (char === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, index + 1);
    }
  }
  return "";
}

describe("queue UI wiring", () => {
  it("allows manual multi-file selection", () => {
    const onPickFile = extractFunction(appSource, "onPickFile");

    expect(onPickFile).toContain("multiple: true");
    expect(onPickFile).toContain("enqueueTranscriptionStartBatch");
  });

  it("does not clear active or queued jobs from Clear Finished", () => {
    expect(appSource).toContain("clearFinishedQueueItems(previous)");
    expect(appSource).not.toContain("onClick={() => setQueueItems([])}");
  });

  it("keeps completed jobs visible in the queue", () => {
    expect(appSource).toContain("markQueueItemTerminal(");
    expect(appSource).toContain('stage: "completed"');
    expect(appSource).toContain("const queueActiveItems = useMemo(\n    () => queueItems");
  });
});
