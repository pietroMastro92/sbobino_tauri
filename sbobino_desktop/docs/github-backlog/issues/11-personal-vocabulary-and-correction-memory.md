# Epic: Personal Vocabulary and Correction Memory

Suggested labels: `epic`, `product`, `transcription-quality`, `personalization`

## Problem

Many transcription errors repeat: names, course titles, domain terminology, company names, and specialized vocabulary. Users often correct the same mistakes again and again, but the app does not yet learn from those corrections.

## Goal

Make Sbobino improve over time through local vocabulary memory and correction-aware transcription assistance, without promising full model fine-tuning.

## User value

- Fewer repeated mistakes on names and technical terminology
- Faster cleanup after transcription
- A concrete sense that the app improves with use

## Proposal

Add a local personalization layer that includes:

- personal vocabulary entries for names, companies, courses, and technical terms
- correction memory from repeated user edits
- post-transcription normalization suggestions based on known corrections
- prompt/prefill biasing where supported by the active engine

## V1 scope

- fully local storage
- user-controlled vocabulary and correction memory
- no background model training
- correction reuse limited to safe and explainable text transformations

## Acceptance criteria

- Users can add and manage personal vocabulary entries.
- Repeated corrections can be suggested or applied in future transcripts.
- Applied vocabulary and correction-memory hits are visible in quality metadata.
- Users can disable personalization entirely.

## Test scenarios

- A repeatedly corrected proper noun is suggested correctly in later transcripts.
- Personal vocabulary improves transcription cleanup of a technical lecture.
- Disabling personalization prevents stored correction memory from affecting future transcripts.

## Out of scope

- Hidden automatic training without user control
- Cloud-synced correction memory
- Full acoustic fine-tuning of whisper models
