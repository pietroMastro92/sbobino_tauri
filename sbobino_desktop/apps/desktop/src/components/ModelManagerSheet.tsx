import { Download, RefreshCw } from "lucide-react";
import type {
  ProvisioningModelCatalogEntry,
  ProvisioningProgressEvent,
  SpeechModel,
} from "../types";

type ModelManagerSheetProps = {
  open: boolean;
  modelsDir: string;
  models: ProvisioningModelCatalogEntry[];
  running: boolean;
  progress: ProvisioningProgressEvent | null;
  statusMessage: string;
  onDownloadModel: (model: SpeechModel) => Promise<void>;
  onDownloadAll: () => Promise<void>;
  onRefresh: () => Promise<void>;
  onCancel: () => Promise<void>;
  onClose: () => void;
};

export function ModelManagerSheet({
  open,
  modelsDir,
  models,
  running,
  progress,
  statusMessage,
  onDownloadModel,
  onDownloadAll,
  onRefresh,
  onCancel,
  onClose,
}: ModelManagerSheetProps): JSX.Element | null {
  if (!open) {
    return null;
  }

  const missingCount = models.filter((model) => !model.installed).length;

  return (
    <div className="sheet-overlay" onClick={onClose}>
      <section className="model-sheet" onClick={(event) => event.stopPropagation()}>
        <header className="model-sheet-head">
          <div>
            <h3>Model Manager</h3>
            <small>
              {missingCount === 0
                ? "All models are available"
                : `${missingCount} model${missingCount > 1 ? "s" : ""} missing`}
            </small>
          </div>
          <button className="icon-button" onClick={() => void onRefresh()} disabled={running} title="Refresh">
            <RefreshCw size={14} />
          </button>
        </header>

        <p className="muted">
          Directory: <code>{modelsDir || "(not configured)"}</code>
        </p>

        <div className="model-list">
          {models.map((model) => (
            <div key={model.key} className="model-row">
              <div className="model-row-main">
                <strong>{model.label}</strong>
                <small>{model.model_file}</small>
              </div>
              <div className="model-row-actions">
                <span className={model.installed ? "kind-chip" : "missing-chip"}>
                  {model.installed ? "Ready" : "Missing"}
                </span>
                <span className={model.coreml_installed ? "kind-chip" : "missing-chip"}>
                  {model.coreml_installed ? "CoreML Ready" : "CoreML Missing"}
                </span>
                <button
                  className="secondary-button"
                  disabled={running || (model.installed && model.coreml_installed)}
                  onClick={() => void onDownloadModel(model.key)}
                >
                  <Download size={14} />
                  {model.installed && model.coreml_installed ? "Installed" : "Download"}
                </button>
              </div>
            </div>
          ))}
        </div>

        {progress ? (
          <div className="inline-progress">
            <div style={{ width: `${progress.percentage}%` }} />
          </div>
        ) : null}

        {statusMessage ? <p className="muted">{statusMessage}</p> : null}

        <footer className="model-sheet-actions">
          {running ? (
            <button className="secondary-button" onClick={() => void onCancel()}>
              Cancel
            </button>
          ) : (
            <button className="primary-button" onClick={() => void onDownloadAll()} disabled={missingCount === 0}>
              Download Missing
            </button>
          )}
          <button className="secondary-button" onClick={onClose}>Close</button>
        </footer>
      </section>
    </div>
  );
}
