use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use uuid::Uuid;

use sbobino_application::RealtimeDelta;
use sbobino_domain::{ArtifactKind, LanguageCode, SpeechModel, TranscriptArtifact};

use crate::{error::CommandError, state::AppState};

#[derive(Debug, Deserialize)]
pub struct StartRealtimePayload {
    pub model: Option<SpeechModel>,
    pub language: Option<LanguageCode>,
    pub resume_artifact_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartRealtimeResponse {
    pub started: bool,
}

#[derive(Debug, Deserialize)]
pub struct StopRealtimePayload {
    pub save: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct StopRealtimeResponse {
    pub saved: bool,
    pub artifact: Option<TranscriptArtifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RealtimeStatusEvent {
    pub state: String,
    pub message: String,
}

#[tauri::command]
pub async fn start_realtime(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: Option<StartRealtimePayload>,
) -> Result<StartRealtimeResponse, CommandError> {
    let payload = payload.unwrap_or(StartRealtimePayload {
        model: None,
        language: None,
        resume_artifact_id: None,
    });

    let settings = state
        .runtime_factory
        .load_settings()
        .map_err(|e| CommandError::new("settings", e))?;

    let default_model = settings.transcription.model;
    let default_language = settings.transcription.language;
    let model = payload.model.unwrap_or(default_model);
    let language = payload.language.unwrap_or(default_language);

    if let Some(id) = &payload.resume_artifact_id {
        let artifact = state
            .artifact_service
            .get(id)
            .await
            .map_err(CommandError::from)?
            .ok_or_else(|| CommandError::new("not_found", "realtime session not found"))?;

        state
            .realtime
            .engine
            .seed_buffer(&artifact.raw_transcript)
            .await;
        *state.realtime.session_name.lock().await = Some(artifact.title.clone());
    } else {
        state.realtime.engine.reset().await;
        *state.realtime.session_name.lock().await = None;
    }

    *state.realtime.model_filename.lock().await = Some(model.ggml_filename().to_string());
    *state.realtime.language_code.lock().await = language.as_whisper_code().to_string();

    let app_handle = app.clone();
    let emit_delta = Arc::new(move |delta: RealtimeDelta| {
        let _ = app_handle.emit("realtime://delta", delta);
    });

    state
        .realtime
        .engine
        .start(
            model.ggml_filename(),
            language.as_whisper_code(),
            emit_delta,
        )
        .await
        .map_err(CommandError::from)?;

    let _ = app.emit(
        "realtime://status",
        RealtimeStatusEvent {
            state: "running".to_string(),
            message: "Live listening".to_string(),
        },
    );

    Ok(StartRealtimeResponse { started: true })
}

#[tauri::command]
pub async fn pause_realtime(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    state
        .realtime
        .engine
        .pause()
        .await
        .map_err(CommandError::from)?;

    let _ = app.emit(
        "realtime://status",
        RealtimeStatusEvent {
            state: "paused".to_string(),
            message: "Live paused".to_string(),
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn resume_realtime(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    state
        .realtime
        .engine
        .resume()
        .await
        .map_err(CommandError::from)?;

    let _ = app.emit(
        "realtime://status",
        RealtimeStatusEvent {
            state: "running".to_string(),
            message: "Live resumed".to_string(),
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn stop_realtime(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: Option<StopRealtimePayload>,
) -> Result<StopRealtimeResponse, CommandError> {
    let save = payload.and_then(|value| value.save).unwrap_or(true);

    let consolidated = state
        .realtime
        .engine
        .stop()
        .await
        .map_err(CommandError::from)?;

    let _ = app.emit(
        "realtime://status",
        RealtimeStatusEvent {
            state: "stopped".to_string(),
            message: "Live stopped".to_string(),
        },
    );

    if !save || consolidated.trim().is_empty() {
        return Ok(StopRealtimeResponse {
            saved: false,
            artifact: None,
        });
    }

    let settings = state
        .runtime_factory
        .load_settings()
        .map_err(|e| CommandError::new("settings", e))?;

    let session_title = state
        .realtime
        .session_name
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| format!("live_{}", Utc::now().format("%d%m%Y_%H%M%S")));

    let language_code = state.realtime.language_code.lock().await.clone();
    let model_filename = state
        .realtime
        .model_filename
        .lock()
        .await
        .clone()
        .unwrap_or_else(|| settings.transcription.model.ggml_filename().to_string());

    let mut optimized = consolidated.clone();
    let mut summary = String::new();
    let mut faqs = String::new();

    if settings.transcription.enable_ai_post_processing {
        if let Some(enhancer) = state
            .runtime_factory
            .build_active_enhancer()
            .map_err(|e| CommandError::new("runtime_factory", e))?
        {
            if let Ok(next_optimized) = enhancer.optimize(&consolidated, &language_code).await {
                optimized = next_optimized;
                if let Ok(summary_faq) =
                    enhancer.summarize_and_faq(&optimized, &language_code).await
                {
                    summary = summary_faq.summary;
                    faqs = summary_faq.faqs;
                }
            }
        }
    }

    let mut metadata = BTreeMap::new();
    metadata.insert("kind".to_string(), "realtime".to_string());
    metadata.insert("language".to_string(), language_code.clone());
    metadata.insert("model".to_string(), model_filename.clone());

    let artifact = TranscriptArtifact::new(
        Uuid::new_v4().to_string(),
        session_title.clone(),
        ArtifactKind::Realtime,
        format!("{session_title}.wav"),
        consolidated,
        optimized,
        summary,
        faqs,
        metadata,
    )
    .map_err(|e| CommandError::new("validation", e.to_string()))?;

    state
        .artifact_service
        .save(&artifact)
        .await
        .map_err(CommandError::from)?;

    let _ = app.emit("realtime://saved", artifact.clone());

    Ok(StopRealtimeResponse {
        saved: true,
        artifact: Some(artifact),
    })
}

#[tauri::command]
pub async fn list_realtime_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<TranscriptArtifact>, CommandError> {
    state
        .artifact_service
        .list(sbobino_application::ArtifactQuery {
            kind: Some(ArtifactKind::Realtime),
            query: None,
            limit: Some(100),
            offset: Some(0),
        })
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn load_realtime_session(
    state: State<'_, AppState>,
    payload: crate::commands::artifacts::GetArtifactPayload,
) -> Result<Option<TranscriptArtifact>, CommandError> {
    let artifact = state
        .artifact_service
        .get(&payload.id)
        .await
        .map_err(CommandError::from)?;

    if let Some(item) = &artifact {
        state
            .realtime
            .engine
            .seed_buffer(&item.raw_transcript)
            .await;
        *state.realtime.session_name.lock().await = Some(item.title.clone());
    }

    Ok(artifact)
}
