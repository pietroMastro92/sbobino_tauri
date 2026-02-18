use std::path::Path;

use async_trait::async_trait;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use sbobino_application::{ApplicationError, AudioTranscoder};

#[derive(Debug, Clone)]
pub struct FfmpegAdapter {
    binary_path: String,
}

impl FfmpegAdapter {
    pub fn new(binary_path: String) -> Self {
        Self { binary_path }
    }
}

#[async_trait]
impl AudioTranscoder for FfmpegAdapter {
    async fn to_wav_mono_16k(&self, input: &Path, output: &Path) -> Result<(), ApplicationError> {
        let mut command = Command::new(&self.binary_path);
        command
            .kill_on_drop(true)
            .arg("-y")
            .arg("-i")
            .arg(input)
            .arg("-ar")
            .arg("16000")
            .arg("-ac")
            .arg("1")
            .arg("-codec")
            .arg("pcm_s16le")
            .arg(output);

        let output = timeout(Duration::from_secs(300), command.output())
            .await
            .map_err(|_| {
                ApplicationError::AudioTranscoding(
                    "ffmpeg conversion timed out after 300s".to_string(),
                )
            })?
            .map_err(|e| {
                ApplicationError::AudioTranscoding(format!(
                    "ffmpeg process failed to start ({}) : {e}",
                    self.binary_path
                ))
            })?;

        if !output.status.success() {
            return Err(ApplicationError::AudioTranscoding(format!(
                "ffmpeg conversion failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}
