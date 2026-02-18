pub mod adapters;
pub mod repositories;

use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::info;

use sbobino_application::{ArtifactService, SettingsService, TranscriptionService};
use sbobino_domain::{AiProvider, AppSettings, PromptTask};

use adapters::{
    ffmpeg::FfmpegAdapter, gemini::GeminiEnhancer, noop_enhancer::NoopEnhancer,
    whisper_cpp::WhisperCppEngine, whisper_stream::WhisperStreamEngine,
};
use repositories::{
    fs_settings_repository::FsSettingsRepository,
    sqlite_artifact_repository::SqliteArtifactRepository,
};

#[derive(Clone)]
pub struct RuntimeTranscriptionFactory {
    settings_repo: Arc<FsSettingsRepository>,
    artifacts_repo: Arc<SqliteArtifactRepository>,
    data_dir: PathBuf,
}

const REQUIRED_MODEL_FILES: [&str; 5] = [
    "ggml-tiny.bin",
    "ggml-base.bin",
    "ggml-small.bin",
    "ggml-medium.bin",
    "ggml-large-v3-turbo-q8_0.bin",
];

const REQUIRED_COREML_ENCODERS: [(&str, &str); 5] = [
    ("ggml-tiny.bin", "ggml-tiny-encoder.mlmodelc"),
    ("ggml-base.bin", "ggml-base-encoder.mlmodelc"),
    ("ggml-small.bin", "ggml-small-encoder.mlmodelc"),
    ("ggml-medium.bin", "ggml-medium-encoder.mlmodelc"),
    (
        "ggml-large-v3-turbo-q8_0.bin",
        "ggml-large-v3-turbo-encoder.mlmodelc",
    ),
];

#[derive(Debug, Clone)]
pub struct RuntimeHealth {
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

#[derive(Debug, Clone)]
struct BinaryResolution {
    resolved_path: String,
}

impl RuntimeTranscriptionFactory {
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| format!("failed to create app data dir {}: {e}", data_dir.display()))?;

        let settings_path = data_dir.join("settings.json");
        let artifacts_db = data_dir.join("artifacts.db");

        let settings_repo = Arc::new(FsSettingsRepository::new(settings_path));
        let artifacts_repo = Arc::new(
            SqliteArtifactRepository::new(artifacts_db)
                .map_err(|e| format!("failed to initialize artifacts repository: {e}"))?,
        );

