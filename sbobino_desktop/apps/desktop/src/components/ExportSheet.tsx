import { Captions, Download, FileText, FileType2, FileType, List } from "lucide-react";
import { useMemo, useState } from "react";

export type ExportFormat = "txt" | "docx" | "pdf";
export type ExportStyle = "transcript" | "subtitles" | "segments";

type ExportSegment = {
  time: string;
  line: string;
};

type ExportSheetProps = {
  open: boolean;
  title: string;
  transcriptText: string;
  segments: ExportSegment[];
  onClose: () => void;
  onExport: (format: ExportFormat, contentOverride: string) => Promise<void>;
};

const formatItems: Array<{
  value: ExportFormat;
  label: string;
  icon: JSX.Element;
  hint: string;
}> = [
  {
    value: "txt",
    label: ".txt",
    icon: <FileText size={16} />,
    hint: "Plain text",
  },
  {
    value: "docx",
    label: ".docx",
    icon: <FileType2 size={16} />,
    hint: "Word document",
  },
  {
    value: "pdf",
    label: ".pdf",
    icon: <FileType size={16} />,
    hint: "Portable document",
  },
];

const styleItems: Array<{
  value: ExportStyle;
  label: string;
  icon: JSX.Element;
}> = [
  {
    value: "transcript",
    label: "Transcript",
    icon: <FileText size={16} />,
  },
  {
    value: "subtitles",
    label: "Subtitles",
    icon: <Captions size={16} />,
  },
  {
    value: "segments",
    label: "Segments",
    icon: <List size={16} />,
  },
];

function parseMmSsToSeconds(value: string): number {
  const [mmRaw, ssRaw] = value.split(":");
  const mm = Number(mmRaw);
  const ss = Number(ssRaw);
  if (Number.isNaN(mm) || Number.isNaN(ss)) {
    return 0;
  }
  return mm * 60 + ss;
}

function formatSrtTime(seconds: number): string {
  const hh = String(Math.floor(seconds / 3600)).padStart(2, "0");
  const mm = String(Math.floor((seconds % 3600) / 60)).padStart(2, "0");
  const ss = String(seconds % 60).padStart(2, "0");
  return `${hh}:${mm}:${ss},000`;
}

function buildExportContent(params: {
  transcriptText: string;
  segments: ExportSegment[];
  style: ExportStyle;
  includeTimestamps: boolean;
}): string {
  const { transcriptText, segments, style, includeTimestamps } = params;
  const normalizedTranscript = transcriptText.trim();

  if (style === "subtitles") {
    if (segments.length === 0) {
      return normalizedTranscript;
    }
    return segments
      .map((segment, index) => {
        const startSeconds = parseMmSsToSeconds(segment.time);
        const endSeconds = startSeconds + 4;
        return `${index + 1}\n${formatSrtTime(startSeconds)} --> ${formatSrtTime(endSeconds)}\n${segment.line.trim()}`;
      })
      .join("\n\n");
  }

  if (style === "segments") {
    if (segments.length === 0) {
      return normalizedTranscript;
    }
    return segments
      .map((segment) =>
        includeTimestamps ? `[${segment.time}] ${segment.line.trim()}` : segment.line.trim(),
      )
      .join("\n");
  }

  if (!includeTimestamps || segments.length === 0) {
    return normalizedTranscript;
  }

  return segments.map((segment) => `[${segment.time}] ${segment.line.trim()}`).join("\n");
}

export function ExportSheet({
  open,
  title,
  transcriptText,
  segments,
  onClose,
  onExport,
}: ExportSheetProps): JSX.Element | null {
  const [format, setFormat] = useState<ExportFormat>("txt");
  const [style, setStyle] = useState<ExportStyle>("transcript");
  const [includeTimestamps, setIncludeTimestamps] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  const exportContent = useMemo(() => {
    return buildExportContent({
      transcriptText,
      segments,
      style,
      includeTimestamps,
    });
  }, [includeTimestamps, segments, style, transcriptText]);

  const preview = useMemo(() => {
    const normalized = exportContent.trim();
    if (!normalized) {
      return "No content available for export.";
    }
    return normalized;
  }, [exportContent]);

  if (!open) {
    return null;
  }

  async function onConfirm(): Promise<void> {
    setIsExporting(true);
    try {
      await onExport(format, exportContent);
      onClose();
    } finally {
      setIsExporting(false);
    }
  }

  return (
    <div className="sheet-overlay" onClick={onClose}>
      <section className="export-sheet" onClick={(event) => event.stopPropagation()}>
        <div className="export-preview">
          <header className="export-preview-head">
            <strong>Transcription Preview</strong>
            <div className="export-tags">
              <span>{title}</span>
              <span>{format}</span>
            </div>
          </header>
          <pre>{preview}</pre>
        </div>

        <aside className="export-controls">
          <h3>Style</h3>
          <div className="export-style-grid">
            {styleItems.map((item) => (
              <button
                key={item.value}
                className={style === item.value ? "format-card active" : "format-card"}
                onClick={() => setStyle(item.value)}
              >
                {item.icon}
                <strong>{item.label}</strong>
              </button>
            ))}
          </div>

          <h3>Format</h3>
          <div className="export-format-grid">
            {formatItems.map((item) => (
              <button
                key={item.value}
                className={format === item.value ? "format-card active" : "format-card"}
                onClick={() => setFormat(item.value)}
              >
                {item.icon}
                <strong>{item.label}</strong>
                <small>{item.hint}</small>
              </button>
            ))}
          </div>

          <div className="inspector-block export-options-block">
            <h4>Options</h4>
            <label className="toggle-row">
              <span>Show timestamp</span>
              <input
                type="checkbox"
                checked={includeTimestamps}
                onChange={(event) => setIncludeTimestamps(event.target.checked)}
                disabled={style === "subtitles"}
              />
            </label>
          </div>

          <div className="export-actions">
            <button className="secondary-button" onClick={onClose} disabled={isExporting}>
              Cancel
            </button>
            <button className="primary-button" onClick={() => void onConfirm()} disabled={isExporting}>
              <Download size={14} />
              {isExporting ? "Exporting..." : "Export"}
            </button>
          </div>
        </aside>
      </section>
    </div>
  );
}
