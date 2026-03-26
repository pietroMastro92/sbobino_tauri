use std::future::Future;
use std::pin::Pin;

use tracing::warn;

use sbobino_application::{is_retryable_ai_provider_error, ApplicationError, TranscriptEnhancer};
use sbobino_infrastructure::AiEnhancerCandidate;

use crate::error::CommandError;

pub type EnhancerOperationFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ApplicationError>> + Send + 'a>>;

pub async fn run_with_enhancer_fallback<T, F>(
    candidates: &[AiEnhancerCandidate],
    operation_name: &str,
    operation: F,
) -> Result<T, ApplicationError>
where
    T: Send,
    F: for<'a> Fn(&'a dyn TranscriptEnhancer) -> EnhancerOperationFuture<'a, T>,
{
    let mut last_retryable_error: Option<ApplicationError> = None;

    for candidate in candidates {
        match operation(candidate.enhancer.as_ref()).await {
            Ok(result) => return Ok(result),
            Err(error) if is_retryable_ai_provider_error(&error) => {
                warn!(
                    "AI operation '{}' failed on provider '{}' and will fall back: {}",
                    operation_name, candidate.label, error
                );
                last_retryable_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_retryable_error.unwrap_or_else(|| {
        ApplicationError::PostProcessing(format!(
            "no AI provider was able to complete the '{}' operation",
            operation_name
        ))
    }))
}

pub fn missing_ai_provider_command_error(reason: Option<&str>) -> CommandError {
    let message = reason
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("No usable AI provider is configured in Settings > AI Services.");

    CommandError::new("missing_ai_provider", message)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use async_trait::async_trait;

    use sbobino_application::{dto::SummaryFaq, ApplicationError};
    use sbobino_infrastructure::AiEnhancerCandidate;

    use super::run_with_enhancer_fallback;

    struct StubEnhancer {
        calls: Arc<AtomicUsize>,
        result: Result<String, String>,
    }

    #[async_trait]
    impl sbobino_application::TranscriptEnhancer for StubEnhancer {
        async fn optimize(
            &self,
            _text: &str,
            _language_code: &str,
        ) -> Result<String, ApplicationError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.result
                .clone()
                .map_err(ApplicationError::PostProcessing)
        }

        async fn summarize_and_faq(
            &self,
            text: &str,
            _language_code: &str,
        ) -> Result<SummaryFaq, ApplicationError> {
            Ok(SummaryFaq {
                summary: text.to_string(),
                faqs: String::new(),
            })
        }
    }

    fn candidate(
        key: &str,
        label: &str,
        calls: Arc<AtomicUsize>,
        result: Result<String, String>,
    ) -> AiEnhancerCandidate {
        AiEnhancerCandidate {
            key: key.to_string(),
            label: label.to_string(),
            fallback: false,
            enhancer: Arc::new(StubEnhancer { calls, result }),
        }
    }

    #[tokio::test]
    async fn success_on_first_candidate_skips_fallbacks() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let candidates = vec![
            candidate(
                "remote",
                "Remote",
                first_calls.clone(),
                Ok("done".to_string()),
            ),
            candidate(
                "foundation",
                "Foundation",
                second_calls.clone(),
                Ok("fallback".to_string()),
            ),
        ];

        let result = run_with_enhancer_fallback(&candidates, "optimize", |enhancer| {
            Box::pin(async move { enhancer.optimize("text", "en").await })
        })
        .await
        .expect("first provider should succeed");

        assert_eq!(result, "done");
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn retryable_errors_fall_back_to_next_candidate() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let candidates = vec![
            candidate(
                "remote",
                "Remote",
                first_calls.clone(),
                Err("AI request failed: connection refused".to_string()),
            ),
            candidate(
                "foundation",
                "Foundation",
                second_calls.clone(),
                Ok("fallback-success".to_string()),
            ),
        ];

        let result = run_with_enhancer_fallback(&candidates, "optimize", |enhancer| {
            Box::pin(async move { enhancer.optimize("text", "en").await })
        })
        .await
        .expect("second provider should succeed");

        assert_eq!(result, "fallback-success");
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 1);
    }
}
