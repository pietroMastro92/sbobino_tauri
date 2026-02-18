use std::{fs, path::PathBuf};

use async_trait::async_trait;

use sbobino_application::{ApplicationError, SettingsRepository};
use sbobino_domain::AppSettings;

#[derive(Debug, Clone)]
pub struct FsSettingsRepository {
    path: PathBuf,
}

impl FsSettingsRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load_sync(&self) -> Result<AppSettings, ApplicationError> {
        if !self.path.exists() {
            let defaults = AppSettings::default();
            let serialized = serde_json::to_string_pretty(&defaults).map_err(|e| {
                ApplicationError::Settings(format!("failed to serialize default settings: {e}"))
            })?;
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    ApplicationError::Settings(format!(
                        "failed to create settings directory {}: {e}",
                        parent.display()
                    ))
                })?;
            }
            fs::write(&self.path, serialized).map_err(|e| {
                ApplicationError::Settings(format!(
                    "failed to write settings file {}: {e}",
                    self.path.display()
                ))
            })?;
            return Ok(defaults);
        }

        let content = fs::read_to_string(&self.path).map_err(|e| {
            ApplicationError::Settings(format!(
                "failed to read settings file {}: {e}",
                self.path.display()
            ))
        })?;

        let raw_json = serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
            ApplicationError::Settings(format!(
                "invalid settings JSON in {}: {e}",
                self.path.display()
            ))
        })?;

        let mut settings =
            serde_json::from_value::<AppSettings>(raw_json.clone()).map_err(|e| {
                ApplicationError::Settings(format!(
                    "invalid settings JSON in {}: {e}",
                    self.path.display()
                ))
            })?;

        // Migrate legacy defaults from Python bundle assumptions to runtime-friendly defaults.
        if settings.ffmpeg_path == "resources/ffmpeg_bin/ffmpeg" {
            settings.ffmpeg_path = "ffmpeg".to_string();
        }
        if settings.whisper_cli_path == "whisper.cpp/build/bin/whisper-cli" {
            settings.whisper_cli_path = "whisper-cli".to_string();
        }

        let has_general = raw_json
            .get("general")
            .is_some_and(|value| value.is_object());
        let has_transcription = raw_json
            .get("transcription")
            .is_some_and(|value| value.is_object());
        let has_ai = raw_json.get("ai").is_some_and(|value| value.is_object());
        let has_prompts = raw_json
            .get("prompts")
            .is_some_and(|value| value.is_object());

        if has_general && has_transcription && has_ai && has_prompts {
            settings.sync_legacy_from_sections();
        } else {
            settings.sync_sections_from_legacy();
        }

        Ok(settings)
    }

    pub fn save_sync(&self, settings: &AppSettings) -> Result<(), ApplicationError> {
        let mut normalized = settings.clone();
        // Repository-level saves keep legacy fields as canonical to remain
        // backward-compatible with callers that only mutate flat settings.
        normalized.sync_sections_from_legacy();
        normalized.sync_legacy_from_sections();

        let serialized = serde_json::to_string_pretty(&normalized).map_err(|e| {
            ApplicationError::Settings(format!("failed to serialize settings: {e}"))
        })?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ApplicationError::Settings(format!(
                    "failed to create settings directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

        fs::write(&self.path, serialized).map_err(|e| {
            ApplicationError::Settings(format!(
                "failed to save settings file {}: {e}",
                self.path.display()
            ))
        })
    }
}

#[async_trait]
impl SettingsRepository for FsSettingsRepository {
    async fn load(&self) -> Result<AppSettings, ApplicationError> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let repo = FsSettingsRepository::new(path);
            repo.load_sync()
        })
        .await
        .map_err(|e| ApplicationError::Settings(format!("settings load join error: {e}")))?
    }

    async fn save(&self, settings: &AppSettings) -> Result<(), ApplicationError> {
        let path = self.path.clone();
        let settings = settings.clone();
        tokio::task::spawn_blocking(move || {
            let repo = FsSettingsRepository::new(path);
            repo.save_sync(&settings)
        })
        .await
        .map_err(|e| ApplicationError::Settings(format!("settings save join error: {e}")))?
    }
}
