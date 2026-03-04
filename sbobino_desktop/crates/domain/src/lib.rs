pub mod artifact;
pub mod error;
pub mod job;
pub mod settings;

pub use artifact::{
    ArtifactKind, TimedSegment, TimedWord, TranscriptArtifact, TranscriptionOutput,
};
pub use error::DomainError;
pub use job::{JobProgress, JobStage, JobStatus, TranscriptionJob};
pub use settings::{
    default_prompt_templates, AiProvider, AiSettings, AppLanguage, AppSettings, AppearanceMode,
    GeneralSettings, LanguageCode, PromptBindings, PromptCategory, PromptSettings, PromptTask,
    PromptTemplate, RemoteServiceConfig, RemoteServiceKind, SpeechModel, TranscriptionEngine,
    TranscriptionSettings, WhisperOptions,
};
