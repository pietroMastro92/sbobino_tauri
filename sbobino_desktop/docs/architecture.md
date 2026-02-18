# Sbobino Rewrite Architecture

## Dependency Rules

- `domain` has zero dependency on Tauri, IO, network, and process execution.
- `application` depends on `domain` and declares ports.
- `infrastructure` depends on `application` and `domain` and implements ports.
- `apps/desktop/src-tauri` composes services and exposes command handlers.
- `apps/desktop` consumes only typed Tauri API wrappers and event streams.

## Layer Responsibilities

### Domain (`crates/domain`)
- Business entities: `TranscriptionJob`, `TranscriptArtifact`, `AppSettings`
- Domain enums: `JobStage`, `JobStatus`, `SpeechModel`, `LanguageCode`
- Validation constraints

### Application (`crates/application`)
- Use cases: `TranscriptionService`, `SettingsService`
- Ports: `AudioTranscoder`, `SpeechToTextEngine`, `TranscriptEnhancer`, repositories
- Orchestration and lifecycle progression

### Infrastructure (`crates/infrastructure`)
- Process adapters:
  - `FfmpegAdapter`
  - `WhisperCppEngine` (CLI first, embedded `whisper-rs` fallback)
- API adapters:
  - `GeminiEnhancer`
  - `NoopEnhancer`
- Persistence adapters:
  - `SqliteArtifactRepository`
  - `FsSettingsRepository`
- Runtime composition:
  - `RuntimeTranscriptionFactory` builds a fresh `TranscriptionService` from current settings for each new job.
  - This ensures adapter reconfiguration (e.g. Gemini key/model, binary paths) applies without app restart.

### Tauri Command Layer (`apps/desktop/src-tauri`)
- Commands:
  - `start_transcription`
  - `cancel_transcription`
  - `list_recent_artifacts`
  - `get_artifact`
  - `update_artifact`
  - `get_settings`
  - `update_settings`
- Event bus topics:
  - `transcription://progress`
  - `transcription://completed`
  - `transcription://failed`

### Frontend Presentation (`apps/desktop`)
- React + TypeScript
- Zustand state for local app state
- Typed Tauri service wrappers in `src/lib/tauri.ts`
- No domain logic in React components

## Why This Is Better Than Python MVC

- Business workflow moves from GUI callbacks to testable Rust use-cases.
- Process execution is adapter-owned, not spread across UI/controller code.
- Persistence is strongly typed and centralized in repository adapters.
- Frontend communicates via stable commands/events, mirroring native desktop app boundaries.
- Clear module ownership enables team parallelism and lower regression risk.
