use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("audio transcoding failed: {0}")]
    AudioTranscoding(String),
    #[error("speech-to-text failed: {0}")]
    SpeechToText(String),
    #[error("speaker diarization failed: {0}")]
    SpeakerDiarization(String),
    #[error("post-processing failed: {0}")]
    PostProcessing(String),
    #[error("persistence failed: {0}")]
    Persistence(String),
    #[error("settings access failed: {0}")]
    Settings(String),
    #[error("transcription cancelled")]
    Cancelled,
}

pub fn is_retryable_ai_provider_error(error: &ApplicationError) -> bool {
    let ApplicationError::PostProcessing(message) = error else {
        return false;
    };

    let text = message.to_lowercase();
    if text.contains("context window")
        || text.contains("model context window")
        || text.contains("context length")
        || text.contains("prompt is too long")
        || text.contains("token limit")
    {
        return false;
    }

    text.contains("request failed")
        || text.contains("provider returned 401")
        || text.contains("provider returned 403")
        || text.contains("provider returned 408")
        || text.contains("provider returned 409")
        || text.contains("provider returned 429")
        || text.contains("provider returned 5")
        || text.contains("api returned 401")
        || text.contains("api returned 403")
        || text.contains("api returned 408")
        || text.contains("api returned 409")
        || text.contains("api returned 429")
        || text.contains("api returned 5")
        || text.contains("service unavailable")
        || text.contains("temporarily unavailable")
        || text.contains("timed out")
        || text.contains("timeout")
        || text.contains("connection")
        || text.contains("network")
        || text.contains("refused")
        || text.contains("unreachable")
        || text.contains("foundation model request failed")
        || text.contains("foundation bridge error")
        || text.contains("invalid ai provider response")
        || text.contains("invalid gemini response")
        || text.contains("response did not contain generated text")
        || text.contains("availability")
}
