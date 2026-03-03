import { Pause, Play, Rabbit, Scissors, X, Trash2, Check } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { readAudioFile, writeTrimmedAudio } from "../lib/tauri";

// ── Types ──────────────────────────────────────────────────────────

export type TrimRegion = {
  id: string;
  startTime: number;
  endTime: number;
};

type AudioPlayerProps = {
  inputPath: string | null;
  onMetadataLoaded?: (metadata: { durationSeconds: number }) => void;
  onTrimRegionsChange?: (regions: TrimRegion[]) => void;
  onTrimApplied?: (trimmedPath: string) => void;
};

// ── Helpers ────────────────────────────────────────────────────────

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return "00:00";
  }
  const mm = String(Math.floor(seconds / 60)).padStart(2, "0");
  const ss = String(Math.floor(seconds % 60)).padStart(2, "0");
  return `${mm}:${ss}`;
}

function mimeFromPath(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  switch (extension) {
    case "mp3": return "audio/mpeg";
    case "wav": return "audio/wav";
    case "m4a": return "audio/mp4";
    case "aac": return "audio/aac";
    case "ogg": return "audio/ogg";
    case "opus": return "audio/opus";
    case "flac": return "audio/flac";
    default: return "audio/*";
  }
}

let regionCounter = 0;
function makeRegionId(): string {
  regionCounter += 1;
  return `region-${regionCounter}-${Date.now()}`;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

// ── Waveform: pre-compute peaks (expensive — done once) ────────────

interface CachedPeaks {
  peaks: Float32Array;
  maxPeak: number;
}

interface WaveformColors {
  accent: string;
  muted: string;
  line: string;
}

function computePeaks(audioBuffer: AudioBuffer, targetWidth: number): CachedPeaks {
  const rawData = audioBuffer.getChannelData(0);
  const samplesPerPixel = Math.max(1, Math.floor(rawData.length / targetWidth));
  const peaks = new Float32Array(targetWidth);

  for (let i = 0; i < targetWidth; i++) {
    const start = i * samplesPerPixel;
    const end = Math.min(start + samplesPerPixel, rawData.length);
    let max = 0;
    for (let j = start; j < end; j++) {
      const abs = Math.abs(rawData[j]);
      if (abs > max) max = abs;
    }
    peaks[i] = max;
  }

  let maxPeak = 0.01;
  for (let i = 0; i < peaks.length; i++) {
    if (peaks[i] > maxPeak) maxPeak = peaks[i];
  }

  return { peaks, maxPeak };
}

// ── Waveform: draw (uses pre-computed peaks, no heavy work) ────────

function drawWaveform(
  canvas: HTMLCanvasElement,
  cachedPeaks: CachedPeaks | null,
  regions: TrimRegion[],
  duration: number,
  currentTime: number,
  trimMode: boolean,
  colors: WaveformColors,
): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  const cssWidth = canvas.clientWidth;
  const cssHeight = canvas.clientHeight;

  const targetW = Math.round(cssWidth * dpr);
  const targetH = Math.round(cssHeight * dpr);
  if (canvas.width !== targetW || canvas.height !== targetH) {
    canvas.width = targetW;
    canvas.height = targetH;
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, cssWidth, cssHeight);

  if (!cachedPeaks || duration <= 0) {
    // Deterministic placeholder bars (no Math.random)
    const barCount = Math.floor(cssWidth / 4);
    ctx.fillStyle = colors.line;
    for (let i = 0; i < barCount; i++) {
      const seed = ((i * 7 + 13) % 37) / 37;
      const height = 4 + seed * (cssHeight * 0.4);
      ctx.fillRect(i * 4, (cssHeight - height) / 2, 2, height);
    }
    return;
  }

  const { peaks, maxPeak } = cachedPeaks;
  const peakCount = Math.min(peaks.length, cssWidth);
  const playheadPx = duration > 0 ? (currentTime / duration) * cssWidth : 0;

  // Dimmed background for trim mode
  if (trimMode && regions.length > 0) {
    ctx.fillStyle = "rgba(128, 128, 128, 0.15)";
    ctx.fillRect(0, 0, cssWidth, cssHeight);
    for (const r of regions) {
      ctx.clearRect((r.startTime / duration) * cssWidth, 0, ((r.endTime - r.startTime) / duration) * cssWidth, cssHeight);
    }
  }

  // Pre-compute per-pixel region membership for O(1) lookup
  let inRegion: Uint8Array | null = null;
  if (trimMode && regions.length > 0) {
    inRegion = new Uint8Array(peakCount);
    for (const r of regions) {
      const pxStart = Math.floor((r.startTime / duration) * peakCount);
      const pxEnd = Math.ceil((r.endTime / duration) * peakCount);
      for (let i = pxStart; i < pxEnd && i < peakCount; i++) {
        inRegion[i] = 1;
      }
    }
  }

  // Batch render by color: accent → muted → line (3 fillStyle switches total)
  // Pass 1: accent bars (played + in region)
  ctx.fillStyle = colors.accent;
  for (let i = 0; i < peakCount; i++) {
    if (i > playheadPx) continue;
    if (inRegion && !inRegion[i]) continue;
    const h = Math.max(2, (peaks[i] / maxPeak) * cssHeight * 0.85);
    ctx.fillRect(i, (cssHeight - h) / 2, 1.5, h);
  }

  // Pass 2: muted bars (unplayed + in region)
  ctx.fillStyle = colors.muted;
  for (let i = 0; i < peakCount; i++) {
    if (i <= playheadPx) continue;
    if (inRegion && !inRegion[i]) continue;
    const h = Math.max(2, (peaks[i] / maxPeak) * cssHeight * 0.85);
    ctx.fillRect(i, (cssHeight - h) / 2, 1.5, h);
  }

  // Pass 3: dimmed bars (outside region in trim mode)
  if (inRegion) {
    ctx.fillStyle = colors.line;
    for (let i = 0; i < peakCount; i++) {
      if (inRegion[i]) continue;
      const h = Math.max(2, (peaks[i] / maxPeak) * cssHeight * 0.85);
      ctx.fillRect(i, (cssHeight - h) / 2, 1.5, h);
    }
  }

  // Region highlights & boundary lines
  if (trimMode) {
    for (const r of regions) {
      const sx = (r.startTime / duration) * cssWidth;
      const ex = (r.endTime / duration) * cssWidth;
      ctx.fillStyle = `${colors.accent}14`;
      ctx.fillRect(sx, 0, ex - sx, cssHeight);
      ctx.fillStyle = colors.accent;
      ctx.fillRect(sx, 0, 1.5, cssHeight);
      ctx.fillRect(ex - 1.5, 0, 1.5, cssHeight);
    }
  }

  // Playhead
  if (currentTime > 0 && duration > 0) {
    ctx.fillStyle = colors.accent;
    ctx.fillRect(playheadPx - 0.75, 0, 1.5, cssHeight);
  }
}

