export function clampPercentage(value: number): number {
  if (!Number.isFinite(value)) return 0;
  if (value < 0) return 0;
  if (value > 100) return 100;
  return value;
}

export function makeProgressVisible(value: number): number {
  const clamped = clampPercentage(value);
  if (clamped > 0 && clamped < 1) {
    return 1;
  }
  return clamped;
}

export function formatProgressPercentageLabel(value: number): string {
  const rounded = Math.round(makeProgressVisible(value));
  if (rounded === 100) {
    return "100%";
  }
  return `${String(rounded).padStart(2, "0")}%`;
}
