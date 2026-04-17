# Epic: Adaptive Model Routing for Transcription

Suggested labels: `epic`, `product`, `transcription-quality`, `performance`

## Problem

One static model choice is rarely ideal for all recordings. Short voice notes, noisy meetings, and long lectures have different runtime and quality needs, yet the user currently has to make those tradeoffs manually.

## Goal

Choose the most appropriate model and decoding strategy automatically based on the characteristics of the input audio.

## User value

- Better default transcription quality without extra setup
- Faster handling of easy audio and stronger fallback on difficult inputs
- Less trial and error when choosing local models

## Proposal

Add adaptive routing that:

- chooses between `base`, `small`, `medium`, and `large_turbo` using audio duration, noise signals, estimated language, and speaker complexity
- distinguishes recording profiles such as `lecture`, `meeting`, `voice memo`, and `interview`
- retries with a stronger model or adjusted decoding strategy when the first pass looks weak
- stores a readable explanation of why a model and strategy were chosen

## V1 scope

- local-only routing logic
- heuristic-based decision making
- readable artifact metadata for routing decisions
- no ML classifier dependency required in V1

## Acceptance criteria

- Sbobino can route a transcription request automatically when adaptive routing is enabled.
- Users can still override the model manually.
- Routing decisions are visible in artifact metadata or quality UI.
- Automatic fallbacks do not violate existing privacy boundaries.

## Test scenarios

- A short clean memo routes to a lighter configuration.
- A longer noisy meeting escalates to a stronger model or fallback pass.
- Manual model selection bypasses automatic routing when the user explicitly chooses it.

## Out of scope

- Remote inference routing
- Cross-user learning from shared telemetry
- Custom per-customer routing policies in V1
