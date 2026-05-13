import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const appSource = fs.readFileSync(path.resolve(currentDir, "../App.tsx"), "utf8");

describe("automatic import settings UI", () => {
  it("uses visible switch controls for automatic import toggles", () => {
    expect(appSource).toContain("renderAutomationSwitch(");
    expect(appSource).toContain('role="switch"');
    expect(appSource).toContain("enabled: !current.enabled");
    expect(appSource).toContain(
      "run_scan_on_app_start: !current.run_scan_on_app_start",
    );
  });

  it("keeps model and language selectors on each watched folder", () => {
    expect(appSource).toContain("settings.automaticImport.sourceModel");
    expect(appSource).toContain("settings.automaticImport.sourceLanguage");
    expect(appSource).toContain("model: event.target.value as SpeechModel");
    expect(appSource).toContain(
      "language: event.target.value as LanguageCode",
    );
  });
});
