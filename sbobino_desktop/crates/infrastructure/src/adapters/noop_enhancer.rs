use async_trait::async_trait;

use sbobino_application::{dto::SummaryFaq, ApplicationError, TranscriptEnhancer};

#[derive(Debug, Clone)]
pub struct NoopEnhancer;

#[async_trait]
impl TranscriptEnhancer for NoopEnhancer {
    async fn optimize(&self, text: &str, _language_code: &str) -> Result<String, ApplicationError> {
        Ok(text.to_string())
    }

    async fn summarize_and_faq(
        &self,
        _text: &str,
        _language_code: &str,
    ) -> Result<SummaryFaq, ApplicationError> {
        Ok(SummaryFaq {
            summary: String::new(),
            faqs: String::new(),
        })
    }
}
