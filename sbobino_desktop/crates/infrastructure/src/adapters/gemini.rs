use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use sbobino_application::{dto::SummaryFaq, ApplicationError, TranscriptEnhancer};

#[derive(Debug, Clone)]
pub struct GeminiEnhancer {
    client: Client,
    api_key: String,
    model: String,
    optimize_prompt_override: Option<String>,
    summary_prompt_override: Option<String>,
}

impl GeminiEnhancer {
    pub fn new(
        api_key: String,
        model: String,
        optimize_prompt_override: Option<String>,
        summary_prompt_override: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            optimize_prompt_override: normalize_prompt(optimize_prompt_override),
            summary_prompt_override: normalize_prompt(summary_prompt_override),
        }
    }

    async fn generate(&self, prompt: &str) -> Result<String, ApplicationError> {
        let endpoint = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self
            .client
            .post(endpoint)
            .json(&json!({
                "contents": [{
                    "parts": [{"text": prompt}]
                }],
                "generationConfig": {
                    "temperature": 0.3,
                    "topP": 0.95,
                    "maxOutputTokens": 4096
                }
            }))
            .send()
            .await
            .map_err(|e| ApplicationError::PostProcessing(format!("gemini request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ApplicationError::PostProcessing(format!(
                "gemini API returned {status}: {body}"
            )));
        }

        let payload: GeminiResponse = response.json().await.map_err(|e| {
            ApplicationError::PostProcessing(format!("invalid gemini response: {e}"))
        })?;

        payload
            .candidates
            .into_iter()
            .flat_map(|candidate| candidate.content.parts.into_iter())
            .find_map(|part| part.text)
            .ok_or_else(|| {
                ApplicationError::PostProcessing(
                    "gemini response did not contain generated text".to_string(),
                )
            })
    }

    pub async fn ask(&self, prompt: &str) -> Result<String, ApplicationError> {
        self.generate(prompt).await
    }

    pub async fn optimize_with_prompt(
        &self,
        text: &str,
        language_code: &str,
        prompt_override: Option<&str>,
    ) -> Result<String, ApplicationError> {
        let prompt = build_optimize_prompt(
            text,
            language_code,
            prompt_override,
            self.optimize_prompt_override.as_deref(),
        );
        self.generate(&prompt).await
    }

    pub async fn summarize_and_faq_with_prompt(
        &self,
        text: &str,
        language_code: &str,
        prompt_override: Option<&str>,
    ) -> Result<SummaryFaq, ApplicationError> {
        let prompt = build_summary_prompt(
            text,
            language_code,
            prompt_override,
            self.summary_prompt_override.as_deref(),
        );
        let output = self.generate(&prompt).await?;

        let (summary, faqs) = if let Some((left, right)) = output.split_once("FAQs:") {
            (
                left.replace("Summary:", "").trim().to_string(),
                right.trim().to_string(),
            )
        } else {
            (output.trim().to_string(), String::new())
        };

        Ok(SummaryFaq { summary, faqs })
    }
}

#[async_trait]
impl TranscriptEnhancer for GeminiEnhancer {
    async fn optimize(&self, text: &str, language_code: &str) -> Result<String, ApplicationError> {
        self.optimize_with_prompt(text, language_code, None).await
    }

    async fn summarize_and_faq(
        &self,
        text: &str,
        language_code: &str,
    ) -> Result<SummaryFaq, ApplicationError> {
        self.summarize_and_faq_with_prompt(text, language_code, None)
            .await
    }
}

fn normalize_prompt(value: Option<String>) -> Option<String> {
    value.and_then(|prompt| {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn build_optimize_prompt(
    text: &str,
    language_code: &str,
    prompt_override: Option<&str>,
    default_override: Option<&str>,
) -> String {
    if let Some(template) = prompt_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            default_override
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
    {
        return format!(
            "{template}\n\nLanguage: {language_code}\n\nTranscript:\n{text}\n\nReturn only the optimized text."
        );
    }

    format!(
        "Clean and optimize this transcript while preserving language {language_code}. Return only optimized text.\n\n{text}"
    )
}

fn build_summary_prompt(
    text: &str,
    language_code: &str,
    prompt_override: Option<&str>,
    default_override: Option<&str>,
) -> String {
    if let Some(template) = prompt_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            default_override
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
    {
        return format!(
            "{template}\n\nLanguage: {language_code}\n\nTranscript:\n{text}\n\nFormat strictly as:\nSummary:\n...\nFAQs:\nQ:...\nA:..."
        );
    }

    format!(
        "Generate in language {language_code}:\n1) Summary\n2) Exactly 3 FAQs with answers.\nFormat:\nSummary:\n...\nFAQs:\nQ:...\nA:...\n\nText:\n{text}"
    )
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    #[serde(default)]
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    #[serde(default)]
    text: Option<String>,
}
