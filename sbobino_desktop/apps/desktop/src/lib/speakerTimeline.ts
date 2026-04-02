import { normalizeSpeakerColorKey } from "./speakerColors";

export type TimelineV2Document = {
  version: number;
  segments: Array<Record<string, unknown>>;
};

export type RenameSpeakerResult =
  | {
      ok: true;
      timeline: TimelineV2Document;
      previousSpeakerLabel: string;
      previousSpeakerId: string | null;
      nextSpeakerId: string;
      renamedCount: number;
    }
  | {
      ok: false;
      reason: "missing_timeline" | "segment_out_of_range" | "speaker_missing" | "speaker_name_empty";
    };

export type MergeSpeakerResult =
  | {
      ok: true;
      timeline: TimelineV2Document;
      sourceSpeakerId: string;
      sourceSpeakerLabel: string;
      targetSpeakerId: string;
      targetSpeakerLabel: string;
      mergedCount: number;
    }
  | {
      ok: false;
      reason: "missing_timeline" | "speaker_missing" | "target_missing" | "same_speaker";
    };

export type RemoveSpeakerResult =
  | {
      ok: true;
      timeline: TimelineV2Document;
      removedSpeakerId: string;
      removedSpeakerLabel: string;
      removedCount: number;
    }
  | {
      ok: false;
      reason: "missing_timeline" | "speaker_missing";
    };

function normalizeText(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function parseTimelineV2Document(
  timelineV2Json: string | null | undefined,
): TimelineV2Document | null {
  const raw = timelineV2Json?.trim();
  if (!raw) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw) as { version?: unknown; segments?: unknown };
    if (!Array.isArray(parsed?.segments)) {
      return null;
    }

    return {
      version:
        typeof parsed.version === "number" && Number.isFinite(parsed.version)
          ? parsed.version
          : 2,
      segments: parsed.segments
        .map((segment) => (segment && typeof segment === "object" ? { ...(segment as Record<string, unknown>) } : null))
        .filter((segment): segment is Record<string, unknown> => Boolean(segment)),
    };
  } catch {
    return null;
  }
}

function readSpeakerLabel(segment: Record<string, unknown>): string | null {
  return normalizeText(segment.speaker_label) ?? normalizeText(segment.speaker_id);
}

function readSpeakerId(segment: Record<string, unknown>): string | null {
  return normalizeText(segment.speaker_id);
}

function matchesSpeaker(
  segment: Record<string, unknown>,
  speakerId: string | null,
  speakerLabel: string | null,
): boolean {
  const segmentSpeakerId = readSpeakerId(segment);
  const segmentSpeakerLabel = readSpeakerLabel(segment);

  if (speakerId) {
    return segmentSpeakerId === speakerId
      || (!segmentSpeakerId && speakerLabel !== null && segmentSpeakerLabel === speakerLabel);
  }

  return speakerLabel !== null && segmentSpeakerLabel === speakerLabel;
}

export function renameSpeakerInTimeline(
  timelineV2Json: string | null | undefined,
  sourceIndex: number,
  nextSpeakerLabel: string,
): RenameSpeakerResult {
  const parsedTimeline = parseTimelineV2Document(timelineV2Json);
  if (!parsedTimeline) {
    return { ok: false, reason: "missing_timeline" };
  }

  if (sourceIndex < 0 || sourceIndex >= parsedTimeline.segments.length) {
    return { ok: false, reason: "segment_out_of_range" };
  }

  const normalizedNextSpeakerLabel = nextSpeakerLabel.trim();
  if (!normalizedNextSpeakerLabel) {
    return { ok: false, reason: "speaker_name_empty" };
  }

  const targetSegment = parsedTimeline.segments[sourceIndex];
  const currentSpeakerLabel = readSpeakerLabel(targetSegment);
  const currentSpeakerId = readSpeakerId(targetSegment);
  if (!currentSpeakerLabel && !currentSpeakerId) {
    return { ok: false, reason: "speaker_missing" };
  }

  const nextSpeakerId = normalizeSpeakerColorKey(normalizedNextSpeakerLabel);
  const previousSpeakerId =
    currentSpeakerId ?? normalizeSpeakerColorKey(currentSpeakerLabel);
  let renamedCount = 0;
  const nextSegments = parsedTimeline.segments.map((segment) => {
    const segmentSpeakerId = readSpeakerId(segment);
    const segmentSpeakerLabel = readSpeakerLabel(segment);
    const matchesCurrentSpeaker = currentSpeakerId
      ? segmentSpeakerId === currentSpeakerId
        || (!segmentSpeakerId && segmentSpeakerLabel === currentSpeakerLabel)
      : segmentSpeakerLabel === currentSpeakerLabel;

    if (!matchesCurrentSpeaker) {
      return segment;
    }

    renamedCount += 1;
    return {
      ...segment,
      speaker_label: normalizedNextSpeakerLabel,
      speaker_id: nextSpeakerId,
    };
  });

  return {
    ok: true,
    timeline: {
      ...parsedTimeline,
      segments: nextSegments,
    },
    previousSpeakerLabel: currentSpeakerLabel ?? currentSpeakerId ?? "",
    previousSpeakerId,
    nextSpeakerId,
    renamedCount,
  };
}

