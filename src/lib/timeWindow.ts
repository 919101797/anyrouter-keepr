export const DEFAULT_START_TIME = "00:00";
export const DEFAULT_END_TIME = "24:00";

export function normalizeTimeLabel(value: string | null | undefined, fallback: string) {
  const trimmed = value?.trim();
  return trimmed || fallback;
}

export function isAllDayWindow(startTime: string | null | undefined, endTime: string | null | undefined) {
  return (
    normalizeTimeLabel(startTime, DEFAULT_START_TIME) === DEFAULT_START_TIME &&
    normalizeTimeLabel(endTime, DEFAULT_END_TIME) === DEFAULT_END_TIME
  );
}

export function formatTimeWindow(startTime: string | null | undefined, endTime: string | null | undefined) {
  const start = normalizeTimeLabel(startTime, DEFAULT_START_TIME);
  const end = normalizeTimeLabel(endTime, DEFAULT_END_TIME);
  return isAllDayWindow(start, end) ? "全天候" : `${start} - ${end}`;
}
