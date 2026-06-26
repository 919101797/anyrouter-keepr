export const THEME_STORAGE_KEY = "anyrouter-keeper-theme-mode";

export type ThemeMode = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export function normalizeThemeMode(value: unknown): ThemeMode {
  return value === "light" || value === "dark" || value === "system" ? value : "system";
}

export function resolveThemeMode(mode: ThemeMode, systemPrefersDark: boolean): ResolvedTheme {
  if (mode === "dark") return "dark";
  if (mode === "light") return "light";
  return systemPrefersDark ? "dark" : "light";
}

export function themeModeLabel(mode: ThemeMode) {
  switch (mode) {
    case "dark":
      return "暗色";
    case "light":
      return "亮色";
    default:
      return "跟随系统";
  }
}

export function resolvedThemeLabel(theme: ResolvedTheme) {
  return theme === "dark" ? "暗色" : "亮色";
}
