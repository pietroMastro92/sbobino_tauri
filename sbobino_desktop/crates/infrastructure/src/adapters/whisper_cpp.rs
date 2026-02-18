use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio::fs;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use sbobino_application::{ApplicationError, SpeechToTextEngine};

#[derive(Debug, Clone)]
pub struct WhisperCppEngine {
    binary_path: String,
    models_dir: String,
}

#[derive(Default)]
struct TranscriptCollector {
    lines: Vec<String>,
}

impl WhisperCppEngine {
    pub fn new(binary_path: String, models_dir: String) -> Self {
        Self {
            binary_path,
            models_dir,
        }
    }

    fn model_path(&self, model_filename: &str) -> PathBuf {
        Path::new(&self.models_dir).join(model_filename)
    }

    fn validate_model_exists(&self, model_filename: &str) -> Result<PathBuf, ApplicationError> {
        let model_path = self.model_path(model_filename);
        if model_path.exists() {
            return Ok(model_path);
        }

        let download_url =
            format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{model_filename}");
        Err(ApplicationError::SpeechToText(format!(
            "model file not found at {}. Download it from {}",
            model_path.display(),
            download_url
        )))
    }

    fn parse_cli_line(raw_line: &str) -> Option<String> {
        let cleaned = raw_line
            .replace("\u{001b}[2K", "")
            .replace("\u{001b}[0m", "")
            .replace("[2K]", "")
            .replace("[BLANK_AUDIO]", "")
            .trim()
            .to_string();

        if cleaned.is_empty() {
            return None;
        }

        const NOISE_PREFIXES: [&str; 10] = [
            "init:",
            "main:",
            "whisper_",
            "ggml_",
            "system_info:",
            "output_",
            "sampling_",
            "encode",
            "decode",
            "progress",
        ];

        if NOISE_PREFIXES
            .iter()
            .any(|prefix| cleaned.starts_with(prefix))
        {
            return None;
        }

        let without_timestamp = if cleaned.starts_with('[') {
            match cleaned.find(']') {
                Some(end_index) => {
                    let bracket_content = cleaned[1..end_index].trim();
                    if bracket_content.contains("-->") {
                        cleaned[end_index + 1..].trim().to_string()
                    } else {
                        cleaned
                    }
                }
                None => cleaned,
            }
        } else {
            cleaned
        };

        let normalized = without_timestamp.trim().to_string();
        if normalized.is_empty() {
            return None;
        }

        Some(normalized)
    }

    fn collect_line(
        collector: &Arc<Mutex<TranscriptCollector>>,
        emit_partial: &Arc<dyn Fn(String) + Send + Sync>,
        line: String,
    ) {
        if let Ok(mut state) = collector.lock() {
            state.lines.push(line.clone());
        }

        emit_partial(line);
    }

    async fn consume_stream<R>(
        reader: R,
        collector: Arc<Mutex<TranscriptCollector>>,
        emit_partial: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<Vec<String>, ApplicationError>
    where
        R: AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(reader);
        let mut chunk = [0_u8; 4096];
        let mut pending = Vec::<u8>::new();
        let mut raw_lines = Vec::<String>::new();

        loop {
            let read = reader.read(&mut chunk).await.map_err(|e| {
                ApplicationError::SpeechToText(format!("failed to read whisper-cli stream: {e}"))
            })?;

            if read == 0 {
                break;
            }

            pending.extend_from_slice(&chunk[..read]);
            let mut start = 0_usize;
            let mut index = 0_usize;

            while index < pending.len() {
                if pending[index] == b'\n' || pending[index] == b'\r' {
                    if index > start {
                        let raw = String::from_utf8_lossy(&pending[start..index]).to_string();
                        raw_lines.push(raw.clone());
                        if let Some(parsed_line) = Self::parse_cli_line(&raw) {
                            Self::collect_line(&collector, &emit_partial, parsed_line);
                        }
                    }

                    index += 1;
                    while index < pending.len()
                        && (pending[index] == b'\n' || pending[index] == b'\r')
                    {
                        index += 1;
                    }
                    start = index;
                    continue;
                }
                index += 1;
            }

            if start > 0 {
                pending.drain(..start);
            }
        }

        if !pending.is_empty() {
            let raw = String::from_utf8_lossy(&pending).to_string();
            raw_lines.push(raw.clone());
            if let Some(parsed_line) = Self::parse_cli_line(&raw) {
                Self::collect_line(&collector, &emit_partial, parsed_line);
            }
        }

        Ok(raw_lines)
    }

