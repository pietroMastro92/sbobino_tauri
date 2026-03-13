# Sbobino Tauri

Sbobino Tauri is a modern desktop transcription workspace built with **Tauri v2, Rust, React, and TypeScript**.

It started as a full rewrite of the original Python-based Sbobino application and evolved into a cleaner, faster, and more maintainable desktop product focused on one goal: turning long audio into structured, useful knowledge with a polished native-style experience.

## Why This Project Exists

Most transcription tools stop at raw text.

Sbobino Tauri is designed to go further:

- transcribe audio locally
- refine and optimize transcripts
- generate summaries and FAQs
- chat with the transcript
- export content in multiple professional formats
- manage recordings, trims, history, and speaker-aware segments from a single desktop app

The project combines **local-first transcription workflows** with **AI-assisted post-processing**, while keeping the architecture robust enough for long-term product development.

## Highlights

- **Native desktop experience**
  Built with Tauri for a lightweight desktop shell and a polished, app-like interface.

- **Fast local transcription**
  Uses Whisper-based runtime adapters with strong Rust orchestration and progress tracking.

- **AI-assisted workflow**
  Improve transcript quality, generate summaries, FAQs, and ask questions directly about the conversation.

- **Trim and retranscribe**
  Cut a specific part of an audio recording and generate a focused transcript from that region.

- **Segment-aware transcript view**
  Work with timestamped segments, optional speaker labels, and grouped transcript structures.

- **Speaker diarization support**
  Offline pyannote-based speaker assignment can enrich timeline metadata when the local assets are installed.

- **Professional export system**
  Export transcript content, subtitles, or segments in formats such as `txt`, `docx`, `html`, `pdf`, `md`, `csv`, `json`, `srt`, and `vtt`, depending on the selected export mode.

- **Summary and AI chat controls**
  Language, timestamps, speakers, sections, bullet points, action items, and prompt behavior are wired into the actual final result.

- **History, queue, and artifact management**
  Track active jobs, browse previous transcripts, rename artifacts, restore deleted items, and continue working without losing context.

## What The App Can Do

### Transcription

- Start transcriptions from local audio files
- See live progress while transcription is running
- Cancel active jobs
- Reopen previous transcripts from history
- Manage a queue of in-progress and completed jobs

### Transcript Editing

- Rename a transcription
- Improve transcript wording with AI
- Switch between original and optimized transcript when an optimized version exists
- Trim audio and create focused child transcripts

### AI Features

- Generate structured summaries
- Control summary language and formatting behavior
- Generate FAQs from transcript content
- Use AI chat against transcript context
- Keep the response language aligned with the language of the user prompt

### Segments and Speakers

- Browse transcript segments with timestamps
- Assign speakers to timeline segments
- Propagate speaker labels to adjacent unlabeled segments
- Group unlabeled segments for easier reading

### Export

- Export complete transcript documents
- Export subtitle files
- Export segment-oriented documents
- Include transcript-derived assets like summaries and FAQs in the final exported document when available
- Generate more informative exported document titles based on the transcription title and selected language

## Architecture

The project is organized as a Rust workspace with a clean layered structure:

- `sbobino_desktop/crates/domain`
  Core business entities and rules

- `sbobino_desktop/crates/application`
  Use cases, orchestration, and ports

- `sbobino_desktop/crates/infrastructure`
  Adapters for transcription engines, AI providers, persistence, and runtime integration

- `sbobino_desktop/apps/desktop`
  React + TypeScript frontend

- `sbobino_desktop/apps/desktop/src-tauri`
  Tauri command layer and desktop runtime composition

This separation keeps domain logic independent from UI and external services, making the codebase easier to test, evolve, and maintain.

## Repository Layout

```text
.
├── README.md
├── sbobino_desktop/
│   ├── apps/
│   │   └── desktop/
│   ├── crates/
│   │   ├── application/
│   │   ├── domain/
│   │   └── infrastructure/
│   ├── docs/
│   └── scripts/
```

## Getting Started

### Prerequisites

- Rust stable
- Node.js 20+
- npm
- macOS recommended for the current desktop workflow

### Run The App

```bash
cd sbobino_desktop/apps/desktop
npm ci
npm run tauri:dev
```

### Runtime Setup

From the `sbobino_desktop` workspace root:

```bash
./scripts/setup_runtime.sh
```

This prepares the local transcription runtime for a first run.

Optional:

- install speaker diarization assets from `Settings > Local Models > Speaker Diarization`
- configure AI services in `Settings > AI Services`

## Development Commands

### Frontend

```bash
cd sbobino_desktop/apps/desktop
npm test -- --run
npm run build
```

### Rust

```bash
cd sbobino_desktop
cargo test -p sbobino-application --test transcription_service_tests
cargo check -p sbobino-desktop
```

## Documentation

Useful technical references live inside `sbobino_desktop/docs`:

- [Architecture](./sbobino_desktop/docs/architecture.md)
- [Release and migration notes](./sbobino_desktop/docs/release-and-migration.md)
- [Feature migration matrix](./sbobino_desktop/docs/feature-migration-matrix.md)

The more technical workspace README is available here:

- [Workspace README](./sbobino_desktop/README.md)

## Project Direction

Sbobino Tauri is not just a straight port from an older application.

It is a product-focused rewrite aimed at:

- better UX
- stronger desktop performance
- cleaner architecture
- local-first control over transcription workflows
- richer AI-assisted output on top of raw transcripts

## Status

The project already includes a substantial end-to-end desktop workflow covering transcription, trimming, export, AI summaries, AI chat, and speaker-aware segment handling.

It is actively evolving as a serious desktop product rather than a prototype migration.
