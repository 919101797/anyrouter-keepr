import { describe, expect, it } from "vitest";
import { formatTimeWindow, isAllDayWindow } from "./timeWindow";

describe("time window helpers", () => {
  it("uses all-day as the default window", () => {
    expect(isAllDayWindow(undefined, undefined)).toBe(true);
    expect(formatTimeWindow(undefined, undefined)).toBe("全天候");
  });

  it("formats custom windows", () => {
    expect(isAllDayWindow("09:00", "18:00")).toBe(false);
    expect(formatTimeWindow("09:00", "18:00")).toBe("09:00 - 18:00");
  });
});