    async fn transcribe_with_cli(
        &self,
        input_wav: &Path,
        model_path: &Path,
        language_code: &str,
        emit_partial: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<String, ApplicationError> {
        let output_base = std::env::temp_dir().join(format!(
            "sbobino-whisper-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or(0)
        ));
        let output_txt_path = output_base.with_extension("txt");

        let mut command = Command::new(&self.binary_path);
        command
            .kill_on_drop(true)
            .arg("-m")
            .arg(model_path)
            .arg("-f")
            .arg(input_wav);

        if language_code != "auto" {
            command.arg("-l").arg(language_code);
        }

        command
            .arg("-et")
            .arg("2.5")
            .arg("-otxt")
            .arg("-of")
            .arg(&output_base)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = command.spawn().map_err(|e| {
            ApplicationError::SpeechToText(format!(
                "whisper-cli failed to start at '{}': {e}. Configure Whisper CLI path in Settings > Local Models.",
                self.binary_path
            ))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            ApplicationError::SpeechToText("missing whisper-cli stdout pipe".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ApplicationError::SpeechToText("missing whisper-cli stderr pipe".to_string())
        })?;

        let collected = Arc::new(Mutex::new(TranscriptCollector::default()));

        let stdout_emit = emit_partial.clone();
        let stdout_collector = collected.clone();
        let stdout_task = tokio::spawn(async move {
            Self::consume_stream(stdout, stdout_collector, stdout_emit).await
        });

        let stderr_emit = emit_partial.clone();
        let stderr_collector = collected.clone();
        let stderr_task = tokio::spawn(async move {
            Self::consume_stream(stderr, stderr_collector, stderr_emit).await
        });

        let status = match timeout(Duration::from_secs(900), child.wait()).await {
            Ok(wait_result) => wait_result.map_err(|e| {
                ApplicationError::SpeechToText(format!("failed to wait for whisper-cli: {e}"))
            })?,
            Err(_) => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(ApplicationError::SpeechToText(
                    "whisper-cli timed out after 900s".to_string(),
                ));
            }
        };

        let _stdout_lines = stdout_task.await.map_err(|e| {
            ApplicationError::SpeechToText(format!("stdout reader task failed: {e}"))
        })??;

        let stderr_lines = stderr_task.await.map_err(|e| {
            ApplicationError::SpeechToText(format!("stderr reader task failed: {e}"))
        })??;
        let stderr_output = stderr_lines.join("\n");

        let transcript_lines = if let Ok(state) = collected.lock() {
            state.lines.clone()
        } else {
            Vec::new()
        };

        if !status.success() {
            return Err(ApplicationError::SpeechToText(format!(
                "whisper-cli failed: {}",
                stderr_output.trim()
            )));
        }

        let transcript_from_file = match fs::read_to_string(&output_txt_path).await {
            Ok(content) => {
                let cleaned = content.trim().to_string();
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            }
            Err(_) => None,
        };

        let transcript =
            transcript_from_file.unwrap_or_else(|| transcript_lines.join("\n").trim().to_string());

        let _ = fs::remove_file(&output_txt_path).await;

        if transcript.is_empty() {
            return Err(ApplicationError::SpeechToText(
                "whisper-cli produced empty output".to_string(),
            ));
        }

        Ok(transcript)
    }
}

#[async_trait]
impl SpeechToTextEngine for WhisperCppEngine {
    async fn transcribe(
        &self,
        input_wav: &Path,
        model_filename: &str,
        language_code: &str,
        emit_partial: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Result<String, ApplicationError> {
        let model_path = self.validate_model_exists(model_filename)?;
        self.transcribe_with_cli(input_wav, &model_path, language_code, emit_partial)
            .await
    }
}
