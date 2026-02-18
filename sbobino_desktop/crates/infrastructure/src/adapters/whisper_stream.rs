use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use sbobino_application::{ApplicationError, RealtimeDelta, RealtimeDeltaKind};

#[derive(Default)]
struct StreamState {
    child: Option<Child>,
    reader_task: Option<JoinHandle<()>>,
    lines: Vec<String>,
    preview: String,
    paused: bool,
    running: bool,
}

#[derive(Clone)]
pub struct WhisperStreamEngine {
    binary_path: String,
    models_dir: String,
    state: Arc<Mutex<StreamState>>,
}

impl WhisperStreamEngine {
    pub fn new(binary_path: String, models_dir: String) -> Self {
        Self {
            binary_path,
            models_dir,
            state: Arc::new(Mutex::new(StreamState::default())),
        }
    }

    fn model_path(&self, model_filename: &str) -> PathBuf {
        Path::new(&self.models_dir).join(model_filename)
    }

    fn clean_line(line: &str) -> String {
        let ansi_replaced = line
            .replace("\u{001b}[2K", "")
            .replace("\u{001b}[0m", "")
            .replace("[2K]", "")
            .replace("[BLANK_AUDIO]", "");
        ansi_replaced.trim().to_string()
    }

    fn should_skip_line(text: &str) -> bool {
        const PREFIXES: [&str; 8] = [
            "init:",
            "whisper_init",
            "whisper_model_load:",
            "whisper_backend_init",
            "ggml_metal_init:",
            "main:",
            "whisper_full_with_state:",
            "whisper_print",
        ];

        PREFIXES.iter().any(|prefix| text.starts_with(prefix))
    }

    pub async fn start(
        &self,
        model_filename: &str,
        language_code: &str,
        emit_delta: Arc<dyn Fn(RealtimeDelta) + Send + Sync>,
    ) -> Result<(), ApplicationError> {
        let mut state = self.state.lock().await;
        if state.running {
            return Err(ApplicationError::Validation(
                "realtime transcription is already running".to_string(),
            ));
        }

        let model_path = self.model_path(model_filename);
        if !model_path.exists() {
            return Err(ApplicationError::SpeechToText(format!(
                "realtime model file not found at {}",
                model_path.display()
            )));
        }

        let mut command = Command::new(&self.binary_path);
        command
            .kill_on_drop(true)
            .arg("-m")
            .arg(&model_path)
            .arg("-t")
            .arg("8")
            .arg("--step")
            .arg("500")
            .arg("--length")
            .arg("5000")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if language_code != "auto" {
            command.arg("-l").arg(language_code);
        }

        let mut child = command.spawn().map_err(|e| {
            ApplicationError::SpeechToText(format!(
                "failed to start realtime whisper stream ({}) : {e}",
                self.binary_path
            ))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            ApplicationError::SpeechToText("missing realtime stdout pipe".to_string())
        })?;

        let shared_state = self.state.clone();
        let reader_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();

            while let Ok(Some(raw_line)) = lines.next_line().await {
                let is_preview = raw_line.contains("[2K]") || raw_line.contains("\u{001b}[2K");
                let cleaned = Self::clean_line(&raw_line);
                if cleaned.is_empty() || Self::should_skip_line(&cleaned) {
                    continue;
                }

                let mut state = shared_state.lock().await;
                if state.paused {
                    continue;
                }

                if is_preview {
                    state.preview = cleaned.clone();
                    emit_delta(RealtimeDelta {
                        kind: RealtimeDeltaKind::UpdatePreview,
                        text: cleaned,
                    });
                    continue;
                }

                if state.lines.last().is_some_and(|last| last == &cleaned) {
                    continue;
                }

                state.lines.push(cleaned.clone());
                state.preview.clear();
                emit_delta(RealtimeDelta {
                    kind: RealtimeDeltaKind::AppendFinal,
                    text: cleaned,
                });
            }

            let mut state = shared_state.lock().await;
            state.running = false;
            state.paused = false;
        });

        state.child = Some(child);
        state.reader_task = Some(reader_task);
        state.running = true;
        state.paused = false;

        Ok(())
    }

    pub async fn pause(&self) -> Result<(), ApplicationError> {
        let mut state = self.state.lock().await;
        if !state.running {
            return Err(ApplicationError::Validation(
                "realtime transcription is not running".to_string(),
            ));
        }
        state.paused = true;
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), ApplicationError> {
        let mut state = self.state.lock().await;
        if !state.running {
            return Err(ApplicationError::Validation(
                "realtime transcription is not running".to_string(),
            ));
        }
        state.paused = false;
        Ok(())
    }

    pub async fn stop(&self) -> Result<String, ApplicationError> {
        let mut state = self.state.lock().await;
        if let Some(child) = &mut state.child {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        if let Some(task) = state.reader_task.take() {
            task.abort();
        }

        state.child = None;
        state.running = false;
        state.paused = false;

        let consolidated = state.lines.join("\n");
        Ok(consolidated)
    }

    pub async fn is_running(&self) -> bool {
        self.state.lock().await.running
    }

    pub async fn is_paused(&self) -> bool {
        self.state.lock().await.paused
    }

    pub async fn seed_buffer(&self, text: &str) {
        let mut state = self.state.lock().await;
        state.lines = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect();
        state.preview.clear();
    }

    pub async fn snapshot_text(&self) -> String {
        self.state.lock().await.lines.join("\n")
    }

    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.lines.clear();
        state.preview.clear();
    }
}
