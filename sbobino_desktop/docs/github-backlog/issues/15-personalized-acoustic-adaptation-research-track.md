# Epic: Personalized Acoustic Adaptation Research Track

Suggested labels: `epic`, `product`, `transcription-quality`, `personalization`, `research`

## Problem

Users naturally want a transcription app that truly improves with their voice, accent, recurring speakers, and recording conditions. That ambition is important, but it should not be confused with near-term product commitments if it depends on model training, licensing, or heavier runtime changes.

## Goal

Create a dedicated research track for true personalized adaptation without overpromising it as a short-term product feature.

## User value

- Clear path toward deeper personalization
- Honest separation between what is already shippable and what still needs feasibility work
- Better long-term strategy for user-owned adaptation

## Proposal

Research a future personalization layer that covers:

- feasibility of fine-tuning, adapters, or LoRA-like approaches compatible with realistic licenses and local constraints
- optional local collection of audio-correction pairs as user-owned adaptation data
- criteria for deciding whether lexical personalization is enough or acoustic adaptation is worth pursuing
- explicit separation between research outcomes and shipped product promises

## V1 scope

- research only
- feasibility, constraints, and architecture exploration
- no product promise of local model training in the current roadmap
- no UI promise beyond possibly exposing a future-facing experimental concept

## Acceptance criteria

- The team can explain whether personalized acoustic adaptation is realistic for Sbobino’s local-first model stack.
- Research outputs define data, licensing, runtime, and UX constraints clearly.
- Any future implementation recommendation is gated behind explicit validation.

## Test scenarios

- Feasibility analysis compares lexical personalization versus acoustic adaptation.
- Research output defines when local-only adaptation is acceptable and when it is too heavy or risky.
- Release-facing product copy remains honest and does not imply this capability already exists.

## Out of scope

- Shipping fine-tuning in the current milestone
- Implicit data collection without user control
- Any mandatory cloud training pipeline
