# Epic: Hard Audio Recovery Mode

Suggested labels: `epic`, `product`, `transcription-quality`, `audio-processing`

## Problem

Some recordings are simply hard: low volume, clipping, background noise, uneven stereo capture, or overlapping speech. These files often need a different recovery path than the standard transcription flow.

## Goal

Introduce a dedicated recovery mode for difficult audio so weak recordings become more transcribable and more reviewable.

## User value

- Better results from messy real-world recordings
- Fewer unusable transcripts from poor capture conditions
- Clearer explanation when the app has to work harder on a file

## Proposal

Add a `Hard audio mode` that:

- analyzes the input for clipping, low volume, background noise, stereo asymmetry, and overlap risk
- applies optional rescue steps such as normalization, light denoise, mono fold, or retry with a stronger model
- records why the file was treated as difficult
- surfaces recovery status in quality metadata and UI

## V1 scope

- local preprocessing and rescue only
- readable quality flags
- automatic activation from heuristics plus manual override
- no promise of full speech enhancement studio quality

## Acceptance criteria

- Difficult audio can be flagged automatically before or during transcription.
- Recovery mode can trigger a safer alternate path without breaking the standard flow.
- Users can see why recovery mode was used.
- Recovery mode does not block startup or unrelated jobs.

## Test scenarios

- A low-volume file triggers recovery mode and yields a more usable transcript.
- A noisy stereo file applies rescue preprocessing and records the reason.
- A clean file does not trigger unnecessary recovery handling.

## Out of scope

- External cloud denoising services
- Real-time hard-audio enhancement in V1
- Professional audio restoration feature parity
