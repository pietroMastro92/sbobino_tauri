import type { JobProgress } from "../types";

export const QUEUED_TRANSCRIPTION_JOB_PREFIX = "queued-start:";

export function buildQueuedTranscriptionJobId(sequence: number): string {
  return `${QUEUED_TRANSCRIPTION_JOB_PREFIX}${sequence}`;
}

export function isQueuedTranscriptionJobId(jobId: string): boolean {
  return jobId.startsWith(QUEUED_TRANSCRIPTION_JOB_PREFIX);
}

export function buildQueuedTranscriptionJob(jobId: string, message: string): JobProgress {
  return {
    job_id: jobId,
    stage: "queued",
    message,
    percentage: 0,
    current_seconds: 0,
    total_seconds: null,
  };
}

export function replaceQueuedTranscriptionJob(
  items: JobProgress[],
  queuedJobId: string,
  startedJob: JobProgress,
): JobProgress[] {
  return items.map((item) => (item.job_id === queuedJobId ? startedJob : item));
}

export function upsertQueueItem(
  items: JobProgress[],
  incoming: JobProgress,
): JobProgress[] {
  const existing = items.find((entry) => entry.job_id === incoming.job_id);
  if (!existing) {
    return [...items, incoming];
  }
  return items.map((entry) =>
    entry.job_id === incoming.job_id ? incoming : entry,
  );
}

export function shouldQueueTranscriptionStart({
  activeJobId,
  isStarting,
  startInFlight,
}: {
  activeJobId: string | null;
  isStarting: boolean;
  startInFlight: boolean;
}): boolean {
  return Boolean(activeJobId || isStarting || startInFlight);
}

export function shouldFocusStartedTranscription({
  queuedPromotion,
  preserveCurrentArtifact,
}: {
  queuedPromotion: boolean;
  preserveCurrentArtifact: boolean;
}): boolean {
  return !queuedPromotion && !preserveCurrentArtifact;
}
