use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use crate::{error::CommandError, state::AppState};

const REQUIRED_MODELS: [&str; 5] = [
    "ggml-tiny.bin",
    "ggml-base.bin",
    "ggml-small.bin",
    "ggml-medium.bin",
    "ggml-large-v3-turbo-q8_0.bin",
];

const COREML_ENCODERS: [(&str, &str); 5] = [
    (
        "ggml-tiny-encoder.mlmodelc",
        "ggml-tiny-encoder.mlmodelc.zip",
    ),
    (
        "ggml-base-encoder.mlmodelc",
        "ggml-base-encoder.mlmodelc.zip",
    ),
    (
        "ggml-small-encoder.mlmodelc",
        "ggml-small-encoder.mlmodelc.zip",
    ),
    (
        "ggml-medium-encoder.mlmodelc",
        "ggml-medium-encoder.mlmodelc.zip",
    ),
    (
        "ggml-large-v3-turbo-encoder.mlmodelc",
        "ggml-large-v3-turbo-encoder.mlmodelc.zip",
    ),
];

const MODEL_CATALOG: [(&str, &str, &str, &str, &str); 5] = [
    (
        "tiny",
        "Tiny",
        "ggml-tiny.bin",
        "ggml-tiny-encoder.mlmodelc",
        "ggml-tiny-encoder.mlmodelc.zip",
    ),
    (
        "base",
        "Base",
        "ggml-base.bin",
        "ggml-base-encoder.mlmodelc",
        "ggml-base-encoder.mlmodelc.zip",
    ),
    (
        "small",
        "Small",
        "ggml-small.bin",
        "ggml-small-encoder.mlmodelc",
        "ggml-small-encoder.mlmodelc.zip",
    ),
    (
        "medium",
        "Medium",
        "ggml-medium.bin",
        "ggml-medium-encoder.mlmodelc",
        "ggml-medium-encoder.mlmodelc.zip",
    ),
    (
        "large_turbo",
        "Large Turbo",
        "ggml-large-v3-turbo-q8_0.bin",
        "ggml-large-v3-turbo-encoder.mlmodelc",
        "ggml-large-v3-turbo-encoder.mlmodelc.zip",
    ),
];

const MODEL_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";

