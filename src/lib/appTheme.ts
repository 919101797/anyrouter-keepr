export const APP_THEME_STORAGE_KEY = "anyrouter-keeper-app-theme";
export const LEGACY_THEME_STORAGE_KEY = "anyrouter-keeper-theme-mode";

export type AppTheme = "classic-light" | "classic-dark" | "liquid-glass-light";
export type AppThemeColorScheme = "light" | "dark";

export interface ThemeRootTarget {
  dataset: {
    appTheme?: string;
    theme?: string;
  };
  style: {
    colorScheme: string;
  };
}

export const APP_THEME_OPTIONS: Array<{ value: AppTheme; label: string }> = [
  {
    value: "classic-light",
    label: "经典亮色",
  },
  {
    value: "classic-dark",
    label: "经典暗色",
  },
  {
    value: "liquid-glass-light",
    label: "液态玻璃",
  },
];

export function normalizeAppTheme(value: unknown): AppTheme {
  return value === "classic-dark" || value === "liquid-glass-light" || value === "classic-light"
    ? value
    : "classic-light";
}

export function migrateLegacyThemeMode(value: unknown, systemPrefersDark: boolean): AppTheme | null {
  if (value === "light") return "classic-light";
  if (value === "dark") return "classic-dark";
  if (value === "system") return systemPrefersDark ? "classic-dark" : "classic-light";
  return null;
}

export function resolveStoredAppTheme(
  storedAppTheme: unknown,
  legacyThemeMode: unknown,
  systemPrefersDark: boolean,
): AppTheme {
  return resolveStoredAppThemeState(storedAppTheme, legacyThemeMode, systemPrefersDark).theme;
}

export function resolveStoredAppThemeState(
  storedAppTheme: unknown,
  legacyThemeMode: unknown,
  systemPrefersDark: boolean,
): { theme: AppTheme; shouldPersist: boolean } {
  if (
    storedAppTheme === "classic-light" ||
    storedAppTheme === "classic-dark" ||
    storedAppTheme === "liquid-glass-light"
  ) {
    return { theme: storedAppTheme, shouldPersist: false };
  }

  const migratedTheme = migrateLegacyThemeMode(legacyThemeMode, systemPrefersDark);
  if (migratedTheme) return { theme: migratedTheme, shouldPersist: true };
  return { theme: "classic-light", shouldPersist: false };
}

export function appThemeColorScheme(theme: AppTheme): AppThemeColorScheme {
  return theme === "classic-dark" ? "dark" : "light";
}

export function appThemeLabel(theme: AppTheme) {
  return APP_THEME_OPTIONS.find((option) => option.value === theme)?.label ?? "经典亮色";
}

export function applyAppThemeToRoot(root: ThemeRootTarget, theme: AppTheme) {
  const colorScheme = appThemeColorScheme(theme);
  root.dataset.appTheme = theme;
  root.dataset.theme = colorScheme;
  root.style.colorScheme = colorScheme;
}
