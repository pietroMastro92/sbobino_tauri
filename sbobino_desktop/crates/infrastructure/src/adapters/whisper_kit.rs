use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use sbobino_application::{ApplicationError, SpeechToTextEngine};
use sbobino_domain::WhisperOptions;

#[derive(Debug, Clone)]
pub struct WhisperKitEngine {
    binary_path: String,
    models_dir: String,
}

impl WhisperKitEngine {
    pub fn new(binary_path: String, models_dir: String) -> Self {
        Self {
            binary_path,
            models_dir,
        }
    }

    fn resolve_model_name(model_filename: &str) -> Result<&'static str, ApplicationError> {
        let basename = Path::new(model_filename)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(model_filename)
            .trim()
            .to_ascii_lowercase();

        if basename.contains("large-v3-turbo") {
            return Ok("large-v3-turbo");
        }
        if basename == "medium"
            || basename.starts_with("medium-")
            || basename.contains("ggml-medium")
        {
            return Ok("medium");
        }
        if basename == "small" || basename.starts_with("small-") || basename.contains("ggml-small")
        {
            return Ok("small");
        }
        if basename == "base" || basename.starts_with("base-") || basename.contains("ggml-base") {
            return Ok("base");
        }
        if basename == "tiny" || basename.starts_with("tiny-") || basename.contains("ggml-tiny") {
            return Ok("tiny");
        }

