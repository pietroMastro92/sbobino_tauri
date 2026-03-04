import { Braces, Captions, Copy, Download, FileCode2, FileText, FileType, FileType2, List, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "../i18n";

export type ExportFormat = "txt" | "docx" | "html" | "pdf" | "json";
export type ExportStyle = "transcript" | "subtitles" | "segments";
export type ExportGrouping = "none" | "speaker_paragraphs";

export type ExportSegment = {
  time: string;
  line: string;
};

export type ExportOptions = {
  includeTimestamps: boolean;
  grouping: ExportGrouping;
};

export type ExportRequest = {
  format: ExportFormat;
  style: ExportStyle;
  options: ExportOptions;
  segments: ExportSegment[];
  contentOverride?: string;
};

type ExportSheetProps = {
  open: boolean;
  transcriptText: string;
  segments: ExportSegment[];
  onClose: () => void;
  onExport: (payload: ExportRequest) => Promise<void>;
};

type FormatItem = {
  value: ExportFormat;
  label: string;
  icon: JSX.Element;
  hint: string;
  badge?: string;
};

function getFormatItems(t: (key: string, fallback?: string) => string): FormatItem[] {
  return [
    {
      value: "txt",
      label: ".txt",
      icon: <FileText size={16} />,
      hint: t("export.plainText", "Plain text"),
    },
    {
      value: "docx",
      label: ".docx",
      icon: <FileType2 size={16} />,
      hint: t("export.wordDocument", "Word document"),
    },
    {
      value: "html",
      label: ".html",
      icon: <FileCode2 size={16} />,
      hint: t("export.webPage", "Web page"),
    },
    {
      value: "pdf",
      label: ".pdf",
      icon: <FileType size={16} />,
      hint: t("export.portableDocument", "Portable document"),
    },
    {
      value: "json",
      label: ".json",
      icon: <Braces size={16} />,
      hint: t("export.structuredData", "Structured data"),
    },
  ];
}

type StyleItem = {
  value?: ExportStyle;
  label: string;
  icon: JSX.Element;
  subtitle?: string;
  badge?: string;
};

function getStyleItems(t: (key: string, fallback?: string) => string): StyleItem[] {
  return [
    {
      value: "transcript",
      label: t("export.transcript", "Transcript"),
      icon: <FileText size={16} />,
    },
    {
      value: "subtitles",
      label: t("export.subtitles", "Subtitles"),
      icon: <Captions size={16} />,
    },
    {
      value: "segments",
      label: t("export.segments", "Segments"),
      icon: <List size={16} />,
    },
  ];
}

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
  transcriptText,
  segments,
  onClose,
  onExport,
}: ExportSheetProps): JSX.Element | null {
  const [format, setFormat] = useState<ExportFormat>("txt");
  const [style, setStyle] = useState<ExportStyle>("transcript");
  const [includeTimestamps, setIncludeTimestamps] = useState(false);
  const [grouping, setGrouping] = useState<ExportGrouping>("none");
  const [isExporting, setIsExporting] = useState(false);
  const { t, language } = useTranslation();

  const formatItems = useMemo(() => getFormatItems(t), [language]);
  const styleItems = useMemo(() => getStyleItems(t), [language]);

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
      return t("export.noContent", "No content available for export.");
    }

    if (format === "json") {
      try {
        const obj = { text: normalized };
        return JSON.stringify(obj, null, 2);
      } catch {
        return normalized;
      }
    }

    return normalized;
  }, [exportContent, format]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose, open]);

  if (!open) {
    return null;
  }

  async function onConfirm(): Promise<void> {
    setIsExporting(true);
    try {
      await onExport({
        format,
        style,
        options: {
          includeTimestamps,
          grouping,
        },
        segments,
        contentOverride: exportContent,
      });
      onClose();
    } finally {
      setIsExporting(false);
    }
  }

  async function onCopyContent(): Promise<void> {
    try {
      await navigator.clipboard.writeText(exportContent);
    } catch {
      // Keep the export sheet responsive even if clipboard fails.
    }
  }

  return (
    <div className="sheet-overlay" onClick={onClose}>
      <section
        className="export-sheet"
        role="dialog"
        aria-modal="true"
        aria-labelledby="export-sheet-title"
        onClick={(event) => event.stopPropagation()}
      >
        <button
          className="export-close-button"
          aria-label={t("export.closePreview", "Close export preview")}
          onClick={onClose}
          disabled={isExporting}
        >
          <X size={14} />
        </button>

        <div className="export-preview">
          <header className="export-preview-head">
            <strong id="export-sheet-title">{t("export.preview", "Export Preview")}</strong>
            <div className="export-tags">
              <span>{style}</span>
              <span>{format}</span>
            </div>
          </header>
          <pre>{preview}</pre>
        </div>

        <aside className="export-controls">
          <div className="export-controls-scroll">
            <h3>{t("export.style", "Style")}</h3>
            <div className="export-style-grid">
              {styleItems.map((item) => (
                <button
                  key={item.label}
                  className={style === item.value ? "format-card active" : "format-card"}
                  onClick={() => {
                    if (item.value) {
                      setStyle(item.value);
                    }
                  }}
                  disabled={!item.value}
                >
                  <span className="format-card-top">
                    <span className="format-card-icon">{item.icon}</span>
                    {item.badge ? <span className="format-card-badge">{item.badge}</span> : null}
                  </span>
                  <strong>{item.label}</strong>
                  {item.subtitle ? <small>{item.subtitle}</small> : null}
                </button>
              ))}
            </div>

            <h3>{t("export.format", "Format")}</h3>
            <div className="export-format-grid">
              {formatItems.map((item) => (
                <button
                  key={item.value}
                  className={format === item.value ? "format-card active" : "format-card"}
                  onClick={() => setFormat(item.value)}
                >
                  <span className="format-card-top">
                    <span className="format-card-icon">{item.icon}</span>
                    {item.badge ? <span className="format-card-badge">{item.badge}</span> : null}
                  </span>
                  <strong>{item.label}</strong>
                  <small>{item.hint}</small>
                </button>
              ))}
            </div>

            <div className="inspector-block export-options-block">
              <h4>{t("export.options", "Options")}</h4>
              <div className="property-line">
                <span>{t("export.grouping", "Grouping")}</span>
                <select
                  value={grouping}
                  onChange={(event) => setGrouping(event.target.value as ExportGrouping)}
                >
                  <option value="none">{t("export.groupingNone", "None")}</option>
                  <option value="speaker_paragraphs" disabled>
                    {t("export.speakerParagraphs", "Speaker paragraphs")}
                  </option>
                </select>
              </div>
              <label className="toggle-row">
                <span>{t("export.showTimestamps", "Show Timestamps")}</span>
                <input
                  type="checkbox"
                  checked={includeTimestamps}
                  onChange={(event) => setIncludeTimestamps(event.target.checked)}
                  disabled={style === "subtitles"}
                />
              </label>
            </div>
          </div>

          <div className="export-actions">
            <button className="secondary-button" onClick={onClose} disabled={isExporting}>
              {t("export.close", "Close")}
            </button>
            <button
              className="secondary-button"
              onClick={() => void onCopyContent()}
              disabled={isExporting}
            >
              <Copy size={14} />
              {t("export.copy", "Copy")}
            </button>
            <button className="primary-button" onClick={() => void onConfirm()} disabled={isExporting}>
              <Download size={14} />
              {isExporting ? t("export.exporting", "Exporting...") : t("export.export", "Export")}
            </button>
          </div>
        </aside>
      </section>
    </div>
  );
}