export function mergeSpeakerInTimeline(
  timelineV2Json: string | null | undefined,
  sourceSpeakerId: string | null | undefined,
  sourceSpeakerLabel: string | null | undefined,
  targetSpeakerId: string | null | undefined,
  targetSpeakerLabel: string | null | undefined,
): MergeSpeakerResult {
  const parsedTimeline = parseTimelineV2Document(timelineV2Json);
  if (!parsedTimeline) {
    return { ok: false, reason: "missing_timeline" };
  }

  const normalizedSourceSpeakerId = normalizeText(sourceSpeakerId);
  const normalizedSourceSpeakerLabel = normalizeText(sourceSpeakerLabel);
  const normalizedTargetSpeakerLabel = normalizeText(targetSpeakerLabel);
  const normalizedTargetSpeakerId =
    normalizeText(targetSpeakerId)
    ?? (normalizedTargetSpeakerLabel ? normalizeSpeakerColorKey(normalizedTargetSpeakerLabel) : null);

  if (!normalizedSourceSpeakerId && !normalizedSourceSpeakerLabel) {
    return { ok: false, reason: "speaker_missing" };
  }

  if (!normalizedTargetSpeakerId || !normalizedTargetSpeakerLabel) {
    return { ok: false, reason: "target_missing" };
  }

  if (
    (normalizedSourceSpeakerId && normalizedSourceSpeakerId === normalizedTargetSpeakerId)
    || (!normalizedSourceSpeakerId && normalizedSourceSpeakerLabel === normalizedTargetSpeakerLabel)
  ) {
    return { ok: false, reason: "same_speaker" };
  }

  let mergedCount = 0;
  const nextSegments = parsedTimeline.segments.map((segment) => {
    if (!matchesSpeaker(segment, normalizedSourceSpeakerId, normalizedSourceSpeakerLabel)) {
      return segment;
    }

    mergedCount += 1;
    return {
      ...segment,
      speaker_label: normalizedTargetSpeakerLabel,
      speaker_id: normalizedTargetSpeakerId,
    };
  });

  if (mergedCount === 0) {
    return { ok: false, reason: "speaker_missing" };
  }

  return {
    ok: true,
    timeline: {
      ...parsedTimeline,
      segments: nextSegments,
    },
    sourceSpeakerId: normalizedSourceSpeakerId ?? normalizeSpeakerColorKey(normalizedSourceSpeakerLabel),
    sourceSpeakerLabel: normalizedSourceSpeakerLabel ?? normalizedSourceSpeakerId ?? "",
    targetSpeakerId: normalizedTargetSpeakerId,
    targetSpeakerLabel: normalizedTargetSpeakerLabel,
    mergedCount,
  };
}

export function removeSpeakerFromTimeline(
  timelineV2Json: string | null | undefined,
  speakerId: string | null | undefined,
  speakerLabel: string | null | undefined,
): RemoveSpeakerResult {
  const parsedTimeline = parseTimelineV2Document(timelineV2Json);
  if (!parsedTimeline) {
    return { ok: false, reason: "missing_timeline" };
  }

  const normalizedSpeakerId = normalizeText(speakerId);
  const normalizedSpeakerLabel = normalizeText(speakerLabel);
  if (!normalizedSpeakerId && !normalizedSpeakerLabel) {
    return { ok: false, reason: "speaker_missing" };
  }

  let removedCount = 0;
  const nextSegments = parsedTimeline.segments.map((segment) => {
    if (!matchesSpeaker(segment, normalizedSpeakerId, normalizedSpeakerLabel)) {
      return segment;
    }

    removedCount += 1;
    const nextSegment = { ...segment };
    delete nextSegment.speaker_id;
    delete nextSegment.speaker_label;
    return nextSegment;
  });

  if (removedCount === 0) {
    return { ok: false, reason: "speaker_missing" };
  }

  return {
    ok: true,
    timeline: {
      ...parsedTimeline,
      segments: nextSegments,
    },
    removedSpeakerId: normalizedSpeakerId ?? normalizeSpeakerColorKey(normalizedSpeakerLabel),
    removedSpeakerLabel: normalizedSpeakerLabel ?? normalizedSpeakerId ?? "",
    removedCount,
  };
}
