#![cfg(unix)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use tempfile::tempdir;

use sbobino_application::{ApplicationError, SpeechToTextEngine};
use sbobino_infrastructure::adapters::whisper_cpp::WhisperCppEngine;

fn write_executable_script(path: &Path, content: &str) {
    std::fs::write(path, content).expect("failed to write script");

    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .expect("failed to read script metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("failed to chmod script");
}

#[tokio::test]
async fn transcribe_collects_lines_from_stdout_and_stderr() {
    let temp = tempdir().expect("failed to create temp dir");
    let script_path = temp.path().join("whisper-cli");
    let models_dir = temp.path().join("models");
    let input_wav = temp.path().join("audio.wav");

    std::fs::create_dir_all(&models_dir).expect("failed to create models dir");
    std::fs::write(models_dir.join("ggml-base.bin"), b"fake model")
        .expect("failed to create model");
    std::fs::write(&input_wav, b"RIFF....WAVE").expect("failed to create input wav");

    write_executable_script(
        &script_path,
        r#"#!/bin/sh
echo "init: loading model"
echo "[00:00:00.000 --> 00:00:01.000] first line"
echo "[00:00:01.000 --> 00:00:02.000] second line" 1>&2
echo "- third line"
exit 0
"#,
    );

    let engine = WhisperCppEngine::new(
        script_path.to_string_lossy().to_string(),
        models_dir.to_string_lossy().to_string(),
    );

    let emitted: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let emitted_clone = emitted.clone();

    let transcript = engine
        .transcribe(
            &input_wav,
            "ggml-base.bin",
            "en",
            Arc::new(move |line: String| {
                emitted_clone.lock().expect("emit lock poisoned").push(line);
            }),
        )
        .await
        .expect("transcription should succeed");

    assert!(transcript.contains("first line"));
    assert!(transcript.contains("second line"));
    assert!(transcript.contains("third line"));

    let lines = emitted.lock().expect("emit lock poisoned").clone();
    assert!(lines.iter().any(|line| line == "first line"));
    assert!(lines.iter().any(|line| line == "second line"));
    assert!(lines.iter().any(|line| line.contains("third line")));
}

#[tokio::test]
async fn transcribe_prefers_generated_txt_output_when_available() {
    let temp = tempdir().expect("failed to create temp dir");
    let script_path = temp.path().join("whisper-cli");
    let models_dir = temp.path().join("models");
    let input_wav = temp.path().join("audio.wav");

    std::fs::create_dir_all(&models_dir).expect("failed to create models dir");
    std::fs::write(models_dir.join("ggml-base.bin"), b"fake model")
        .expect("failed to create model");
    std::fs::write(&input_wav, b"RIFF....WAVE").expect("failed to create input wav");

    write_executable_script(
        &script_path,
        r#"#!/bin/sh
out=""
while [ $# -gt 0 ]; do
  if [ "$1" = "-of" ]; then
    shift
    out="$1"
  fi
  shift
done
echo "[00:00:00.000 --> 00:00:01.000] partial stdout line"
if [ -n "$out" ]; then
  printf "final line from txt\nanother final line\n" > "${out}.txt"
fi
exit 0
"#,
    );

    let engine = WhisperCppEngine::new(
        script_path.to_string_lossy().to_string(),
        models_dir.to_string_lossy().to_string(),
    );

    let transcript = engine
        .transcribe(
            &input_wav,
            "ggml-base.bin",
            "en",
            Arc::new(|_line: String| {}),
        )
        .await
        .expect("transcription should succeed");

    assert!(transcript.contains("final line from txt"));
    assert!(transcript.contains("another final line"));
    assert!(
        !transcript.contains("partial stdout line"),
        "expected file output to override noisy stream output"
    );
}

#[tokio::test]
async fn transcribe_returns_stderr_on_failure() {
    let temp = tempdir().expect("failed to create temp dir");
    let script_path = temp.path().join("whisper-cli");
    let models_dir = temp.path().join("models");
    let input_wav = temp.path().join("audio.wav");

    std::fs::create_dir_all(&models_dir).expect("failed to create models dir");
    std::fs::write(models_dir.join("ggml-base.bin"), b"fake model")
        .expect("failed to create model");
    std::fs::write(&input_wav, b"RIFF....WAVE").expect("failed to create input wav");

    write_executable_script(
        &script_path,
        r#"#!/bin/sh
echo "fatal: runtime crash" 1>&2
exit 2
"#,
    );

    let engine = WhisperCppEngine::new(
        script_path.to_string_lossy().to_string(),
        models_dir.to_string_lossy().to_string(),
    );

    let error = engine
        .transcribe(
            &input_wav,
            "ggml-base.bin",
            "en",
            Arc::new(|_line: String| {}),
        )
        .await
        .expect_err("transcription should fail");

    match error {
        ApplicationError::SpeechToText(message) => {
            assert!(
                message.contains("fatal: runtime crash"),
                "expected stderr details in error, got: {message}"
            );
        }
        other => panic!("unexpected error variant: {other}"),
    }
}

#[tokio::test]
async fn transcribe_keeps_repeated_lines_from_stream() {
    let temp = tempdir().expect("failed to create temp dir");
    let script_path = temp.path().join("whisper-cli");
    let models_dir = temp.path().join("models");
    let input_wav = temp.path().join("audio.wav");

    std::fs::create_dir_all(&models_dir).expect("failed to create models dir");
    std::fs::write(models_dir.join("ggml-base.bin"), b"fake model")
        .expect("failed to create model");
    std::fs::write(&input_wav, b"RIFF....WAVE").expect("failed to create input wav");

    write_executable_script(
        &script_path,
        r#"#!/bin/sh
echo "[00:00:00.000 --> 00:00:01.000] repeated line"
echo "[00:00:01.000 --> 00:00:02.000] repeated line"
echo "[00:00:02.000 --> 00:00:03.000] final line"
exit 0
"#,
    );

    let engine = WhisperCppEngine::new(
        script_path.to_string_lossy().to_string(),
        models_dir.to_string_lossy().to_string(),
    );

    let emitted: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let emitted_clone = emitted.clone();

    let transcript = engine
        .transcribe(
            &input_wav,
            "ggml-base.bin",
            "en",
            Arc::new(move |line: String| {
                emitted_clone.lock().expect("emit lock poisoned").push(line);
            }),
        )
        .await
        .expect("transcription should succeed");

    let transcript_lines: Vec<&str> = transcript.lines().collect();
    assert_eq!(transcript_lines.len(), 3, "expected all streamed lines");
    assert_eq!(transcript_lines[0], "repeated line");
    assert_eq!(transcript_lines[1], "repeated line");
    assert_eq!(transcript_lines[2], "final line");

    let lines = emitted.lock().expect("emit lock poisoned").clone();
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.as_str() == "repeated line")
            .count(),
        2
    );
}
