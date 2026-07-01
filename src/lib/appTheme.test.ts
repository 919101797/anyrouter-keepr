import { describe, expect, it } from "vitest";
import {
  APP_THEME_OPTIONS,
  APP_THEME_STORAGE_KEY,
  LEGACY_THEME_STORAGE_KEY,
  appThemeColorScheme,
  appThemeLabel,
  applyAppThemeToRoot,
  migrateLegacyThemeMode,
  normalizeAppTheme,
  resolveStoredAppTheme,
  type ThemeRootTarget,
} from "./appTheme";

describe("app theme helpers", () => {
  it("exposes the new storage keys", () => {
    expect(APP_THEME_STORAGE_KEY).toBe("anyrouter-keeper-app-theme");
    expect(LEGACY_THEME_STORAGE_KEY).toBe("anyrouter-keeper-theme-mode");
  });

  it("normalizes supported themes and falls back to classic light", () => {
    expect(normalizeAppTheme("classic-light")).toBe("classic-light");
    expect(normalizeAppTheme("classic-dark")).toBe("classic-dark");
    expect(normalizeAppTheme("liquid-glass-light")).toBe("liquid-glass-light");
    expect(normalizeAppTheme("system")).toBe("classic-light");
    expect(normalizeAppTheme(null)).toBe("classic-light");
  });

  it("migrates old light dark and system values", () => {
    expect(migrateLegacyThemeMode("light", false)).toBe("classic-light");
    expect(migrateLegacyThemeMode("dark", false)).toBe("classic-dark");
    expect(migrateLegacyThemeMode("system", true)).toBe("classic-dark");
    expect(migrateLegacyThemeMode("system", false)).toBe("classic-light");
    expect(migrateLegacyThemeMode("sepia", true)).toBeNull();
  });

  it("resolves stored app theme before legacy values", () => {
    expect(resolveStoredAppTheme("liquid-glass-light", "dark", false)).toBe("liquid-glass-light");
    expect(resolveStoredAppTheme("classic-dark", "light", false)).toBe("classic-dark");
  });

  it("resolves legacy theme when no app theme exists", () => {
    expect(resolveStoredAppTheme(null, "dark", false)).toBe("classic-dark");
    expect(resolveStoredAppTheme(null, "system", true)).toBe("classic-dark");
    expect(resolveStoredAppTheme(null, "system", false)).toBe("classic-light");
  });

  it("returns labels and color schemes for every option", () => {
    expect(APP_THEME_OPTIONS.map((option) => option.value)).toEqual([
      "classic-light",
      "classic-dark",
      "liquid-glass-light",
    ]);
    expect(appThemeLabel("classic-light")).toBe("经典亮色");
    expect(appThemeLabel("classic-dark")).toBe("经典暗色");
    expect(appThemeLabel("liquid-glass-light")).toBe("液态玻璃");
    expect(appThemeColorScheme("classic-light")).toBe("light");
    expect(appThemeColorScheme("classic-dark")).toBe("dark");
    expect(appThemeColorScheme("liquid-glass-light")).toBe("light");
  });

  it("applies root attributes for the selected theme", () => {
    const root = { dataset: {}, style: { colorScheme: "" } } as ThemeRootTarget;

    applyAppThemeToRoot(root, "liquid-glass-light");

    expect(root.dataset.appTheme).toBe("liquid-glass-light");
    expect(root.dataset.theme).toBe("light");
    expect(root.style.colorScheme).toBe("light");
  });
});
