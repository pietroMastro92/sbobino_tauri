use std::{collections::HashMap, sync::Arc};

use sbobino_application::{ArtifactService, SettingsService};
use sbobino_infrastructure::{
    adapters::whisper_stream::WhisperStreamEngine, RuntimeTranscriptionFactory,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct TranscriptionTask {
    pub cancel_token: CancellationToken,
}

#[derive(Clone)]
pub struct RealtimeRuntime {
    pub engine: WhisperStreamEngine,
    pub session_name: Arc<Mutex<Option<String>>>,
    pub model_filename: Arc<Mutex<Option<String>>>,
    pub language_code: Arc<Mutex<String>>,
}

#[derive(Clone)]
pub struct ProvisioningRuntime {
    pub cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}

#[derive(Clone)]
pub struct AppState {
    pub artifact_service: Arc<ArtifactService>,
    pub settings_service: Arc<SettingsService>,
    pub runtime_factory: Arc<RuntimeTranscriptionFactory>,
    pub transcription_tasks: Arc<Mutex<HashMap<String, TranscriptionTask>>>,
    pub realtime: RealtimeRuntime,
    pub provisioning: ProvisioningRuntime,
}
