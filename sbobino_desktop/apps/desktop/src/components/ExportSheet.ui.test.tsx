import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ExportSheet } from "./ExportSheet";

function renderExportSheet(): void {
  render(
    <ExportSheet
      open
      transcriptText="Hello world"
      segments={[
        { time: "00:00", line: "Hello world", speakerLabel: "Alice" },
      ]}
      title="Meeting"
      summary=""
      faqs=""
      onClose={vi.fn()}
      onExport={vi.fn().mockResolvedValue(true)}
    />,
  );
}

describe("ExportSheet options cleanup", () => {
  it("does not render orphan transcript options", () => {
    renderExportSheet();

    expect(screen.queryByText("Grouping")).not.toBeInTheDocument();
    expect(screen.queryByText("Speaker paragraphs")).not.toBeInTheDocument();
  });

  it("does not render orphan subtitles/segments options", () => {
    renderExportSheet();

    fireEvent.click(screen.getAllByRole("button", { name: /Subtitles/i })[0]);
    expect(
      screen.queryByText("Favorited Segments Only"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("Allow multiple lines")).not.toBeInTheDocument();
    expect(screen.queryByText("Use Original File Name")).not.toBeInTheDocument();

    fireEvent.click(screen.getAllByRole("button", { name: /Segments/i })[0]);
    expect(
      screen.queryByText("Favorited Segments Only"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("Allow multiple lines")).not.toBeInTheDocument();
    expect(screen.queryByText("Use Original File Name")).not.toBeInTheDocument();
  });
});
