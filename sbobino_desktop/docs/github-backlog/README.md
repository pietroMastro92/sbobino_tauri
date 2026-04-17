# GitHub Backlog: Automated Ingestion and Productive Outputs

This folder contains issue-ready product epics for the next Sbobino backlog wave.

The backlog is intentionally shaped around the current desktop architecture:

- persistent settings and repositories
- job queue and artifact history
- automatic post-processing hooks
- local-first privacy boundaries
- non-blocking startup

## Product direction

Shared core:

- ingest new audio automatically from user-controlled sources
- prepare usable outputs before the user opens the app
- keep automation visible, reversible, and privacy-safe
- improve transcription quality on the actual device and evolve with safe local personalization

Initial priority order:

1. Auto Inbox from watched folders
2. Apple Voice Memos / iCloud-first import
3. Background worker with fast startup
4. Post-processing automation rules
5. Workspaces and smart folders
6. Student study outputs
7. Enterprise meeting intelligence
8. Trust, control, and local compliance

## V1 defaults

- Sources: local folders and filesystem-synced cloud folders only
- Cloud scope: no native provider APIs in V1
- Deduplication: stable path + size + modified time, with optional hash fallback
- Runtime model: persistent scanner plus watcher where available, periodic rescan otherwise
- Privacy: no implicit upload of new audio
- UX: one "Automatic Import" dashboard for source status, recent discoveries, and errors

## Sequencing guidance

Wave 1:

- `01-auto-inbox-watched-folders.md`
- `02-apple-voice-memos-icloud-import.md`
- `03-background-worker-fast-startup.md`
- `08-trust-control-local-compliance.md`

Wave 2:

- `04-post-processing-automation-rules.md`
- `05-workspaces-smart-folders.md`

Wave 3:

- `06-student-study-output-pack.md`
- `07-enterprise-meeting-intelligence.md`

## Wave 2: Transcription Quality and Personal Adaptation

This second backlog wave focuses on how Sbobino transcribes, not just what it does after import.

Shared quality goals:

- improve whisper.cpp and WhisperKit behavior on the current device
- route models and decoding strategies more intelligently
- learn from user vocabulary and corrections in a local-first way
- surface uncertainty clearly and make review faster
- keep research on true acoustic personalization separate from shippable product work

Quality wave priority order:

9. Device-aware Whisper Optimization
10. Adaptive Model Routing for Transcription
11. Personal Vocabulary and Correction Memory
12. Hard Audio Recovery Mode
13. Confidence, Uncertainty, and Review-Aware Transcript UI
14. Segment Repair and Speaker Quality Pass
15. Personalized Acoustic Adaptation Research Track

Quality wave delivery split:

- Shippable product epics: `09` to `14`
- Research-only epic: `15`

## Shared test scenarios

- A new Voice Memos recording synced to the Mac is queued once and is already available when the user opens the app.
- A new file appears in an iCloud Drive or Dropbox synced folder while the app is closed and is ingested correctly on the next startup.
- A file rename or move does not create a duplicate transcript when the audio was already processed.
- A cloud placeholder that is not yet downloaded locally produces a readable error and a safe retry path.
- Automatic import remains responsive with many watched sources and does not block bootstrap.
- Restrictive privacy settings prevent remote AI automation from running implicitly.

## Issue drafts

- [01 Auto Inbox from watched folders](./issues/01-auto-inbox-watched-folders.md)
- [02 Apple Voice Memos / iCloud-first import](./issues/02-apple-voice-memos-icloud-import.md)
- [03 Background worker and fast startup](./issues/03-background-worker-fast-startup.md)
- [04 Post-processing automation rules](./issues/04-post-processing-automation-rules.md)
- [05 Workspaces and smart folders](./issues/05-workspaces-smart-folders.md)
- [06 Student study output pack](./issues/06-student-study-output-pack.md)
- [07 Enterprise meeting intelligence](./issues/07-enterprise-meeting-intelligence.md)
- [08 Trust, control, and local compliance](./issues/08-trust-control-local-compliance.md)
- [09 Device-aware Whisper Optimization](./issues/09-device-aware-whisper-optimization.md)
- [10 Adaptive Model Routing for Transcription](./issues/10-adaptive-model-routing-for-transcription.md)
- [11 Personal Vocabulary and Correction Memory](./issues/11-personal-vocabulary-and-correction-memory.md)
- [12 Hard Audio Recovery Mode](./issues/12-hard-audio-recovery-mode.md)
- [13 Confidence, Uncertainty, and Review-Aware Transcript UI](./issues/13-confidence-uncertainty-and-review-aware-transcript-ui.md)
- [14 Segment Repair and Speaker Quality Pass](./issues/14-segment-repair-and-speaker-quality-pass.md)
- [15 Personalized Acoustic Adaptation Research Track](./issues/15-personalized-acoustic-adaptation-research-track.md)
