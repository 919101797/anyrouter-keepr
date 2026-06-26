import { describe, expect, it, vi } from "vitest";
import { formatDuration, formatLongDuration, formatRelativeTime, statusLabel } from "./utils";

describe("ui utility formatting", () => {
  it("formats duration without writing state", () => {
    expect(formatDuration(420)).toBe("420ms");
    expect(formatDuration(1420)).toBe("1.4s");
    expect(formatDuration(null)).toBe("-");
  });

  it("formats long duration for monitoring stats", () => {
    expect(formatLongDuration(42_000)).toBe("42.0s");
    expect(formatLongDuration(5 * 60_000)).toBe("5 分钟");
    expect(formatLongDuration(90 * 60_000)).toBe("1.5 小时");
  });

  it("formats relative time", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-06-26T10:00:00Z"));
    expect(formatRelativeTime("2026-06-26T09:58:00Z")).toBe("2 分钟前");
    expect(formatRelativeTime(null)).toBe("从未");
    vi.useRealTimers();
  });

  it("maps statuses to Chinese labels", () => {
    expect(statusLabel("success")).toBe("已联通");
    expect(statusLabel("queue_miss")).toBe("抢占中");
    expect(statusLabel("config_error")).toBe("配置错误");
  });
});