        Err(ApplicationError::SpeechToText(format!(
            "unsupported WhisperKit model mapping for '{}'. Expected tiny, base, small, medium, or large-v3-turbo ggml filenames.",
            model_filename
        )))
    }

    fn map_compute_units(value: &str) -> &'static str {
        match value.trim().to_ascii_lowercase().as_str() {
            "all" => "all",
            "cpu_only" | "cpuonly" => "cpuOnly",
            "cpu_and_gpu" | "cpuandgpu" => "cpuAndGPU",
            "cpu_and_neural_engine" | "cpuandneuralengine" => "cpuAndNeuralEngine",
            "random" => "random",
            _ => "cpuAndNeuralEngine",
        }
    }

    fn map_chunking_strategy(value: &str) -> &'static str {
        if value.trim().eq_ignore_ascii_case("none") {
            "none"
        } else {
            "vad"
        }
    }

    fn wav_duration_seconds(wav_path: &Path) -> Option<f32> {
        let reader = hound::WavReader::open(wav_path).ok()?;
        let spec = reader.spec();
        if spec.channels == 0 || spec.sample_rate == 0 {
            return None;
        }

        let samples = reader.duration() as f32;
        let frames = samples / f32::from(spec.channels);
        if frames <= 0.0 {
            return None;
        }

        Some(frames / (spec.sample_rate as f32))
    }

    fn strip_ansi_codes(input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\u{001b}' {
                if matches!(chars.peek(), Some('[')) {
                    let _ = chars.next();
                    while let Some(next) = chars.next() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                continue;
            }
            output.push(ch);
        }

        output
    }

    fn sanitize_output_line(raw_line: &str) -> String {
        Self::strip_ansi_codes(raw_line)
            .replace('\u{0008}', "")
            .trim()
            .to_string()
    }

    fn parse_progress_percent(line: &str) -> Option<f32> {
        if !line.contains('%') || !line.contains("Elapsed Time:") {
            return None;
        }

        line.split_whitespace()
            .find_map(|token| token.strip_suffix('%'))
            .and_then(|value| value.trim().parse::<f32>().ok())
            .map(|value| value.clamp(0.0, 100.0))
    }

    fn process_stream_line(
        raw_line: &str,
        collected_lines: &mut Vec<String>,
        track_progress: bool,
        total_audio_seconds: Option<f32>,
        last_progress_percent: &mut Option<f32>,
        emit_progress_seconds: &Arc<dyn Fn(f32) + Send + Sync>,
    ) {
        let cleaned = Self::sanitize_output_line(raw_line);
        if cleaned.is_empty() {
            return;
        }

        if track_progress {
            if let Some(percent) = Self::parse_progress_percent(&cleaned) {
                let should_emit = match *last_progress_percent {
                    Some(last) => percent > last + 0.2,
                    None => true,
                };
                if should_emit {
                    *last_progress_percent = Some(percent);
                    if let Some(total) = total_audio_seconds {
                        emit_progress_seconds((total * (percent / 100.0)).clamp(0.0, total));
                    }
                }
            }
        }

        collected_lines.push(cleaned);
    }

    async fn consume_stream<R>(
        reader: R,
        track_progress: bool,
        total_audio_seconds: Option<f32>,
        emit_progress_seconds: Arc<dyn Fn(f32) + Send + Sync>,
    ) -> Result<Vec<String>, ApplicationError>
    where
        R: AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(reader);
        let mut chunk = [0_u8; 4096];
        let mut pending = Vec::<u8>::new();
        let mut collected_lines = Vec::<String>::new();
        let mut last_progress_percent: Option<f32> = None;

        loop {
            let read = reader.read(&mut chunk).await.map_err(|e| {
                ApplicationError::SpeechToText(format!("failed to read whisperkit-cli stream: {e}"))
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
                        Self::process_stream_line(
                            &raw,
                            &mut collected_lines,
                            track_progress,
                            total_audio_seconds,
                            &mut last_progress_percent,
                            &emit_progress_seconds,
                        );
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
            Self::process_stream_line(
                &raw,
                &mut collected_lines,
                track_progress,
                total_audio_seconds,
                &mut last_progress_percent,
                &emit_progress_seconds,
            );
        }

        Ok(collected_lines)
    }

    fn extract_verbose_transcript_section(lines: &[String]) -> Option<String> {
        let marker_index = lines
            .iter()
            .rposition(|line| line.trim_start().starts_with("Transcription of "))?;

        let mut transcript_lines = Vec::<String>::new();
        for line in lines.iter().skip(marker_index + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("Processing transcription result for:")
                || trimmed.starts_with("Transcription of ")
            {
                break;
            }

            transcript_lines.push(trimmed.to_string());
        }

        let transcript = transcript_lines.join("\n").trim().to_string();
        if transcript.is_empty() {
            None
        } else {
            Some(transcript)
        }
    }

    fn fallback_non_empty_join(lines: &[String]) -> Option<String> {
        let joined = lines
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let transcript = joined.trim().to_string();
        if transcript.is_empty() {
            None
        } else {
            Some(transcript)
        }
    }

    async fn transcribe_with_cli(
        &self,
        input_wav: &Path,
        model_filename: &str,
        language_code: &str,
        options: &WhisperOptions,
        emit_partial: Arc<dyn Fn(String) + Send + Sync>,
        emit_progress_seconds: Arc<dyn Fn(f32) + Send + Sync>,
    ) -> Result<String, ApplicationError> {
        let model_name = Self::resolve_model_name(model_filename)?;
        tokio::fs::create_dir_all(&self.models_dir)
            .await
            .map_err(|e| {
                ApplicationError::SpeechToText(format!(
                    "failed to ensure WhisperKit models directory '{}': {e}",
                    self.models_dir
                ))
            })?;

        let total_audio_seconds = Self::wav_duration_seconds(input_wav);
        let mut command = Command::new(&self.binary_path);
        command
            .kill_on_drop(true)
            .arg("transcribe")
            .arg("--audio-path")
            .arg(input_wav)
            .arg("--model")
            .arg(model_name)
            .arg("--download-model-path")
            .arg(&self.models_dir)
            .arg("--download-tokenizer-path")
            .arg(&self.models_dir)
            .arg("--task")
            .arg(if options.translate_to_english {
                "translate"
            } else {
                "transcribe"
            })
            .arg("--temperature")
            .arg(options.temperature.to_string())
            .arg("--temperature-increment-on-fallback")
            .arg(options.temperature_increment_on_fallback.to_string())
            .arg("--temperature-fallback-count")
            .arg(options.temperature_fallback_count.to_string())
            .arg("--best-of")
            .arg(options.best_of.to_string())
            .arg("--logprob-threshold")
            .arg(options.logprob_threshold.to_string())
            .arg("--first-token-log-prob-threshold")
            .arg(options.first_token_logprob_threshold.to_string())
            .arg("--no-speech-threshold")
            .arg(options.no_speech_threshold.to_string())
            .arg("--concurrent-worker-count")
            .arg(options.concurrent_worker_count.to_string())
            .arg("--chunking-strategy")
            .arg(Self::map_chunking_strategy(&options.chunking_strategy))
            .arg("--audio-encoder-compute-units")
            .arg(Self::map_compute_units(
                &options.audio_encoder_compute_units,
            ))
            .arg("--text-decoder-compute-units")
            .arg(Self::map_compute_units(&options.text_decoder_compute_units))
            .arg("--verbose")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if !language_code.trim().is_empty() && language_code != "auto" {
            command.arg("--language").arg(language_code);
        }

        if options.use_prefill_prompt {
            command.arg("--use-prefill-prompt");
        }
        if options.use_prefill_cache {
            command.arg("--use-prefill-cache");
        }
        if options.without_timestamps {
            command.arg("--without-timestamps");
        }
        if options.word_timestamps {
            command.arg("--word-timestamps");
        }
        if let Some(prompt) = options
            .prompt
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            command.arg("--prompt").arg(prompt);
        }

        let mut child = command.spawn().map_err(|e| {
            ApplicationError::SpeechToText(format!(
                "whisperkit-cli failed to start at '{}': {e}. Configure WhisperKit CLI path in Settings > Local Models.",
                self.binary_path
            ))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            ApplicationError::SpeechToText("missing whisperkit-cli stdout pipe".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ApplicationError::SpeechToText("missing whisperkit-cli stderr pipe".to_string())
        })?;

        let stdout_progress = emit_progress_seconds.clone();
        let stdout_task = tokio::spawn(async move {
            Self::consume_stream(stdout, true, total_audio_seconds, stdout_progress).await
        });

        let stderr_progress = emit_progress_seconds.clone();
        let stderr_task = tokio::spawn(async move {
            Self::consume_stream(stderr, false, None, stderr_progress).await
        });

        let status = match timeout(Duration::from_secs(900), child.wait()).await {
            Ok(wait_result) => wait_result.map_err(|e| {
                ApplicationError::SpeechToText(format!("failed to wait for whisperkit-cli: {e}"))
            })?,
            Err(_) => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(ApplicationError::SpeechToText(
                    "whisperkit-cli timed out after 900s".to_string(),
                ));
            }
        };

        let stdout_lines = stdout_task.await.map_err(|e| {
            ApplicationError::SpeechToText(format!("stdout reader task failed: {e}"))
        })??;
        let stderr_lines = stderr_task.await.map_err(|e| {
            ApplicationError::SpeechToText(format!("stderr reader task failed: {e}"))
        })??;

        if !status.success() {
            let stderr_output = stderr_lines.join("\n").trim().to_string();
            let stdout_output = stdout_lines.join("\n").trim().to_string();
            let details = if !stderr_output.is_empty() {
                stderr_output
            } else {
                stdout_output
            };

            return Err(ApplicationError::SpeechToText(format!(
                "whisperkit-cli failed: {}",
                details
            )));
        }

        let transcript = Self::extract_verbose_transcript_section(&stdout_lines)
            .or_else(|| Self::fallback_non_empty_join(&stdout_lines))
            .or_else(|| Self::fallback_non_empty_join(&stderr_lines))
            .ok_or_else(|| {
                ApplicationError::SpeechToText("whisperkit-cli produced empty output".to_string())
            })?;

        for line in transcript.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                emit_partial(trimmed.to_string());
            }
        }

        Ok(transcript)
    }
}

#[async_trait]
impl SpeechToTextEngine for WhisperKitEngine {
    async fn transcribe(
        &self,
        input_wav: &Path,
        model_filename: &str,
        language_code: &str,
        options: &WhisperOptions,
        emit_partial: Arc<dyn Fn(String) + Send + Sync>,
        emit_progress_seconds: Arc<dyn Fn(f32) + Send + Sync>,
    ) -> Result<String, ApplicationError> {
        self.transcribe_with_cli(
            input_wav,
            model_filename,
            language_code,
            options,
            emit_partial,
            emit_progress_seconds,
        )
        .await
    }
}
