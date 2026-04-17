# Epic: Confidence, Uncertainty, and Review-Aware Transcript UI

Suggested labels: `epic`, `product`, `transcription-quality`, `ux`

## Problem

Users currently see the transcript result, but they do not always know which parts are likely fragile. Without visible uncertainty, review becomes slower and trust in the transcript is harder to calibrate.

## Goal

Expose uncertainty in a useful way and turn it into a fast human review workflow.

## User value

- Faster review of problematic transcript spans
- Better trust calibration on long recordings
- Easier correction of names, uncertain words, and weak speaker attribution

## Proposal

Add a review-aware transcript quality UI with:

- confidence indicators per segment or word when the engine can provide them
- visible highlighting for low-confidence spans, uncertain names, and weak speaker assignments
- a focused review queue of “check these points”
- shortcuts to confirm, correct, and save corrections into local memory

## V1 scope

- segment-first quality UI, with word-level detail where available
- explicit review flow integrated into transcript inspection
- local correction save path only
- no collaboration workflow required in V1

## Acceptance criteria

- Users can identify weak parts of a transcript quickly.
- Confirming or correcting an uncertain span is faster than manually rewriting large sections.
- Review actions can feed the local correction-memory system.
- Quality indicators degrade gracefully when an engine does not expose detailed confidence.

## Test scenarios

- A low-confidence transcript surfaces a focused review queue.
- Correcting an uncertain proper noun can be saved into local correction memory.
- An engine without word confidence still shows segment-level review guidance.

## Out of scope

- Multi-reviewer workflows
- External annotation tools
- Automated silent rewriting of the full transcript without review visibility
