use serde::Serialize;

use sbobino_application::ApplicationError;

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<ApplicationError> for CommandError {
    fn from(value: ApplicationError) -> Self {
        let code = match value {
            ApplicationError::Validation(_) => "validation",
            ApplicationError::AudioTranscoding(_) => "audio_transcoding",
            ApplicationError::SpeechToText(_) => "speech_to_text",
            ApplicationError::SpeakerDiarization(_) => "speaker_diarization",
            ApplicationError::PostProcessing(_) => "post_processing",
            ApplicationError::Persistence(_) => "persistence",
            ApplicationError::Settings(_) => "settings",
            ApplicationError::Cancelled => "cancelled",
        }
        .to_string();

        Self {
            code,
            message: value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CommandError;
    use sbobino_application::ApplicationError;

    #[test]
    fn maps_validation_error_to_validation_code() {
        let command_error = CommandError::from(ApplicationError::Validation("bad input".into()));
        assert_eq!(command_error.code, "validation");
        assert!(command_error.message.contains("bad input"));
    }

    #[test]
    fn maps_persistence_error_to_persistence_code() {
        let command_error = CommandError::from(ApplicationError::Persistence("db down".into()));
        assert_eq!(command_error.code, "persistence");
        assert!(command_error.message.contains("db down"));
    }

    #[test]
    fn maps_cancelled_error_to_cancelled_code() {
        let command_error = CommandError::from(ApplicationError::Cancelled);
        assert_eq!(command_error.code, "cancelled");
        assert!(command_error.message.contains("cancelled"));
    }
}
