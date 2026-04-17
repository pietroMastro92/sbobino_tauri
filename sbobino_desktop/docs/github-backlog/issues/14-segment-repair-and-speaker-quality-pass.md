# Epic: Segment Repair and Speaker Quality Pass

Suggested labels: `epic`, `product`, `transcription-quality`, `speaker-diarization`, `transcript-structure`

## Problem

Transcript quality is not only about words. Bad segment boundaries, repeated chunks, unstable timestamps, and suspicious speaker switches make transcripts harder to read and export even when the underlying text is mostly correct.

## Goal

Improve transcript structure after decoding so the final artifact is cleaner, more stable, and easier to review.

## User value

- More readable transcripts
- Better export quality
- Fewer manual fixes to segment boundaries and speaker turns

## Proposal

Add a quality pass that:

- merges or splits segments when the boundaries are clearly unnatural
- repairs suspicious timestamps and broken segmentation
- flags likely bad speaker changes for review
- reduces repeated lines and duplicated chunk artifacts in long transcripts

## V1 scope

- structural cleanup after transcription
- speaker-quality heuristics compatible with local diarization
- artifact metadata for repair status
- no destructive rewriting of user-confirmed edits

## Acceptance criteria

- Segment repair improves readability without hiding raw transcript provenance.
- Speaker-quality warnings are visible when diarization looks suspicious.
- Repetition cleanup reduces duplicated transcript chunks in long files.
- Repair status is persisted in artifact metadata.

## Test scenarios

- A transcript with repeated chunk tails is repaired into cleaner segments.
- A diarized transcript with suspicious speaker flips is flagged for review.
- Segment repair preserves manually edited transcript content once confirmed by the user.

## Out of scope

- Full diarization retraining
- Perfect speaker attribution guarantee
- Multi-artifact structural merge logic
