use serde::Serialize;
use tauri::State;

use crate::{error::CommandError, state::AppState};

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeHealthResponse {
    pub whisper_cli_path: String,
    pub whisper_cli_resolved: String,
    pub whisper_stream_path: String,
    pub whisper_stream_resolved: String,
    pub models_dir_configured: String,
    pub models_dir_resolved: String,
    pub model_filename: String,
    pub model_present: bool,
    pub coreml_encoder_present: bool,
    pub missing_models: Vec<String>,
    pub missing_encoders: Vec<String>,
}

#[tauri::command]
pub async fn get_transcription_runtime_health(
    state: State<'_, AppState>,
) -> Result<RuntimeHealthResponse, CommandError> {
    let health = state
        .runtime_factory
        .runtime_health()
        .map_err(|e| CommandError::new("runtime_health", e))?;

    Ok(RuntimeHealthResponse {
        whisper_cli_path: health.whisper_cli_path,
        whisper_cli_resolved: health.whisper_cli_resolved,
        whisper_stream_path: health.whisper_stream_path,
        whisper_stream_resolved: health.whisper_stream_resolved,
        models_dir_configured: health.models_dir_configured,
        models_dir_resolved: health.models_dir_resolved,
        model_filename: health.model_filename,
        model_present: health.model_present,
        coreml_encoder_present: health.coreml_encoder_present,
        missing_models: health.missing_models,
        missing_encoders: health.missing_encoders,
    })
}
