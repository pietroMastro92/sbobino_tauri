use serde::{Deserialize, Serialize};
use tauri::State;

use sbobino_domain::{
    AiProvider, AiSettings, AppSettings, GeneralSettings, LanguageCode, PromptSettings, PromptTask,
    PromptTemplate, TranscriptionSettings,
};

use crate::{error::CommandError, state::AppState};

#[derive(Debug, Deserialize, Default)]
pub struct UpdateSettingsPartialPayload {
    pub general: Option<GeneralSettings>,
    pub transcription: Option<TranscriptionSettings>,
    pub ai: Option<AiSettings>,
    pub prompts: Option<PromptSettings>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateAiProvidersPayload {
    pub active_provider: Option<AiProvider>,
    pub foundation_apple_enabled: Option<bool>,
    pub gemini_api_key: Option<Option<String>>,
    pub gemini_model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SavePromptPayload {
    pub template: PromptTemplate,
    pub bind_task: Option<PromptTask>,
}

#[derive(Debug, Deserialize)]
pub struct DeletePromptPayload {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct TestPromptPayload {
    pub input: String,
    pub language: Option<LanguageCode>,
    pub task: Option<PromptTask>,
    pub prompt_override: Option<String>,
    pub model_override: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestPromptResponse {
    pub output: String,
    pub summary: String,
    pub faqs: String,
    pub model: String,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, CommandError> {
    state
        .settings_service
        .get()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, CommandError> {
    state
        .settings_service
        .update(settings)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_settings_snapshot(
    state: State<'_, AppState>,
) -> Result<AppSettings, CommandError> {
    state
        .settings_service
        .snapshot()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_settings_partial(
    state: State<'_, AppState>,
    payload: UpdateSettingsPartialPayload,
) -> Result<AppSettings, CommandError> {
    state
        .settings_service
        .update_partial(
            payload.general,
            payload.transcription,
            payload.ai,
            payload.prompts,
        )
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_ai_providers(state: State<'_, AppState>) -> Result<AiSettings, CommandError> {
    state
        .settings_service
        .ai_settings()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_ai_providers(
    state: State<'_, AppState>,
    payload: UpdateAiProvidersPayload,
) -> Result<AiSettings, CommandError> {
    state
        .settings_service
        .update_ai_settings(
            payload.active_provider,
            payload.foundation_apple_enabled,
            payload.gemini_api_key,
            payload.gemini_model,
        )
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_prompts(state: State<'_, AppState>) -> Result<Vec<PromptTemplate>, CommandError> {
    state
        .settings_service
        .list_prompts()
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn save_prompt(
    state: State<'_, AppState>,
    payload: SavePromptPayload,
) -> Result<Vec<PromptTemplate>, CommandError> {
    let updated_settings = state
        .settings_service
        .save_prompt(payload.template, payload.bind_task)
        .await
        .map_err(CommandError::from)?;

    Ok(updated_settings.prompts.templates)
}

#[tauri::command]
pub async fn delete_prompt(
    state: State<'_, AppState>,
    payload: DeletePromptPayload,
) -> Result<Vec<PromptTemplate>, CommandError> {
    let updated_settings = state
        .settings_service
        .delete_prompt(payload.id)
        .await
        .map_err(CommandError::from)?;

    Ok(updated_settings.prompts.templates)
}

#[tauri::command]
pub async fn reset_prompts(
    state: State<'_, AppState>,
) -> Result<Vec<PromptTemplate>, CommandError> {
    let updated_settings = state
        .settings_service
        .reset_prompts()
        .await
        .map_err(CommandError::from)?;

    Ok(updated_settings.prompts.templates)
}

#[tauri::command]
pub async fn test_prompt(
    state: State<'_, AppState>,
    payload: TestPromptPayload,
) -> Result<TestPromptResponse, CommandError> {
    let input = payload.input.trim().to_string();
    if input.is_empty() {
        return Err(CommandError::new(
            "validation",
            "prompt test input cannot be empty",
        ));
    }

    let settings = state
        .settings_service
        .snapshot()
        .await
        .map_err(CommandError::from)?;

    let model = payload
        .model_override
        .clone()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| settings.ai.providers.gemini.model.clone());
    let language = payload
        .language
        .unwrap_or(settings.transcription.language)
        .as_whisper_code()
        .to_string();

    let Some(enhancer) = state
        .runtime_factory
        .build_gemini_enhancer_with_overrides(Some(model.clone()), None, None)
        .map_err(|e| CommandError::new("runtime_factory", e))?
    else {
        return Err(CommandError::new(
            "validation",
            "Gemini API key is required to test prompts",
        ));
    };

    let prompt_override = payload.prompt_override.as_deref();
    let task = payload.task.unwrap_or(PromptTask::Optimize);

    match task {
        PromptTask::Optimize => {
            let output = enhancer
                .optimize_with_prompt(&input, &language, prompt_override)
                .await
                .map_err(CommandError::from)?;

            Ok(TestPromptResponse {
                output: output.clone(),
                summary: String::new(),
                faqs: String::new(),
                model,
            })
        }
        PromptTask::Summary | PromptTask::Faq => {
            let output = enhancer
                .summarize_and_faq_with_prompt(&input, &language, prompt_override)
                .await
                .map_err(CommandError::from)?;

            Ok(TestPromptResponse {
                output: format!("Summary:\n{}\n\nFAQs:\n{}", output.summary, output.faqs),
                summary: output.summary,
                faqs: output.faqs,
                model,
            })
        }
    }
}
