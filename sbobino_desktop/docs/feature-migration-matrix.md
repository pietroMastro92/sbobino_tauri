# Feature Migration Matrix (Python -> Rust/Tauri)

This matrix maps every functional area from `python_source` into the new layered architecture.
Use it as the execution checklist for migration and parity sign-off.

## Feature Inventory and Ownership

| Legacy feature area | Python source | New owner layer(s) | Rust target modules |
| --- | --- | --- | --- |
| File transcription pipeline (convert + whisper-cli + staged progress) | `model/transcriptor_model.py`, `controller/transcriptor_controller.py` | Application + Infrastructure + Tauri | `application::TranscriptionService`, `infrastructure::adapters::{ffmpeg, whisper_cpp}`, `src-tauri::commands::transcription` |
| AI optimization + summary/FAQ (Gemini) | `model/transcriptor_model.py` | Application + Infrastructure | `application::TranscriptionService`, `infrastructure::adapters::gemini` |
| Artifact persistence (save/load/list/rename/delete/manual updates) | `model/storage.py` | Domain + Application + Infrastructure | `domain::TranscriptArtifact`, `application::ArtifactRepository`, `infrastructure::repositories::sqlite_artifact_repository` |
| Settings/config lifecycle | `model/config_manager.py` | Domain + Application + Infrastructure | `domain::AppSettings`, `application::SettingsService`, `infrastructure::repositories::fs_settings_repository` |
| Realtime transcription sessions (start/pause/resume/stop, save, resume) | `model/realtime_model.py`, `controller/realtime_controller.py` | Domain + Application + Infrastructure + Tauri | `domain::live_session*` (to add), `application::RealtimeService` (to add), `infrastructure::adapters::whisper_stream` (to add), `src-tauri::commands::realtime` (to add) |
| Export output (TXT, DOCX, PDF) | `controller/transcriptor_controller.py` | Application + Infrastructure | `application::ExportService` (to add), `infrastructure::adapters::{txt_export, docx_export, pdf_export}` (to add) |
| Setup wizard and model bootstrap/downloader | `view/setup_window.py`, `model/transcriptor_model.py`, `model/model_assets.py` | Application + Infrastructure + Frontend | `application::ModelProvisioningService` (to add), `infrastructure::adapters::{asset_index, downloader, archive_extractor}` (to add) |
| i18n language management | `model/i18n_manager.py`, `model/translations/*.json` | Frontend + Infrastructure | UI translation catalog in frontend, optional Rust-backed locale persistence |
| Auto-update checks + installer flow | `model/updater.py`, `model/version.py` | Tauri + CI/CD + Frontend | Tauri updater plugin, release pipeline, update UX in React |
| Path/resource resolution and user-data directories | `path_utils.py` | Infrastructure | `tauri::path` usage + typed path services in infrastructure |
| Network availability checks | `model/network_utils.py` | Infrastructure | connectivity helper in adapter layer |
| Licensing (currently disabled in app startup) | `model/license_manager.py`, `view/license_prompt.py` | Deferred capability | separate `license` bounded context (later) |

## Non-Portable-by-Design (Do Not Clone 1:1)

| Legacy pattern to avoid | Why it should not be ported as-is | Replacement |
| --- | --- | --- |
| UI-driven business orchestration in controllers | Hard to test, race-prone, poor ownership boundaries | Application services + domain events |
| Mutable JSON files per artifact as primary storage | difficult querying/history/scale | SQLite repository + typed domain records |
| `shell=True` subprocess command strings | quoting/security/portability risk | structured `tokio::process::Command` args |
| Tk-specific threading and UI refresh loops | framework-bound and brittle | async services + event-stream updates |
| PyInstaller path branching (`_MEIPASS`, rename hacks) | packaging-specific leakage into domain behavior | Tauri resource and app-data path APIs |
| macOS updater utilities in app code (`hdiutil` flow) | platform logic coupled to product behavior | Tauri updater plugin + CI-managed signed assets |

## Migration Waves and Acceptance Gates

### Wave 1 (Done)
- File transcription vertical slice (no realtime, no export).
- Settings + artifact persistence baseline.
- Progress event channel to frontend.
- Gates:
  - application orchestration tests pass
  - repository contract tests pass
  - frontend build passes

### Wave 2
- Storage parity extensions: rename/delete/manual field edits.
- Artifact list filters (file mode vs realtime mode).
- Gates:
  - parity tests mirroring `test_transcription_storage.py`
  - manual edit persistence tests mirroring `test_transcriptor_manual_edit.py`

### Wave 3
- Realtime transcription bounded context.
- start/pause/resume/stop/session resume semantics.
- Gates:
  - parity tests mirroring `test_realtime.py` and `test_realtime_controller.py`
  - no UI freezes under long sessions

### Wave 4
- Export pipeline (TXT/DOCX/PDF) as independent use case.
- Gates:
  - parity tests mirroring `test_transcriptor_controller_export.py`
  - deterministic artifact snapshots for each format

### Wave 5
- Setup/provisioning workflow (required models, encoder archives, connectivity feedback).
- Gates:
  - parity tests mirroring setup/config robustness suites
  - resumable downloads + checksum validation

### Wave 6
- Updater UX + production key material enablement.
- optional licensing context revival (only if business requires it).
- Gates:
  - signed releases on all targets
  - staged rollout and rollback verification

## Parity Test Mapping

- `test_audio_pipeline.py` -> `application::transcription_service_tests` + adapter integration tests.
- `test_transcription_storage.py` -> `infrastructure::sqlite_artifact_repository_tests`.
- `test_config_manager_robustness.py` + `test_config_i18n*.py` -> settings + locale persistence tests.
- `test_realtime*.py` -> future realtime service tests.
- `test_transcriptor_controller_export.py` -> future export service tests.
- `test_updater.py` -> release/updater integration checks in CI.

## Delivery Rule

For every migrated feature, parity is considered complete only when:
1. Domain/application contracts are tested.
2. Adapter integration is tested against realistic fixtures.
3. Tauri command boundary returns stable DTOs/errors.
4. Frontend adds only presentation behavior and no business logic.
