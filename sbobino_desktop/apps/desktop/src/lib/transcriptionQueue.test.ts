import { describe, expect, it } from "vitest";

import {
  buildQueuedTranscriptionJob,
  buildQueuedTranscriptionJobId,
  isQueuedTranscriptionJobId,
  replaceQueuedTranscriptionJob,
  shouldFocusStartedTranscription,
  shouldQueueTranscriptionStart,
  upsertQueueItem,
} from "./transcriptionQueue";

describe("transcriptionQueue helpers", () => {
  it("marks placeholder jobs with a dedicated prefix", () => {
    const jobId = buildQueuedTranscriptionJobId(3);

    expect(jobId).toBe("queued-start:3");
    expect(isQueuedTranscriptionJobId(jobId)).toBe(true);
    expect(isQueuedTranscriptionJobId("real-job-id")).toBe(false);
  });

  it("replaces a queued placeholder once the backend returns a real job id", () => {
    const queuedJob = buildQueuedTranscriptionJob("queued-start:1", "Queued transcription job.");
    const startedJob = {
      ...queuedJob,
      job_id: "real-job-1",
      stage: "preparing_audio" as const,
      message: "Preparing audio",
      percentage: 10,
    };

    const updated = replaceQueuedTranscriptionJob(
      [queuedJob, buildQueuedTranscriptionJob("queued-start:2", "Queued transcription job.")],
      queuedJob.job_id,
      startedJob,
    );

    expect(updated).toEqual([
      startedJob,
      buildQueuedTranscriptionJob("queued-start:2", "Queued transcription job."),
    ]);
  });

  it("appends new jobs in FIFO order and replaces existing jobs in place", () => {
    const first = buildQueuedTranscriptionJob("queued-start:1", "Queued A");
    const second = buildQueuedTranscriptionJob("queued-start:2", "Queued B");
    const updatedFirst = {
      ...first,
      stage: "preparing_audio" as const,
      message: "Preparing A",
      percentage: 12,
    };

    expect(upsertQueueItem(upsertQueueItem([], first), second)).toEqual([
      first,
      second,
    ]);
    expect(upsertQueueItem([first, second], updatedFirst)).toEqual([
      updatedFirst,
      second,
    ]);
  });

  it("queues starts while a backend start request is synchronously in flight", () => {
    expect(
      shouldQueueTranscriptionStart({
        activeJobId: null,
        isStarting: false,
        startInFlight: true,
      }),
    ).toBe(true);
    expect(
      shouldQueueTranscriptionStart({
        activeJobId: "job-1",
        isStarting: false,
        startInFlight: false,
      }),
    ).toBe(true);
    expect(
      shouldQueueTranscriptionStart({
        activeJobId: null,
        isStarting: false,
        startInFlight: false,
      }),
    ).toBe(false);
  });

  it("focuses manual starts but keeps queued promotions in the background", () => {
    expect(
      shouldFocusStartedTranscription({
        queuedPromotion: false,
        preserveCurrentArtifact: false,
      }),
    ).toBe(true);
    expect(
      shouldFocusStartedTranscription({
        queuedPromotion: true,
        preserveCurrentArtifact: false,
      }),
    ).toBe(false);
    expect(
      shouldFocusStartedTranscription({
        queuedPromotion: false,
        preserveCurrentArtifact: true,
      }),
    ).toBe(false);
  });
});
