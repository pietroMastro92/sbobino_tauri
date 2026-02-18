use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use sbobino_application::{ApplicationError, SettingsRepository, SettingsService};
use sbobino_domain::{AppSettings, LanguageCode, SpeechModel};

#[derive(Default)]
struct InMemorySettingsRepository {
    settings: Mutex<AppSettings>,
    save_calls: Mutex<usize>,
}

impl InMemorySettingsRepository {
    fn new(settings: AppSettings) -> Self {
        Self {
            settings: Mutex::new(settings),
            save_calls: Mutex::new(0),
        }
    }
}

#[async_trait]
impl SettingsRepository for InMemorySettingsRepository {
    async fn load(&self) -> Result<AppSettings, ApplicationError> {
        Ok(self
            .settings
            .lock()
            .expect("settings lock poisoned")
            .clone())
    }

    async fn save(&self, settings: &AppSettings) -> Result<(), ApplicationError> {
        *self.settings.lock().expect("settings lock poisoned") = settings.clone();
        let mut save_calls = self.save_calls.lock().expect("save_calls lock poisoned");
        *save_calls += 1;
        Ok(())
    }
}

#[tokio::test]
async fn get_returns_current_settings_from_repository() {
    let repo = Arc::new(InMemorySettingsRepository::new(AppSettings {
        language: LanguageCode::It,
        model: SpeechModel::Small,
        ai_post_processing: true,
        ..AppSettings::default()
    }));

    let service = SettingsService::new(repo);
    let loaded = service.get().await.expect("get settings should succeed");

    assert_eq!(loaded.language, LanguageCode::It);
    assert_eq!(loaded.model, SpeechModel::Small);
    assert!(loaded.ai_post_processing);
}

#[tokio::test]
async fn update_persists_and_returns_saved_settings() {
    let repo = Arc::new(InMemorySettingsRepository::new(AppSettings::default()));
    let service = SettingsService::new(repo.clone());

    let updated = service
        .update(AppSettings {
            language: LanguageCode::En,
            model: SpeechModel::Medium,
            ai_post_processing: true,
            gemini_model: "gemini-2.5-pro".to_string(),
            ..AppSettings::default()
        })
        .await
        .expect("update should succeed");

    assert_eq!(updated.language, LanguageCode::En);
    assert_eq!(updated.model, SpeechModel::Medium);
    assert!(updated.ai_post_processing);
    assert_eq!(updated.gemini_model, "gemini-2.5-pro");
    assert_eq!(
        *repo.save_calls.lock().expect("save_calls lock poisoned"),
        1
    );
}
