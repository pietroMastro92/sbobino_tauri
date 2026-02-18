use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    File,
    Realtime,
}

impl ArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Realtime => "realtime",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptArtifact {
    pub id: String,
    pub job_id: String,
    pub title: String,
    pub kind: ArtifactKind,
    pub input_path: String,
    pub raw_transcript: String,
    pub optimized_transcript: String,
    pub summary: String,
    pub faqs: String,
    pub metadata: BTreeMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TranscriptArtifact {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: impl Into<String>,
        title: impl Into<String>,
        kind: ArtifactKind,
        input_path: impl Into<String>,
        raw_transcript: impl Into<String>,
        optimized_transcript: impl Into<String>,
        summary: impl Into<String>,
        faqs: impl Into<String>,
        metadata: BTreeMap<String, String>,
    ) -> Result<Self, DomainError> {
        let raw_transcript = raw_transcript.into();
        if raw_transcript.trim().is_empty() {
            return Err(DomainError::EmptyTranscript);
        }

        let optimized_transcript = optimized_transcript.into();
        let input_path = input_path.into();
        let now = Utc::now();
        let title = title.into();
        let title = if title.trim().is_empty() {
            input_path.clone()
        } else {
            title
        };

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            job_id: job_id.into(),
            title,
            kind,
            input_path,
            raw_transcript,
            optimized_transcript: if optimized_transcript.trim().is_empty() {
                String::new()
            } else {
                optimized_transcript
            },
            summary: summary.into(),
            faqs: faqs.into(),
            metadata,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
