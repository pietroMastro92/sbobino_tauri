use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("audio transcoding failed: {0}")]
    AudioTranscoding(String),
    #[error("speech-to-text failed: {0}")]
    SpeechToText(String),
    #[error("post-processing failed: {0}")]
    PostProcessing(String),
    #[error("persistence failed: {0}")]
    Persistence(String),
    #[error("settings access failed: {0}")]
    Settings(String),
    #[error("transcription cancelled")]
    Cancelled,
}
