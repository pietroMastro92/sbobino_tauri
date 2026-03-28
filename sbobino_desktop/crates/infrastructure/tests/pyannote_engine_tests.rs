#![cfg(unix)]

use std::path::Path;

use tempfile::tempdir;

use sbobino_application::SpeakerDiarizationEngine;
use sbobino_infrastructure::adapters::pyannote::PyannoteSpeakerDiarizationEngine;

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
async fn diarize_parses_json_output_from_helper() {
    let temp = tempdir().expect("failed to create temp dir");
    let python_path = temp.path().join("python3");
    let script_path = temp.path().join("pyannote_helper.py");
    let input_wav = temp.path().join("audio.wav");

    std::fs::write(&input_wav, b"RIFF....WAVE").expect("failed to create input wav");
    std::fs::write(&script_path, "print('placeholder')").expect("failed to create helper script");

    write_executable_script(
        &python_path,
        r#"#!/bin/sh
script_path="$1"
audio_path="$3"
model_path="$5"
device="$7"
if [ ! -f "$script_path" ]; then
  echo "missing helper script" 1>&2
  exit 2
fi
if [ ! -f "$audio_path" ]; then
  echo "missing audio input" 1>&2
  exit 3
fi
if [ ! -d "$model_path" ]; then
  echo "missing model path" 1>&2
  exit 4
fi
printf '{"speakers":[{"speaker_id":"speaker_1","speaker_label":"Speaker 1","start_seconds":0.0,"end_seconds":1.4},{"speaker_id":"speaker_2","start_seconds":1.4,"end_seconds":2.9}]}\n'
"#,
    );
    std::fs::create_dir_all(temp.path().join("pyannote-model"))
        .expect("failed to create model dir");

    let engine = PyannoteSpeakerDiarizationEngine::new(
        python_path.to_string_lossy().to_string(),
        None,
        script_path.to_string_lossy().to_string(),
        temp.path()
            .join("pyannote-model")
            .to_string_lossy()
            .to_string(),
        "cpu".to_string(),
        vec![],
    );

    let turns = engine
        .diarize(&input_wav)
        .await
        .expect("pyannote helper should succeed");

    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].speaker_id, "speaker_1");
    assert_eq!(turns[0].speaker_label.as_deref(), Some("Speaker 1"));
    assert_eq!(turns[1].speaker_id, "speaker_2");
}
