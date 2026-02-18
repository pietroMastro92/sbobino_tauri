use tempfile::tempdir;

use sbobino_application::SettingsRepository;
use sbobino_domain::{LanguageCode, SpeechModel};
use sbobino_infrastructure::repositories::fs_settings_repository::FsSettingsRepository;

#[tokio::test]
async fn load_creates_default_settings_when_file_is_missing() {
    let temp = tempdir().expect("failed to create temp dir");
    let settings_path = temp.path().join("config").join("settings.json");
    let repo = FsSettingsRepository::new(settings_path.clone());

    let settings = repo.load().await.expect("load should create defaults");

    assert!(settings_path.exists(), "settings file should be created");
    assert_eq!(settings.model, SpeechModel::Base);
    assert_eq!(settings.language, LanguageCode::Auto);
    assert!(!settings.ai_post_processing);
}

#[tokio::test]
async fn save_then_load_round_trips_settings() {
    let temp = tempdir().expect("failed to create temp dir");
    let settings_path = temp.path().join("settings.json");
    let repo = FsSettingsRepository::new(settings_path);

    let mut settings = repo.load().await.expect("initial load should succeed");
    settings.model = SpeechModel::LargeTurbo;
    settings.language = LanguageCode::Fr;
    settings.ai_post_processing = true;
    settings.gemini_model = "gemini-2.5-pro".to_string();

    repo.save(&settings).await.expect("save should succeed");
    let loaded = repo.load().await.expect("second load should succeed");

    assert_eq!(loaded.model, SpeechModel::LargeTurbo);
    assert_eq!(loaded.language, LanguageCode::Fr);
    assert!(loaded.ai_post_processing);
    assert_eq!(loaded.gemini_model, "gemini-2.5-pro");
}
