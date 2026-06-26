import { describe, expect, it } from "vitest";
import { normalizeThemeMode, resolveThemeMode } from "./theme";

describe("theme mode helpers", () => {
  it("normalizes unknown values to system", () => {
    expect(normalizeThemeMode("light")).toBe("light");
    expect(normalizeThemeMode("dark")).toBe("dark");
    expect(normalizeThemeMode("system")).toBe("system");
    expect(normalizeThemeMode("sepia")).toBe("system");
    expect(normalizeThemeMode(null)).toBe("system");
  });

  it("resolves system mode from media preference", () => {
    expect(resolveThemeMode("light", true)).toBe("light");
    expect(resolveThemeMode("dark", false)).toBe("dark");
    expect(resolveThemeMode("system", true)).toBe("dark");
    expect(resolveThemeMode("system", false)).toBe("light");
  });
});
