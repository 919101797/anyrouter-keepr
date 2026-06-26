import { useCallback, useEffect, useMemo, useState } from "react";
import {
  normalizeThemeMode,
  resolveThemeMode,
  THEME_STORAGE_KEY,
  type ResolvedTheme,
  type ThemeMode,
} from "./theme";

const mediaQuery = "(prefers-color-scheme: dark)";

export function useThemeMode() {
  const [mode, setModeState] = useState<ThemeMode>(() => readStoredThemeMode());
  const [systemPrefersDark, setSystemPrefersDark] = useState(() => readSystemPrefersDark());
  const resolvedTheme = useMemo<ResolvedTheme>(
    () => resolveThemeMode(mode, systemPrefersDark),
    [mode, systemPrefersDark],
  );

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const media = window.matchMedia(mediaQuery);
    const update = () => setSystemPrefersDark(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.themeMode = mode;
    root.dataset.theme = resolvedTheme;
    root.style.colorScheme = resolvedTheme;
  }, [mode, resolvedTheme]);

  const setThemeMode = useCallback((nextMode: ThemeMode) => {
    const normalized = normalizeThemeMode(nextMode);
    setModeState(normalized);
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, normalized);
    } catch {
      // Local storage may be unavailable in private or locked-down WebViews.
    }
  }, []);

  return { mode, resolvedTheme, setThemeMode };
}

function readStoredThemeMode(): ThemeMode {
  if (typeof window === "undefined") return "system";
  try {
    return normalizeThemeMode(window.localStorage.getItem(THEME_STORAGE_KEY));
  } catch {
    return "system";
  }
}

function readSystemPrefersDark() {
  return typeof window !== "undefined" && window.matchMedia?.(mediaQuery).matches;
}
