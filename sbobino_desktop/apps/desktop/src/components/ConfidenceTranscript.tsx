import type { CSSProperties } from "react";

import type { ConfidenceTranscriptDocument } from "../lib/whisperConfidence";

export function ConfidenceTranscript({
  document,
  fontSize,
}: {
  document: ConfidenceTranscriptDocument;
  fontSize: number;
}): JSX.Element {
  return (
    <div
      className="detail-editor confidence-transcript"
      style={{ fontSize: `${fontSize}px` }}
      role="document"
      aria-label="Confidence-colored transcript"
    >
      {document.fragments.map((fragment, index) => {
        if (!fragment.color || fragment.confidence === null) {
          return <span key={`${index}-${fragment.text.length}`}>{fragment.text}</span>;
        }

        return (
          <span
            key={`${index}-${fragment.text.length}`}
            className="confidence-word"
            data-tooltip={fragment.tooltip ?? undefined}
            style={{
              color: fragment.color,
              "--confidence-color": fragment.color,
            } as CSSProperties}
            tabIndex={0}
            aria-label={fragment.tooltip ?? undefined}
          >
            {fragment.text}
          </span>
        );
      })}
    </div>
  );
}