        Ok(Self {
            settings_repo,
            artifacts_repo,
            data_dir: data_dir.to_path_buf(),
        })
    }

    pub fn build_service(&self) -> Result<Arc<TranscriptionService>, String> {
        let settings = self.load_settings()?;

        let ffmpeg_path = self.resolve_binary_path(&settings.transcription.ffmpeg_path, "ffmpeg");
        let whisper_cli_path =
            self.resolve_binary_path(&settings.transcription.whisper_cli_path, "whisper-cli");
        let models_dir = self.resolve_models_dir(&settings.transcription.models_dir);

        let transcoder = Arc::new(FfmpegAdapter::new(ffmpeg_path));
        let speech_engine = Arc::new(WhisperCppEngine::new(whisper_cli_path, models_dir));

        let enhancer: Arc<dyn sbobino_application::TranscriptEnhancer> =
            match settings.ai.active_provider {
                AiProvider::Gemini => {
                    if let Some(api_key) = settings.ai.providers.gemini.api_key.clone() {
                        Arc::new(GeminiEnhancer::new(
                            api_key,
                            settings.ai.providers.gemini.model.clone(),
                            settings.prompt_for_task(PromptTask::Optimize),
                            settings.prompt_for_task(PromptTask::Summary),
                        ))
                    } else {
                        Arc::new(NoopEnhancer)
                    }
                }
                AiProvider::FoundationApple | AiProvider::None => Arc::new(NoopEnhancer),
            };

        Ok(Arc::new(TranscriptionService::new(
            transcoder,
            speech_engine,
            enhancer,
            self.artifacts_repo.clone(),
        )))
    }

    pub fn settings_service(&self) -> Arc<SettingsService> {
        Arc::new(SettingsService::new(self.settings_repo.clone()))
    }

    pub fn artifact_service(&self) -> Arc<ArtifactService> {
        Arc::new(ArtifactService::new(self.artifacts_repo.clone()))
    }

    pub fn build_gemini_enhancer(&self) -> Result<Option<GeminiEnhancer>, String> {
        self.build_gemini_enhancer_with_overrides(None, None, None)
    }

    pub fn build_gemini_enhancer_with_overrides(
        &self,
        model_override: Option<String>,
        optimize_prompt_override: Option<String>,
        summary_prompt_override: Option<String>,
    ) -> Result<Option<GeminiEnhancer>, String> {
        let settings = self.load_settings()?;

        let Some(api_key) = settings.ai.providers.gemini.api_key.clone() else {
            return Ok(None);
        };

        let model = model_override
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .unwrap_or_else(|| settings.ai.providers.gemini.model.clone());

        Ok(Some(GeminiEnhancer::new(
            api_key,
            model,
            optimize_prompt_override.or_else(|| settings.prompt_for_task(PromptTask::Optimize)),
            summary_prompt_override.or_else(|| settings.prompt_for_task(PromptTask::Summary)),
        )))
    }

    pub fn build_whisper_stream_engine(&self) -> Result<WhisperStreamEngine, String> {
        let settings = self.load_settings()?;
        let whisper_stream_path = self.resolve_binary_path(
            &settings
                .transcription
                .whisper_cli_path
                .replace("whisper-cli", "whisper-stream"),
            "whisper-stream",
        );
        let models_dir = self.resolve_models_dir(&settings.transcription.models_dir);
        Ok(WhisperStreamEngine::new(whisper_stream_path, models_dir))
    }

    pub fn load_settings(&self) -> Result<AppSettings, String> {
        let mut settings = self
            .settings_repo
            .load_sync()
            .map_err(|e| format!("failed to load settings: {e}"))?;

        self.migrate_models_dir_if_needed(&mut settings)?;
        Ok(settings)
    }

    pub fn runtime_health(&self) -> Result<RuntimeHealth, String> {
        let settings = self.load_settings()?;
        let configured_models_dir = if settings.transcription.models_dir.trim().is_empty() {
            settings.models_dir.clone()
        } else {
            settings.transcription.models_dir.clone()
        };
        let resolved_models_dir = self.resolve_models_dir(&configured_models_dir);
        let models_dir = PathBuf::from(&resolved_models_dir);

        let whisper_cli_configured = settings.transcription.whisper_cli_path.clone();
        let whisper_stream_configured = settings
            .transcription
            .whisper_cli_path
            .replace("whisper-cli", "whisper-stream");

        let whisper_cli_resolution =
            self.resolve_binary_details(&whisper_cli_configured, "whisper-cli");
        let whisper_stream_resolution =
            self.resolve_binary_details(&whisper_stream_configured, "whisper-stream");

        let model_filename = settings.transcription.model.ggml_filename().to_string();
        let model_present = models_dir.join(&model_filename).exists();
        let coreml_encoder = encoder_for_model(&model_filename).unwrap_or_default();
        let coreml_encoder_present = if coreml_encoder.is_empty() {
            false
        } else {
            models_dir.join(coreml_encoder).is_dir()
        };

        Ok(RuntimeHealth {
            whisper_cli_path: whisper_cli_configured,
            whisper_cli_resolved: whisper_cli_resolution.resolved_path,
            whisper_stream_path: whisper_stream_configured,
            whisper_stream_resolved: whisper_stream_resolution.resolved_path,
            models_dir_configured: configured_models_dir,
            models_dir_resolved: resolved_models_dir,
            model_filename,
            model_present,
            coreml_encoder_present,
            missing_models: missing_models(&models_dir),
            missing_encoders: missing_encoders(&models_dir),
        })
    }

    pub fn resolve_binary_path(&self, configured: &str, fallback: &str) -> String {
        self.resolve_binary_details(configured, fallback)
            .resolved_path
    }

    pub fn resolve_models_dir(&self, configured: &str) -> String {
        let trimmed = configured.trim();
        if trimmed.is_empty() {
            return self.data_dir.join("models").to_string_lossy().to_string();
        }

        if let Some(stripped) = trimmed.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home)
                    .join(stripped)
                    .to_string_lossy()
                    .to_string();
            }
        }

        let candidate = PathBuf::from(trimmed);
        if candidate.is_absolute() {
            return candidate.to_string_lossy().to_string();
        }

        self.data_dir.join(candidate).to_string_lossy().to_string()
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    fn resolve_binary_details(&self, configured: &str, fallback: &str) -> BinaryResolution {
        let configured_trimmed = configured.trim();

        if let Some(path) = self.find_binary_candidate(configured_trimmed) {
            return BinaryResolution {
                resolved_path: path.to_string_lossy().to_string(),
            };
        }

        if let Some(path) = self.find_binary_candidate(fallback) {
            return BinaryResolution {
                resolved_path: path.to_string_lossy().to_string(),
            };
        }

        let unresolved = if configured_trimmed.is_empty() {
            fallback.to_string()
        } else {
            configured_trimmed.to_string()
        };
        BinaryResolution {
            resolved_path: unresolved,
        }
    }

    fn find_binary_candidate(&self, value: &str) -> Option<PathBuf> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut candidates = Vec::<PathBuf>::new();
        let has_separator = trimmed.contains('/') || trimmed.contains('\\');
        let expanded = expand_home(trimmed);

        if has_separator {
            let path = PathBuf::from(&expanded);
            if path.is_absolute() {
                candidates.push(path);
            } else {
                candidates.push(self.data_dir.join(&path));
                candidates.push(path);
            }
        } else {
            candidates.push(self.data_dir.join("bin").join(trimmed));
            candidates.push(self.data_dir.join(trimmed));
            candidates.push(PathBuf::from("/opt/homebrew/bin").join(trimmed));
            candidates.push(PathBuf::from("/usr/local/bin").join(trimmed));
            candidates.push(PathBuf::from("/usr/bin").join(trimmed));

            if let Some(path_entries) = env::var_os("PATH") {
                for entry in env::split_paths(&path_entries) {
                    candidates.push(entry.join(trimmed));
                }
            }
        }

        let mut seen = HashSet::<PathBuf>::new();
        candidates
            .into_iter()
            .filter(|candidate| seen.insert(candidate.clone()))
            .find(|candidate| candidate.is_file())
    }

    fn migrate_models_dir_if_needed(&self, settings: &mut AppSettings) -> Result<(), String> {
        let current_models_dir =
            PathBuf::from(self.resolve_models_dir(&settings.transcription.models_dir));
        let Some(legacy_models_dir) = legacy_models_dir() else {
            return Ok(());
        };

        if current_models_dir == legacy_models_dir {
            return Ok(());
        }

        let current_missing_models = missing_models(&current_models_dir);
        let current_missing_encoders = missing_encoders(&current_models_dir);
        if current_missing_models.is_empty() && current_missing_encoders.is_empty() {
            return Ok(());
        }

        let legacy_missing_models = missing_models(&legacy_models_dir);
        let legacy_missing_encoders = missing_encoders(&legacy_models_dir);
        if !legacy_missing_models.is_empty() || !legacy_missing_encoders.is_empty() {
            return Ok(());
        }

        let migrated_models_dir = legacy_models_dir.to_string_lossy().to_string();
        settings.transcription.models_dir = migrated_models_dir.clone();
        settings.models_dir = migrated_models_dir.clone();
        settings.sync_sections_from_legacy();
        settings.sync_legacy_from_sections();

        self.settings_repo
            .save_sync(settings)
            .map_err(|e| format!("failed to persist migrated models path: {e}"))?;
        info!("migrated models directory to {}", migrated_models_dir);
        Ok(())
    }
}

