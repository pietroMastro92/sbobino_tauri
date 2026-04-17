# Epic: Device-aware Whisper Optimization

Suggested labels: `epic`, `product`, `transcription-quality`, `performance`

## Problem

The current transcription stack exposes model and runtime controls, but it still depends too much on manual tuning. Users should not need to understand compute units, worker counts, or chunking tradeoffs to get strong transcription quality on their specific Mac.

## Goal

Optimize whisper.cpp and WhisperKit execution for the real device profile so Sbobino can deliver better quality-per-latency out of the box.

## User value

- Better transcripts without manual engine tweaking
- More predictable behavior across different Apple Silicon devices
- Clear choice between speed, balance, and maximum quality

## Proposal

Add device-aware quality profiles that:

- tune compute units, concurrent workers, chunking, timestamps, and VAD defaults
- expose three user-facing modes: `Maximum quality`, `Balanced`, `Maximum speed`
- run an optional local benchmark to recommend the best profile for the active device
- persist the recommended profile per engine and model family

## V1 scope

- macOS device-aware tuning for whisper.cpp and WhisperKit
- local benchmark only
- non-blocking recommendation flow outside critical startup
- no cloud profiling or server-side optimization

## Acceptance criteria

- Users can choose or accept a recommended transcription quality profile.
- Sbobino can persist a device-specific recommendation without blocking startup.
- The recommendation can differ by engine and model.
- Users can reset back to defaults at any time.

## Test scenarios

- The same recording on two different Macs can receive different recommended runtime profiles.
- Benchmarking does not delay first interactive render.
- Switching between quality profiles updates runtime settings predictably.

## Out of scope

- Cross-device sync of performance profiles
- Automatic background benchmarking on every launch
- Remote telemetry-driven model tuning