// ── Component ──────────────────────────────────────────────────────

export function AudioPlayer({
  inputPath,
  onMetadataLoaded,
  onTrimRegionsChange,
  onTrimApplied,
}: AudioPlayerProps): JSX.Element | null {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const waveformTrackRef = useRef<HTMLDivElement | null>(null);
  const fallbackBlobUrlRef = useRef<string | null>(null);
  const fallbackAttemptedRef = useRef(false);
  const pendingAutoPlayRef = useRef(false);
  const sourceVersionRef = useRef(0);
  const animFrameRef = useRef<number>(0);

  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [playbackRate, setPlaybackRate] = useState(1);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [needsFallback, setNeedsFallback] = useState(false);

  // Trim state
  const [trimMode, setTrimMode] = useState(false);
  const [regions, setRegions] = useState<TrimRegion[]>([]);
  const [dragging, setDragging] = useState<{ regionId: string; handle: "start" | "end" } | null>(null);
  const [creating, setCreating] = useState<{ startTime: number; currentTime: number } | null>(null);

  // Cached waveform peaks — computed ONCE per audio source, not every frame
  const [cachedPeaks, setCachedPeaks] = useState<CachedPeaks | null>(null);
  const [isDecodingWaveform, setIsDecodingWaveform] = useState(false);
  const [isApplyingTrim, setIsApplyingTrim] = useState(false);
  const audioBufferRef = useRef<AudioBuffer | null>(null);

  // Cached CSS colors — read once + updated on theme change
  const colorsRef = useRef<WaveformColors>({
    accent: "#6D94C5",
    muted: "#8a7e6e",
    line: "rgba(180, 165, 140, 0.25)",
  });

  // Haptics
  const hapticsRef = useRef<{ trigger: (preset: string) => void } | null>(null);
  const lastHapticSecondRef = useRef(-1);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const moduleName = "web-haptics";
        const mod = await import(/* @vite-ignore */ moduleName);
        if (!cancelled) {
          const instance = new mod.WebHaptics({ debug: false });
          hapticsRef.current = {
            trigger: (preset: string) => void instance.trigger(preset as "success" | "nudge" | "error"),
          };
        }
      } catch {
        // web-haptics not installed — haptics silently disabled
      }
    })();
    return () => { cancelled = true; };
  }, []);

  const triggerHaptic = useCallback((preset: string) => {
    hapticsRef.current?.trigger(preset);
  }, []);

  const sourcePath = useMemo(() => {
    if (!inputPath || inputPath.trim().length === 0) return null;
    return inputPath;
  }, [inputPath]);

  // Notify parent of region changes
  useEffect(() => {
    onTrimRegionsChange?.(regions);
  }, [regions, onTrimRegionsChange]);

  // ── Cache CSS colors & listen for theme changes ─────────────

  useEffect(() => {
    function updateColors() {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const style = getComputedStyle(canvas);
      colorsRef.current = {
        accent: style.getPropertyValue("--accent").trim() || "#6D94C5",
        muted: style.getPropertyValue("--muted").trim() || "#8a7e6e",
        line: style.getPropertyValue("--line").trim() || "rgba(180, 165, 140, 0.25)",
      };
    }
    updateColors();
    const observer = new MutationObserver(updateColors);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, []);

  // ── Source management ──────────────────────────────────────

  useEffect(() => {
    if (fallbackBlobUrlRef.current) {
      URL.revokeObjectURL(fallbackBlobUrlRef.current);
      fallbackBlobUrlRef.current = null;
    }

    if (!sourcePath) {
      setSrc(null);
      setLoadError(null);
      setIsLoading(false);
      fallbackAttemptedRef.current = false;
      pendingAutoPlayRef.current = false;
      setNeedsFallback(false);
      setCachedPeaks(null);
      setRegions([]);
      setTrimMode(false);
      onMetadataLoaded?.({ durationSeconds: 0 });
      return;
    }

    if (sourcePath.startsWith("http://") || sourcePath.startsWith("https://")) {
      setSrc(sourcePath);
      setLoadError(null);
      setIsLoading(false);
      fallbackAttemptedRef.current = false;
      pendingAutoPlayRef.current = false;
      setNeedsFallback(false);
      return;
    }

    const primarySrc = isTauri() ? convertFileSrc(sourcePath) : sourcePath;
    const sourceVersion = sourceVersionRef.current + 1;
    sourceVersionRef.current = sourceVersion;

    setSrc(primarySrc);
    setLoadError(null);
    setIsLoading(false);
    fallbackAttemptedRef.current = false;
    pendingAutoPlayRef.current = false;
    setNeedsFallback(false);
    setCachedPeaks(null);
    setRegions([]);
    setTrimMode(false);

    return () => {
      if (sourceVersionRef.current === sourceVersion) {
        sourceVersionRef.current += 1;
      }
    };
  }, [sourcePath]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.pause();
    audio.currentTime = 0;
    setIsPlaying(false);
    setCurrentTime(0);
    setDuration(0);
    setLoadError(null);
    pendingAutoPlayRef.current = false;
  }, [src]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.playbackRate = playbackRate;
  }, [playbackRate]);

  // ── LAZY waveform decode — only when trim mode is activated ─

  useEffect(() => {
    if (!trimMode || !src || cachedPeaks) return;

    let cancelled = false;
    setIsDecodingWaveform(true);

    (async () => {
      try {
        const response = await fetch(src);
        if (cancelled) return;
        const arrayBuffer = await response.arrayBuffer();
        if (cancelled) return;
        const audioCtx = new AudioContext();
        const decoded = await audioCtx.decodeAudioData(arrayBuffer);
        if (cancelled) return;
        // Compute peaks ONCE at a reasonable resolution (max 1200px)
        const targetWidth = Math.min(canvasRef.current?.clientWidth || 800, 1200);
        setCachedPeaks(computePeaks(decoded, targetWidth));
        audioBufferRef.current = decoded;
        await audioCtx.close();
      } catch {
        setCachedPeaks(null);
      } finally {
        if (!cancelled) setIsDecodingWaveform(false);
      }
    })();

    return () => { cancelled = true; };
  }, [trimMode, src, cachedPeaks]);

  // ── Canvas redraw — ONLY when state changes, NOT continuous rAF ─

  const redrawCanvas = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas || !trimMode) return;
    drawWaveform(canvas, cachedPeaks, regions, duration, currentTime, trimMode, colorsRef.current);
  }, [cachedPeaks, regions, duration, currentTime, trimMode]);

  useEffect(() => {
    if (!trimMode) return;
    animFrameRef.current = requestAnimationFrame(redrawCanvas);
    return () => cancelAnimationFrame(animFrameRef.current);
  }, [trimMode, redrawCanvas]);

  // ── Fallback audio loading ─────────────────────────────────

  if (!sourcePath) return null;

  async function loadFallbackAudio(autoPlay: boolean): Promise<boolean> {
    const currentSourcePath = sourcePath;
    if (!currentSourcePath) return false;

    if (fallbackBlobUrlRef.current) {
      setSrc(fallbackBlobUrlRef.current);
      setNeedsFallback(false);
      pendingAutoPlayRef.current = autoPlay;
      return true;
    }

    if (fallbackAttemptedRef.current) return false;

    fallbackAttemptedRef.current = true;
    const sourceVersion = sourceVersionRef.current;
    setIsLoading(true);
    setLoadError(null);

    try {
      const bytes = await readAudioFile(currentSourcePath);
      if (sourceVersionRef.current !== sourceVersion) return false;
      const blob = new Blob([new Uint8Array(bytes)], { type: mimeFromPath(currentSourcePath) });
      const objectUrl = URL.createObjectURL(blob);
      if (fallbackBlobUrlRef.current) URL.revokeObjectURL(fallbackBlobUrlRef.current);
      fallbackBlobUrlRef.current = objectUrl;
      pendingAutoPlayRef.current = autoPlay;
      setSrc(objectUrl);
      setNeedsFallback(false);
      setLoadError(null);
      return true;
    } catch (error) {
      if (sourceVersionRef.current !== sourceVersion) return false;
      setLoadError(`Cannot load fallback audio: ${String(error)}`);
      return false;
    } finally {
      if (sourceVersionRef.current === sourceVersion) setIsLoading(false);
    }
  }

  // ── Playback ───────────────────────────────────────────────

  async function togglePlayback(): Promise<void> {
    const audio = audioRef.current;
    if (!audio) return;

    if (audio.paused) {
      if (needsFallback && !fallbackBlobUrlRef.current) {
        const prepared = await loadFallbackAudio(true);
        if (prepared) return;
      }
      try {
        await audio.play();
        setIsPlaying(true);
      } catch (error) {
        setIsPlaying(false);
        setLoadError(`Cannot play audio: ${String(error)}`);
      }
      return;
    }

    audio.pause();
    setIsPlaying(false);
  }

  function onSeek(value: number): void {
    const audio = audioRef.current;
    if (!audio || !Number.isFinite(value)) return;
    audio.currentTime = value;
    setCurrentTime(value);
  }

  function onChangeSpeed(nextRate: number): void {
    const audio = audioRef.current;
    if (audio) audio.playbackRate = nextRate;
    setPlaybackRate(nextRate);
  }

  // ── Waveform: click-drag to create region ──────────────────

  function getTimeFromMouseEvent(e: React.MouseEvent | MouseEvent): number {
    const track = waveformTrackRef.current;
    if (!track || duration <= 0) return 0;
    const rect = track.getBoundingClientRect();
    return clamp((e.clientX - rect.left) / rect.width, 0, 1) * duration;
  }

  function onWaveformMouseDown(event: React.MouseEvent<HTMLDivElement>): void {
    if (!trimMode || duration <= 0) return;
    // Don't start creating if clicking on a handle
    if ((event.target as HTMLElement).closest(".trim-handle")) return;

    const time = getTimeFromMouseEvent(event);
    setCreating({ startTime: time, currentTime: time });

    function onMouseMove(e: MouseEvent) {
      const moveTime = getTimeFromMouseEvent(e);
      setCreating((prev) => prev ? { ...prev, currentTime: moveTime } : null);
    }

    function onMouseUp(e: MouseEvent) {
      const endTime = getTimeFromMouseEvent(e);
      setCreating(null);

      const s = Math.min(time, endTime);
      const en = Math.max(time, endTime);

      // Only create region if dragged at least 0.3 seconds
      if (en - s >= 0.3) {
        const newRegion: TrimRegion = {
          id: makeRegionId(),
          startTime: clamp(s, 0, duration),
          endTime: clamp(en, 0, duration),
        };

        // Check overlaps
        const overlaps = regions.some(
          (r) => newRegion.startTime < r.endTime && newRegion.endTime > r.startTime,
        );

        if (overlaps) {
          triggerHaptic("error");
        } else {
          triggerHaptic("success");
          setRegions((prev) =>
            [...prev, newRegion].sort((a, b) => a.startTime - b.startTime),
          );
        }
      } else {
        // Short click → seek
        onSeek(endTime);
      }

      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    }

    event.preventDefault();
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }

  // ── Trim region management ─────────────────────────────────

  function removeRegion(id: string): void {
    setRegions((prev) => prev.filter((r) => r.id !== id));
  }

  function clearAllRegions(): void {
    setRegions([]);
  }

  // ── Handle dragging (existing region handles) ──────────────

  function onHandleMouseDown(event: React.MouseEvent, regionId: string, handle: "start" | "end"): void {
    event.preventDefault();
    event.stopPropagation();
    setDragging({ regionId, handle });
    lastHapticSecondRef.current = -1;

    function onMouseMove(e: MouseEvent) {
      const time = getTimeFromMouseEvent(e);

      const currentSecond = Math.floor(time);
      if (currentSecond !== lastHapticSecondRef.current) {
        lastHapticSecondRef.current = currentSecond;
        triggerHaptic("nudge");
      }

      setRegions((prev) =>
        prev.map((r) => {
          if (r.id !== regionId) return r;
          if (handle === "start") return { ...r, startTime: clamp(time, 0, r.endTime - 0.5) };
          return { ...r, endTime: clamp(time, r.startTime + 0.5, duration) };
        }),
      );

      onSeek(time);
    }

    function onMouseUp() {
      setDragging(null);
      triggerHaptic("success");
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    }

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }

  // ── Compute creating region for display ─────────────────────
  const creatingRegion = creating ? {
    startTime: Math.min(creating.startTime, creating.currentTime),
    endTime: Math.max(creating.startTime, creating.currentTime),
  } : null;

  // ── Render ─────────────────────────────────────────────────

  return (
    <footer className={`audio-player ${trimMode ? "audio-player--trim" : ""}`}>
      <audio
        ref={audioRef}
        src={src ?? undefined}
        preload="metadata"
        onLoadedMetadata={(event) => {
          const durationSeconds = event.currentTarget.duration || 0;
          setDuration(durationSeconds);
          onMetadataLoaded?.({ durationSeconds });
          setLoadError(null);
          if (pendingAutoPlayRef.current) {
            pendingAutoPlayRef.current = false;
            void event.currentTarget.play().catch(() => setIsPlaying(false));
          }
        }}
        onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime || 0)}
        onEnded={() => setIsPlaying(false)}
        onPause={() => setIsPlaying(false)}
        onPlay={() => setIsPlaying(true)}
        onError={() => {
          const audioPath = sourcePath;
          if (!audioPath) { setIsPlaying(false); return; }
          const fallbackSrc = fallbackBlobUrlRef.current;
          if (fallbackSrc && src !== fallbackSrc) {
            setSrc(fallbackSrc);
            setNeedsFallback(false);
            setLoadError(null);
            return;
          }
          if (fallbackAttemptedRef.current) {
            setLoadError(null);
            setIsPlaying(false);
            setIsLoading(false);
            return;
          }
          setNeedsFallback(true);
          setLoadError(null);
          setIsPlaying(false);
          setIsLoading(false);
        }}
      />

      {/* Controls row */}
      <div className="audio-controls-row">
        <button
          className="playback-button"
          onClick={() => void togglePlayback()}
          title="Play/Pause"
          disabled={!src || isLoading}
        >
          {isPlaying ? <Pause size={16} /> : <Play size={16} />}
        </button>

        {!trimMode ? (
          <input
            className="audio-slider"
            type="range"
            min={0}
            max={Math.max(duration, 0.01)}
            step={0.05}
            value={Math.min(currentTime, duration || 0)}
            onChange={(e) => onSeek(Number(e.target.value))}
            onInput={(e) => onSeek(Number((e.target as HTMLInputElement).value))}
            disabled={duration <= 0 || !src || isLoading}
            aria-label="Seek audio"
          />
        ) : (
          <div className="audio-waveform-spacer" />
        )}

        <span className="audio-time">
          {formatTime(currentTime)} / {formatTime(duration)}
        </span>

        <label className="audio-speed">
          <Rabbit size={14} />
          <select
            value={String(playbackRate)}
            onChange={(e) => onChangeSpeed(Number(e.target.value))}
            aria-label="Playback speed"
          >
            <option value="0.75">0.75x</option>
            <option value="1">1x</option>
            <option value="1.25">1.25x</option>
            <option value="1.5">1.5x</option>
            <option value="1.75">1.75x</option>
            <option value="2">2x</option>
          </select>
        </label>

        <button
          className={`trim-toggle ${trimMode ? "trim-toggle--active" : ""}`}
          onClick={() => {
            setTrimMode((prev) => !prev);
            if (!trimMode) triggerHaptic("nudge");
          }}
          title={trimMode ? "Disable trim mode" : "Enable trim mode"}
          disabled={duration <= 0 || !src}
        >
          <Scissors size={14} />
        </button>
      </div>

      {/* Waveform + trim handles */}
      {trimMode ? (
        <div className="audio-trim-panel">
          <div
            className="audio-waveform-track"
            ref={waveformTrackRef}
            onMouseDown={onWaveformMouseDown}
          >
            <canvas ref={canvasRef} className="audio-waveform-canvas" />

            {isDecodingWaveform ? (
              <div className="audio-waveform-loading">Decoding waveform...</div>
            ) : null}

            {/* Hint text when no regions */}
            {!isDecodingWaveform && regions.length === 0 && !creating ? (
              <div className="audio-waveform-hint">Click and drag to select a region</div>
            ) : null}

            {/* Live preview of region being created */}
            {creatingRegion && creatingRegion.endTime - creatingRegion.startTime >= 0.1 ? (
              <div
                className="trim-creating-overlay"
                style={{
                  left: `${(creatingRegion.startTime / duration) * 100}%`,
                  width: `${((creatingRegion.endTime - creatingRegion.startTime) / duration) * 100}%`,
                }}
              />
            ) : null}

            {/* Existing region handles */}
            {regions.map((region) => {
              const startPct = duration > 0 ? (region.startTime / duration) * 100 : 0;
              const endPct = duration > 0 ? (region.endTime / duration) * 100 : 0;
              return (
                <div key={region.id} className="trim-region-overlay">
                  <div
                    className="trim-handle trim-handle--start"
                    style={{ left: `${startPct}%` }}
                    onMouseDown={(e) => onHandleMouseDown(e, region.id, "start")}
                    title={`Start: ${formatTime(region.startTime)}`}
                  >
                    <div className="trim-handle-grip" />
                  </div>
                  <div
                    className="trim-handle trim-handle--end"
                    style={{ left: `${endPct}%` }}
                    onMouseDown={(e) => onHandleMouseDown(e, region.id, "end")}
                    title={`End: ${formatTime(region.endTime)}`}
                  >
                    <div className="trim-handle-grip" />
                  </div>
                </div>
              );
            })}
          </div>

          {/* Region chips bar */}
          {regions.length > 0 ? (
            <div className="trim-regions-bar">
              {regions.map((region) => (
                <div key={region.id} className="trim-region-chip">
                  <span>{formatTime(region.startTime)} – {formatTime(region.endTime)}</span>
                  <button className="trim-region-chip-remove" onClick={() => removeRegion(region.id)} title="Remove region">
                    <X size={10} />
                  </button>
                </div>
              ))}

              {regions.length > 1 ? (
                <button className="trim-clear-button" onClick={clearAllRegions} title="Clear all regions">
                  <Trash2 size={11} />
                  <span>Clear All</span>
                </button>
              ) : null}

              <button
                className="trim-apply-button"
                onClick={async () => {
                  if (isApplyingTrim || regions.length === 0 || !sourcePath) return;
                  setIsApplyingTrim(true);
                  try {
                    const result = await writeTrimmedAudio(
                      sourcePath,
                      regions.map((r) => ({ start: r.startTime, end: r.endTime })),
                    );

                    triggerHaptic("success");
                    onTrimApplied?.(result.path);
                  } catch (error) {
                    console.error("Failed to apply trim:", error);
                    triggerHaptic("error");
                    setLoadError(`Trim failed: ${String(error)}`);
                  } finally {
                    setIsApplyingTrim(false);
                  }
                }}
                disabled={isApplyingTrim || regions.length === 0}
                title="Apply trim and prepare for transcription"
              >
                {isApplyingTrim ? (
                  <span className="trim-apply-spinner" />
                ) : (
                  <Check size={12} />
                )}
                <span>{isApplyingTrim ? "Applying..." : "Apply Trim"}</span>
              </button>
            </div>
          ) : null}
        </div>
      ) : null}

      {loadError ? <span className="audio-error">{loadError}</span> : null}
      {isLoading ? <span className="audio-error">Loading audio...</span> : null}
    </footer>
  );
}
