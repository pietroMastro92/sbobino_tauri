use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("input path cannot be empty")]
    EmptyInputPath,
    #[error("invalid model selection: {0}")]
    InvalidModel(String),
    #[error("transcript content is empty")]
    EmptyTranscript,
}