#[derive(Clone)]
pub struct InfrastructureBundle {
    pub transcription_service: Arc<TranscriptionService>,
    pub artifact_service: Arc<ArtifactService>,
    pub settings_service: Arc<SettingsService>,
    pub runtime_factory: Arc<RuntimeTranscriptionFactory>,
}

pub fn bootstrap(data_dir: &Path) -> Result<InfrastructureBundle, String> {
    let runtime_factory = Arc::new(RuntimeTranscriptionFactory::new(data_dir)?);
    let transcription_service = runtime_factory.build_service()?;
    let artifact_service = runtime_factory.artifact_service();
    let settings_service = runtime_factory.settings_service();

    Ok(InfrastructureBundle {
        transcription_service,
        artifact_service,
        settings_service,
        runtime_factory,
    })
}

fn missing_models(models_dir: &Path) -> Vec<String> {
    REQUIRED_MODEL_FILES
        .iter()
        .filter_map(|filename| {
            if models_dir.join(filename).exists() {
                None
            } else {
                Some((*filename).to_string())
            }
        })
        .collect::<Vec<_>>()
}

fn missing_encoders(models_dir: &Path) -> Vec<String> {
    REQUIRED_COREML_ENCODERS
        .iter()
        .filter_map(|(_model, encoder_dir)| {
            if models_dir.join(encoder_dir).is_dir() {
                None
            } else {
                Some((*encoder_dir).to_string())
            }
        })
        .collect::<Vec<_>>()
}

fn encoder_for_model(model_filename: &str) -> Option<&'static str> {
    REQUIRED_COREML_ENCODERS
        .iter()
        .find(|(model, _encoder)| *model == model_filename)
        .map(|(_model, encoder)| *encoder)
}

fn legacy_models_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("sbobino")
            .join("models")
    })
}

fn expand_home(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join(stripped)
                .to_string_lossy()
                .to_string();
        }
    }
    path.to_string()
}
