import { useCallback, useEffect, useState } from "react";
import {
  APP_THEME_STORAGE_KEY,
  LEGACY_THEME_STORAGE_KEY,
  applyAppThemeToRoot,
  normalizeAppTheme,
  resolveStoredAppTheme,
  type AppTheme,
} from "./appTheme";

const mediaQuery = "(prefers-color-scheme: dark)";

export function useAppTheme() {
  const [theme, setThemeState] = useState<AppTheme>(() => readStoredAppTheme());

  useEffect(() => {
    applyAppThemeToRoot(document.documentElement, theme);
  }, [theme]);

  const setTheme = useCallback((nextTheme: AppTheme) => {
    const normalized = normalizeAppTheme(nextTheme);
    setThemeState(normalized);
    try {
      window.localStorage.setItem(APP_THEME_STORAGE_KEY, normalized);
    } catch {
      // Local storage may be unavailable in private or locked-down WebViews.
    }
  }, []);

  return { theme, setTheme };
}

function readStoredAppTheme(): AppTheme {
  if (typeof window === "undefined") return "classic-light";
  try {
    return resolveStoredAppTheme(
      window.localStorage.getItem(APP_THEME_STORAGE_KEY),
      window.localStorage.getItem(LEGACY_THEME_STORAGE_KEY),
      readSystemPrefersDark(),
    );
  } catch {
    return "classic-light";
  }
}

function readSystemPrefersDark() {
  return typeof window !== "undefined" && window.matchMedia?.(mediaQuery).matches;
}