#[derive(Debug, Serialize)]
pub struct ProvisioningStatusResponse {
    pub ready: bool,
    pub models_dir: String,
    pub missing_models: Vec<String>,
    pub missing_encoders: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProvisioningProgressEvent {
    pub current: usize,
    pub total: usize,
    pub asset: String,
    pub stage: String,
    pub percentage: u8,
}

#[derive(Debug, Deserialize)]
pub struct ProvisioningStartPayload {
    pub include_coreml: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ProvisioningDownloadModelPayload {
    pub model: String,
    pub include_coreml: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProvisioningStartResponse {
    pub started: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProvisioningModelCatalogEntry {
    pub key: String,
    pub label: String,
    pub model_file: String,
    pub installed: bool,
    pub coreml_installed: bool,
}

fn collect_missing_models(models_dir: &Path) -> Vec<String> {
    REQUIRED_MODELS
        .iter()
        .filter_map(|filename| {
            let path = models_dir.join(filename);
            if path.exists() {
                None
            } else {
                Some((*filename).to_string())
            }
        })
        .collect::<Vec<_>>()
}

fn collect_missing_encoders(models_dir: &Path) -> Vec<String> {
    COREML_ENCODERS
        .iter()
        .filter_map(|(dir_name, _archive)| {
            let path = models_dir.join(dir_name);
            if path.is_dir() {
                None
            } else {
                Some((*dir_name).to_string())
            }
        })
        .collect::<Vec<_>>()
}

fn coreml_missing_for(models_dir: &Path, dir_name: &str) -> bool {
    !models_dir.join(dir_name).is_dir()
}

#[tauri::command]
pub async fn provisioning_status(
    state: State<'_, AppState>,
) -> Result<ProvisioningStatusResponse, CommandError> {
    let settings = state
        .runtime_factory
        .load_settings()
        .map_err(|e| CommandError::new("settings", e))?;

    let models_dir_value = if settings.transcription.models_dir.trim().is_empty() {
        settings.models_dir
    } else {
        settings.transcription.models_dir
    };
    let models_dir = PathBuf::from(state.runtime_factory.resolve_models_dir(&models_dir_value));

    let missing_models = collect_missing_models(&models_dir);
    let missing_encoders = collect_missing_encoders(&models_dir);

    Ok(ProvisioningStatusResponse {
        ready: missing_models.is_empty() && missing_encoders.is_empty(),
        models_dir: models_dir.to_string_lossy().to_string(),
        missing_models,
        missing_encoders,
    })
}

#[tauri::command]
pub async fn provisioning_models(
    state: State<'_, AppState>,
) -> Result<Vec<ProvisioningModelCatalogEntry>, CommandError> {
    let settings = state
        .runtime_factory
        .load_settings()
        .map_err(|e| CommandError::new("settings", e))?;

    let models_dir_value = if settings.transcription.models_dir.trim().is_empty() {
        settings.models_dir
    } else {
        settings.transcription.models_dir
    };
    let models_dir = PathBuf::from(state.runtime_factory.resolve_models_dir(&models_dir_value));

    Ok(MODEL_CATALOG
        .iter()
        .map(|(key, label, model_file, encoder_dir, _encoder_archive)| {
            ProvisioningModelCatalogEntry {
                key: (*key).to_string(),
                label: (*label).to_string(),
                model_file: (*model_file).to_string(),
                installed: models_dir.join(model_file).exists(),
                coreml_installed: models_dir.join(encoder_dir).is_dir(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn provisioning_start(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: Option<ProvisioningStartPayload>,
) -> Result<ProvisioningStartResponse, CommandError> {
    let include_coreml = payload
        .and_then(|value| value.include_coreml)
        .unwrap_or(true);

    let settings = state
        .runtime_factory
        .load_settings()
        .map_err(|e| CommandError::new("settings", e))?;

    let models_dir_value = if settings.transcription.models_dir.trim().is_empty() {
        settings.models_dir
    } else {
        settings.transcription.models_dir
    };
    let models_dir = PathBuf::from(state.runtime_factory.resolve_models_dir(&models_dir_value));

    tokio::fs::create_dir_all(&models_dir).await.map_err(|e| {
        CommandError::new("provisioning", format!("failed to create models dir: {e}"))
    })?;

    let missing_models = collect_missing_models(&models_dir);

    let missing_encoders = if include_coreml {
        COREML_ENCODERS
            .iter()
            .filter_map(|(dir_name, archive)| {
                let path = models_dir.join(dir_name);
                if path.is_dir() {
                    None
                } else {
                    Some(((*dir_name).to_string(), (*archive).to_string()))
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let total = missing_models.len() + missing_encoders.len();
    if total == 0 {
        let _ = app.emit(
            "provisioning://status",
            BTreeMap::from([
                ("state".to_string(), "completed".to_string()),
                (
                    "message".to_string(),
                    "All required models are already available.".to_string(),
                ),
            ]),
        );
        return Ok(ProvisioningStartResponse { started: false });
    }

    let cancel_token = CancellationToken::new();
    *state.provisioning.cancel_token.lock().await = Some(cancel_token.clone());

    spawn_provisioning_download(
        app,
        models_dir,
        missing_models,
        missing_encoders,
        cancel_token,
    );

    Ok(ProvisioningStartResponse { started: true })
}

#[tauri::command]
pub async fn provisioning_download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: ProvisioningDownloadModelPayload,
) -> Result<ProvisioningStartResponse, CommandError> {
    let include_coreml = payload.include_coreml.unwrap_or(true);

    let settings = state
        .runtime_factory
        .load_settings()
        .map_err(|e| CommandError::new("settings", e))?;

    let models_dir_value = if settings.transcription.models_dir.trim().is_empty() {
        settings.models_dir
    } else {
        settings.transcription.models_dir
    };
    let models_dir = PathBuf::from(state.runtime_factory.resolve_models_dir(&models_dir_value));

    tokio::fs::create_dir_all(&models_dir).await.map_err(|e| {
        CommandError::new("provisioning", format!("failed to create models dir: {e}"))
    })?;

    let Some((_, label, model_file, encoder_dir, encoder_archive)) = MODEL_CATALOG
        .iter()
        .find(|(key, _, _, _, _)| *key == payload.model)
    else {
        return Err(CommandError::new(
            "validation",
            format!("unknown model key: {}", payload.model),
        ));
    };

    let mut missing_models = Vec::new();
    if !models_dir.join(model_file).exists() {
        missing_models.push((*model_file).to_string());
    }

    let mut missing_encoders = Vec::new();
    if include_coreml && coreml_missing_for(&models_dir, encoder_dir) {
        missing_encoders.push(((*encoder_dir).to_string(), (*encoder_archive).to_string()));
    }

    let total = missing_models.len() + missing_encoders.len();
    if total == 0 {
        let _ = app.emit(
            "provisioning://status",
            BTreeMap::from([
                ("state".to_string(), "completed".to_string()),
                (
                    "message".to_string(),
                    format!("{label} is already available."),
                ),
            ]),
        );
        return Ok(ProvisioningStartResponse { started: false });
    }

    let cancel_token = CancellationToken::new();
    *state.provisioning.cancel_token.lock().await = Some(cancel_token.clone());

    spawn_provisioning_download(
        app,
        models_dir,
        missing_models,
        missing_encoders,
        cancel_token,
    );

    Ok(ProvisioningStartResponse { started: true })
}

#[tauri::command]
pub async fn provisioning_cancel(state: State<'_, AppState>) -> Result<(), CommandError> {
    let token = {
        let mut guard = state.provisioning.cancel_token.lock().await;
        guard.take()
    };

    if let Some(token) = token {
        token.cancel();
    }

    Ok(())
}

fn spawn_provisioning_download(
    app: tauri::AppHandle,
    models_dir: PathBuf,
    missing_models: Vec<String>,
    missing_encoders: Vec<(String, String)>,
    cancel_token: CancellationToken,
) {
    let total = missing_models.len() + missing_encoders.len();

    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        let mut current = 0usize;

        let mut emit_progress = |asset: String, stage: String| {
            current += 1;
            let percentage = ((current as f32 / total as f32) * 100.0).round() as u8;
            let _ = app.emit(
                "provisioning://progress",
                ProvisioningProgressEvent {
                    current,
                    total,
                    asset,
                    stage,
                    percentage,
                },
            );
        };

        for model in missing_models {
            if cancel_token.is_cancelled() {
                let _ = app.emit(
                    "provisioning://status",
                    BTreeMap::from([
                        ("state".to_string(), "cancelled".to_string()),
                        ("message".to_string(), "Provisioning cancelled.".to_string()),
                    ]),
                );
                return;
            }

            let url = format!("{MODEL_BASE_URL}{model}");
            let destination = models_dir.join(&model);
            match download_to_path(&client, &url, &destination, &cancel_token).await {
                Ok(()) => emit_progress(model, "downloaded".to_string()),
                Err(error) => {
                    let _ = app.emit(
                        "provisioning://status",
                        BTreeMap::from([
                            ("state".to_string(), "error".to_string()),
                            (
                                "message".to_string(),
                                format!("Provisioning failed: {error}"),
                            ),
                        ]),
                    );
                    return;
                }
            }
        }

        for (encoder_dir, archive) in missing_encoders {
            if cancel_token.is_cancelled() {
                let _ = app.emit(
                    "provisioning://status",
                    BTreeMap::from([
                        ("state".to_string(), "cancelled".to_string()),
                        ("message".to_string(), "Provisioning cancelled.".to_string()),
                    ]),
                );
                return;
            }

            let url = format!("{MODEL_BASE_URL}{archive}");
            let archive_path = models_dir.join(&archive);

            match download_to_path(&client, &url, &archive_path, &cancel_token).await {
                Ok(()) => {}
                Err(error) => {
                    let _ = app.emit(
                        "provisioning://status",
                        BTreeMap::from([
                            ("state".to_string(), "error".to_string()),
                            (
                                "message".to_string(),
                                format!("Failed to download {encoder_dir}: {error}"),
                            ),
                        ]),
                    );
                    return;
                }
            }

            let extraction = tokio::task::spawn_blocking({
                let archive_path = archive_path.clone();
                let models_dir = models_dir.clone();
                move || extract_zip_archive(&archive_path, &models_dir)
            })
            .await;

            match extraction {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let _ = app.emit(
                        "provisioning://status",
                        BTreeMap::from([
                            ("state".to_string(), "error".to_string()),
                            (
                                "message".to_string(),
                                format!("Failed to extract {encoder_dir}: {error}"),
                            ),
                        ]),
                    );
                    return;
                }
                Err(error) => {
                    let _ = app.emit(
                        "provisioning://status",
                        BTreeMap::from([
                            ("state".to_string(), "error".to_string()),
                            (
                                "message".to_string(),
                                format!(
                                    "Failed to extract {encoder_dir}: task join error: {error}"
                                ),
                            ),
                        ]),
                    );
                    return;
                }
            }

            let _ = tokio::fs::remove_file(&archive_path).await;
            emit_progress(encoder_dir, "downloaded".to_string());
        }

        let _ = app.emit(
            "provisioning://status",
            BTreeMap::from([
                ("state".to_string(), "completed".to_string()),
                (
                    "message".to_string(),
                    "Provisioning completed successfully.".to_string(),
                ),
            ]),
        );
    });
}

async fn download_to_path(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    cancel_token: &CancellationToken,
) -> Result<(), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download failed: {e}"))?;

    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|e| format!("failed to create destination file: {e}"))?;

    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("download stream failure: {e}"))?
    {
        if cancel_token.is_cancelled() {
            return Err("cancelled".to_string());
        }

        file.write_all(&chunk)
            .await
            .map_err(|e| format!("failed to write chunk: {e}"))?;
    }

    file.flush()
        .await
        .map_err(|e| format!("failed to flush destination file: {e}"))?;

    Ok(())
}

fn extract_zip_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file =
        std::fs::File::open(archive_path).map_err(|e| format!("failed to open archive: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("invalid zip archive: {e}"))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("failed to read zip entry: {e}"))?;

        let Some(safe_path) = entry.enclosed_name() else {
            continue;
        };

        let out_path = destination.join(safe_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("failed to create directory: {e}"))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create parent directory: {e}"))?;
        }

        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| format!("failed to create extracted file: {e}"))?;

        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("failed to extract zip entry: {e}"))?;
    }

    Ok(())
}
