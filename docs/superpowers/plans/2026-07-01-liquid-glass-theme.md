# Liquid Glass Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit theme-style switching with `classic-light`, `classic-dark`, and the new `liquid-glass-light` theme while keeping existing app behavior unchanged.

**Architecture:** Introduce a new `AppTheme` preference layer that writes `data-app-theme` on the document root and keeps `data-theme` as a compatibility bridge for existing dark CSS. Replace the cycle button with an explicit picker, then add Liquid Glass shell/background/lens styling through semantic classes and a dedicated non-interactive background component.

**Tech Stack:** React 19, TypeScript, Vite, Tailwind CSS v4, Radix Select, lucide-react, Vitest.

---

## File Structure

- Create `src/lib/appTheme.ts`: theme enum, storage keys, labels, migration from old values, root attribute application.
- Create `src/lib/appTheme.test.ts`: unit tests for normalization, migration, labels, color scheme, and root attribute application.
- Create `src/lib/useAppTheme.ts`: React hook that reads, persists, and applies the app theme.
- Create `src/components/ThemePicker.tsx`: compact top-right theme selector using the existing Select primitive.
- Create `src/components/LiquidGlassBackdrop.tsx`: non-interactive SVG/CSS refraction layer rendered only as visual decoration.
- Modify `src/App.tsx`: replace `useThemeMode` and `ThemeCycleButton` with `useAppTheme`, `ThemePicker`, and Liquid Glass shell hooks.
- Modify `src/styles.css`: add `data-app-theme` token blocks, bridge classic dark selectors, add theme picker styles, and add Liquid Glass shell/lens styles.
- Keep `src/lib/theme.ts`, `src/lib/theme.test.ts`, and `src/lib/useThemeMode.ts` during the first implementation unless no imports remain and removing them is verified by `pnpm run build`.

## Task 1: Add App Theme Model

**Files:**

- Create: `src/lib/appTheme.test.ts`
- Create: `src/lib/appTheme.ts`

- [ ] **Step 1: Write the failing theme model tests**

Create `src/lib/appTheme.test.ts`:

```ts
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
```

- [ ] **Step 2: Run the new test and verify it fails**

Run:

```bash
pnpm exec vitest run src/lib/appTheme.test.ts
```

Expected: FAIL because `src/lib/appTheme.ts` does not exist.

- [ ] **Step 3: Implement the app theme helpers**

Create `src/lib/appTheme.ts`:

```ts
export const APP_THEME_STORAGE_KEY = "anyrouter-keeper-app-theme";
export const LEGACY_THEME_STORAGE_KEY = "anyrouter-keeper-theme-mode";

export type AppTheme = "classic-light" | "classic-dark" | "liquid-glass-light";
export type AppThemeColorScheme = "light" | "dark";

export interface ThemeRootTarget {
  dataset: {
    appTheme?: string;
    theme?: string;
    themeMode?: string;
  };
  style: {
    colorScheme: string;
  };
}

export const APP_THEME_OPTIONS: Array<{ value: AppTheme; label: string; description: string }> = [
  {
    value: "classic-light",
    label: "经典亮色",
    description: "保留当前浅色界面",
  },
  {
    value: "classic-dark",
    label: "经典暗色",
    description: "保留当前暗色界面",
  },
  {
    value: "liquid-glass-light",
    label: "液态玻璃",
    description: "浅色通透玻璃风格",
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

export function appThemeColorScheme(theme: AppTheme): AppThemeColorScheme {
  return theme === "classic-dark" ? "dark" : "light";
}

export function appThemeLabel(theme: AppTheme) {
  return APP_THEME_OPTIONS.find((option) => option.value === theme)?.label ?? "经典亮色";
}

export function appThemeDescription(theme: AppTheme) {
  return APP_THEME_OPTIONS.find((option) => option.value === theme)?.description ?? "保留当前浅色界面";
}

export function applyAppThemeToRoot(root: ThemeRootTarget, theme: AppTheme) {
  const colorScheme = appThemeColorScheme(theme);
  root.dataset.appTheme = theme;
  root.dataset.theme = colorScheme;
  root.dataset.themeMode = theme;
  root.style.colorScheme = colorScheme;
}
```

- [ ] **Step 4: Run the theme model test and verify it passes**

Run:

```bash
pnpm exec vitest run src/lib/appTheme.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit the theme model**

Run:

```bash
git add src/lib/appTheme.ts src/lib/appTheme.test.ts
git commit -m "feat: add app theme model"
```

## Task 2: Add App Theme Hook

**Files:**

- Modify: `src/lib/appTheme.test.ts`
- Modify: `src/lib/appTheme.ts`
- Create: `src/lib/useAppTheme.ts`

- [ ] **Step 1: Add tests for stored theme resolution**

Append these tests inside `describe("app theme helpers", () => { ... })` in `src/lib/appTheme.test.ts`:

```ts
it("resolves stored app theme before legacy values", () => {
  expect(resolveStoredAppTheme("liquid-glass-light", "dark", false)).toBe("liquid-glass-light");
  expect(resolveStoredAppTheme("classic-dark", "light", false)).toBe("classic-dark");
});

it("resolves legacy theme when no app theme exists", () => {
  expect(resolveStoredAppTheme(null, "dark", false)).toBe("classic-dark");
  expect(resolveStoredAppTheme(null, "system", true)).toBe("classic-dark");
  expect(resolveStoredAppTheme(null, "system", false)).toBe("classic-light");
});
```

Update the import list in `src/lib/appTheme.test.ts`:

```ts
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
```

- [ ] **Step 2: Run the test and verify it fails**

Run:

```bash
pnpm exec vitest run src/lib/appTheme.test.ts
```

Expected: FAIL because `resolveStoredAppTheme` is not exported.

- [ ] **Step 3: Implement stored theme resolution**

Add this function to `src/lib/appTheme.ts`:

```ts
export function resolveStoredAppTheme(
  storedAppTheme: unknown,
  legacyThemeMode: unknown,
  systemPrefersDark: boolean,
): AppTheme {
  if (
    storedAppTheme === "classic-light" ||
    storedAppTheme === "classic-dark" ||
    storedAppTheme === "liquid-glass-light"
  ) {
    return storedAppTheme;
  }

  return migrateLegacyThemeMode(legacyThemeMode, systemPrefersDark) ?? "classic-light";
}
```

- [ ] **Step 4: Create the React hook**

Create `src/lib/useAppTheme.ts`:

```ts
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
```

- [ ] **Step 5: Run the theme model test and full lib tests**

Run:

```bash
pnpm exec vitest run src/lib/appTheme.test.ts src/lib/theme.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit the app theme hook**

Run:

```bash
git add src/lib/appTheme.ts src/lib/appTheme.test.ts src/lib/useAppTheme.ts
git commit -m "feat: add app theme hook"
```

## Task 3: Replace Cycle Button With Theme Picker

**Files:**

- Create: `src/components/ThemePicker.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create the theme picker component**

Create `src/components/ThemePicker.tsx`:

```tsx
import { Palette } from "lucide-react";
import { APP_THEME_OPTIONS, appThemeLabel, type AppTheme } from "../lib/appTheme";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";

interface ThemePickerProps {
  theme: AppTheme;
  onThemeChange: (theme: AppTheme) => void;
}

export function ThemePicker({ theme, onThemeChange }: ThemePickerProps) {
  return (
    <Select value={theme} onValueChange={(value) => onThemeChange(value as AppTheme)}>
      <Tooltip>
        <TooltipTrigger asChild>
          <SelectTrigger className="theme-picker-trigger" aria-label={`主题：${appThemeLabel(theme)}`}>
            <Palette className="h-4 w-4" />
            <SelectValue />
          </SelectTrigger>
        </TooltipTrigger>
        <TooltipContent>主题：{appThemeLabel(theme)}</TooltipContent>
      </Tooltip>
      <SelectContent>
        {APP_THEME_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            <span className="theme-picker-item">
              <span>{option.label}</span>
              <span>{option.description}</span>
            </span>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
```

- [ ] **Step 2: Wire `App.tsx` to the new hook and picker**

Modify the imports in `src/App.tsx`:

```tsx
import { useEffect, useState } from "react";
import type { ComponentType } from "react";
import {
  BrainCircuit,
  CircleAlert,
  Clock4,
  HardDrive,
  Hourglass,
  Moon,
  NotebookTabs,
  Power,
  Route,
  Shuffle,
  SlidersHorizontal,
  Terminal,
  Timer,
  X,
} from "lucide-react";
import { ActivityHeatmap } from "./components/ActivityHeatmap";
import { ProbeHistoryTable } from "./components/ProbeHistoryTable";
import { SettingsPanel } from "./components/SettingsPanel";
import { StatStrip } from "./components/StatStrip";
import { StatusHero } from "./components/StatusHero";
import { ThemePicker } from "./components/ThemePicker";
import { UpdatePanel, UpdateStatusButton } from "./components/UpdatePanel";
import { Badge } from "./components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { TooltipProvider } from "./components/ui/tooltip";
import { contextSizeLabel, effectiveModelValue, effortLabel } from "./lib/modelOptions";
import { formatTimeWindow } from "./lib/timeWindow";
import { useAppTheme } from "./lib/useAppTheme";
import { useAppUpdater } from "./lib/useAppUpdater";
import { useAppStore } from "./store/appStore";
```

Replace this line:

```tsx
const { mode: themeMode, setThemeMode } = useThemeMode();
```

with:

```tsx
const { theme, setTheme } = useAppTheme();
```

Replace the theme button usage:

```tsx
<ThemeCycleButton mode={themeMode} onChange={setThemeMode} />
```

with:

```tsx
<ThemePicker theme={theme} onThemeChange={setTheme} />
```

Remove the `ThemeCycleButton` and `nextThemeMode` functions from `src/App.tsx`.

- [ ] **Step 3: Add temporary theme picker CSS**

Append this block to `src/styles.css`:

```css
.theme-picker-trigger {
  min-width: 132px;
  height: 32px;
  border-color: var(--app-border-strong) !important;
  border-radius: 7px !important;
  background: color-mix(in srgb, var(--app-surface) 76%, transparent) !important;
  color: var(--app-text-muted) !important;
  font-size: 12px !important;
  font-weight: 800 !important;
  backdrop-filter: blur(14px);
}

.theme-picker-trigger:hover,
.theme-picker-trigger[data-state="open"] {
  border-color: var(--app-focus) !important;
  background: var(--app-control-hover) !important;
  color: var(--app-text) !important;
}

.theme-picker-item {
  display: grid;
  gap: 2px;
}

.theme-picker-item span:last-child {
  color: var(--app-text-soft);
  font-size: 11px;
  font-weight: 600;
}
```

- [ ] **Step 4: Run typecheck through build**

Run:

```bash
pnpm run build
```

Expected: PASS. If it fails on an unused import in `src/App.tsx`, remove the import named in the error and rerun the command.

- [ ] **Step 5: Commit the theme picker**

Run:

```bash
git add src/App.tsx src/components/ThemePicker.tsx src/styles.css
git commit -m "feat: replace theme cycle with theme picker"
```

## Task 4: Add Liquid Glass Backdrop Component

**Files:**

- Create: `src/components/LiquidGlassBackdrop.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create a decorative backdrop component**

Create `src/components/LiquidGlassBackdrop.tsx`:

```tsx
export function LiquidGlassBackdrop() {
  return (
    <div className="lg-refraction-layer" aria-hidden="true">
      <svg viewBox="0 0 1460 830" preserveAspectRatio="none">
        <defs>
          <linearGradient id="lg-wall" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#fbfaf1" />
            <stop offset="0.44" stopColor="#edf9f1" />
            <stop offset="1" stopColor="#f9f2ff" />
          </linearGradient>
          <radialGradient id="lg-lime" cx="15%" cy="18%" r="34%">
            <stop offset="0" stopColor="#ccff32" stopOpacity="0.9" />
            <stop offset="1" stopColor="#ccff32" stopOpacity="0" />
          </radialGradient>
          <radialGradient id="lg-blue" cx="82%" cy="14%" r="34%">
            <stop offset="0" stopColor="#7497ff" stopOpacity="0.78" />
            <stop offset="1" stopColor="#7497ff" stopOpacity="0" />
          </radialGradient>
          <radialGradient id="lg-cyan" cx="76%" cy="82%" r="36%">
            <stop offset="0" stopColor="#37ddd6" stopOpacity="0.68" />
            <stop offset="1" stopColor="#37ddd6" stopOpacity="0" />
          </radialGradient>
          <filter id="lg-refraction" x="-20%" y="-20%" width="140%" height="140%">
            <feTurbulence
              type="fractalNoise"
              baseFrequency="0.008 0.016"
              numOctaves="2"
              seed="8"
              result="noise"
            />
            <feDisplacementMap
              in="SourceGraphic"
              in2="noise"
              scale="22"
              xChannelSelector="R"
              yChannelSelector="G"
              result="displaced"
            />
            <feGaussianBlur in="displaced" stdDeviation="0.7" result="blurred" />
            <feColorMatrix
              in="blurred"
              type="matrix"
              values="1.08 0 0 0 0 0 1.1 0 0 0 0 0 1.12 0 0 0 0 0 0.96 0"
            />
          </filter>
        </defs>
        <rect width="1460" height="830" fill="url(#lg-wall)" />
        <rect width="1460" height="830" fill="url(#lg-lime)" />
        <rect width="1460" height="830" fill="url(#lg-blue)" />
        <rect width="1460" height="830" fill="url(#lg-cyan)" />
        <g filter="url(#lg-refraction)" opacity="0.74">
          <path
            d="M-150 184 C130 52 340 172 560 126 C820 70 940 180 1160 132 C1290 104 1390 42 1580 76"
            fill="none"
            stroke="#ccff32"
            strokeWidth="44"
            strokeLinecap="round"
            opacity="0.42"
          />
          <path
            d="M60 648 C198 450 330 690 470 450 C650 146 776 596 980 286 C1120 72 1286 212 1440 104"
            fill="none"
            stroke="#151917"
            strokeWidth="2"
            strokeLinecap="round"
            opacity="0.16"
          />
          <path
            d="M88 566 C240 416 350 510 492 356 C666 168 812 454 1012 280 C1166 146 1276 284 1420 180"
            fill="none"
            stroke="#1fc877"
            strokeWidth="5"
            strokeLinecap="round"
            opacity="0.38"
          />
        </g>
      </svg>
    </div>
  );
}
```

- [ ] **Step 2: Render the backdrop in `App.tsx`**

Add the import:

```tsx
import { LiquidGlassBackdrop } from "./components/LiquidGlassBackdrop";
```

Place the component as the first child inside `<main>`:

```tsx
      <main className="app-root h-screen overflow-auto overflow-x-hidden p-3 text-[#17211d] sm:p-5">
        <LiquidGlassBackdrop />
        <div className="app-shell mx-auto flex min-h-full max-w-[1500px] flex-col gap-4">
```

Update the closing `div` that matches the shell wrapper without changing child order.

- [ ] **Step 3: Run build**

Run:

```bash
pnpm run build
```

Expected: PASS.

- [ ] **Step 4: Commit the backdrop component**

Run:

```bash
git add src/App.tsx src/components/LiquidGlassBackdrop.tsx
git commit -m "feat: add liquid glass backdrop"
```

## Task 5: Add Theme Tokens And Classic Bridges

**Files:**

- Modify: `src/styles.css`

- [ ] **Step 1: Add app theme token blocks**

Modify the top of `src/styles.css` so the first root selector starts with:

```css
:root,
:root[data-app-theme="classic-light"],
:root[data-app-theme="liquid-glass-light"] {
```

Modify the current dark root selector from:

```css
:root[data-theme="dark"] {
```

to:

```css
:root[data-theme="dark"],
:root[data-app-theme="classic-dark"] {
```

- [ ] **Step 2: Bridge dark selectors**

For every selector that starts with `:root[data-theme="dark"]`, add the matching `:root[data-app-theme="classic-dark"]` selector beside it. For example, change:

```css
:root[data-theme="dark"] body {
```

to:

```css
:root[data-theme="dark"] body,
:root[data-app-theme="classic-dark"] body {
```

Run this search until no dark-only selector remains:

```bash
rg -n ':root\\[data-theme="dark"\\]' src/styles.css
```

Expected: each result should either already include `:root[data-app-theme="classic-dark"]` in the same selector group or be edited immediately.

- [ ] **Step 3: Add Liquid Glass base tokens and background**

Append this block near the root token section in `src/styles.css`:

```css
:root[data-app-theme="liquid-glass-light"] {
  --app-text: #151917;
  --app-bg: #f8f8f2;
  --app-surface: rgba(255, 255, 255, 0.58);
  --app-surface-muted: rgba(255, 255, 255, 0.36);
  --app-surface-raised: rgba(255, 255, 255, 0.72);
  --app-border: rgba(255, 255, 255, 0.62);
  --app-border-strong: rgba(21, 25, 23, 0.12);
  --app-text-muted: rgba(21, 25, 23, 0.62);
  --app-text-soft: rgba(21, 25, 23, 0.46);
  --app-accent: #cfff35;
  --app-focus: #1fc877;
  --app-row-hover: rgba(255, 255, 255, 0.44);
  --app-control-hover: rgba(255, 255, 255, 0.58);
  --app-control-hover-strong: rgba(255, 255, 255, 0.72);
  --select-hover: rgba(255, 255, 255, 0.6);
  --select-hover-text: #151917;
  --hero-bg: rgba(255, 255, 255, 0.62);
  --hero-bg-tint: rgba(255, 255, 255, 0.36);
  --hero-text: #121614;
  --hero-muted: rgba(21, 25, 23, 0.62);
  --hero-soft: rgba(21, 25, 23, 0.48);
  --hero-border: rgba(255, 255, 255, 0.62);
  --hero-border-subtle: rgba(21, 25, 23, 0.11);
  --hero-surface: rgba(255, 255, 255, 0.28);
  --hero-surface-strong: rgba(255, 255, 255, 0.48);
  --hero-grid: rgba(21, 25, 23, 0.035);
  --hero-line-lime: rgba(207, 255, 53, 0.32);
  --hero-line-cyan: rgba(55, 221, 214, 0.24);
}

:root[data-app-theme="liquid-glass-light"] body {
  background:
    radial-gradient(circle at 14% 18%, rgba(204, 255, 50, 0.62), transparent 21%),
    radial-gradient(circle at 82% 12%, rgba(116, 151, 255, 0.62), transparent 22%),
    radial-gradient(circle at 74% 82%, rgba(55, 221, 214, 0.52), transparent 25%),
    radial-gradient(circle at 22% 88%, rgba(255, 145, 180, 0.42), transparent 21%),
    linear-gradient(135deg, #fbfaf1 0%, #ecf8f2 45%, #f8f3ff 100%);
}
```

- [ ] **Step 4: Run CSS-sensitive build**

Run:

```bash
pnpm run build
```

Expected: PASS.

- [ ] **Step 5: Commit theme token bridges**

Run:

```bash
git add src/styles.css
git commit -m "feat: add app theme style tokens"
```

## Task 6: Style Liquid Glass Shell And Controls

**Files:**

- Modify: `src/App.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Add semantic shell classes in `App.tsx`**

Update these class names in `src/App.tsx`:

```tsx
      <main className="app-root h-screen overflow-auto overflow-x-hidden p-3 text-[#17211d] sm:p-5">
        <LiquidGlassBackdrop />
        <div className="app-shell mx-auto flex min-h-full max-w-[1500px] flex-col gap-4">
          <header className="app-header flex flex-col gap-3 rounded-[10px] border border-[#d2ded7] bg-white/[0.82] px-4 py-3 shadow-[0_14px_42px_rgba(36,55,47,0.08)] backdrop-blur md:flex-row md:items-center md:justify-between">
```

Change the main content wrapper:

```tsx
          <div className="app-content flex min-h-0 flex-1 flex-col gap-4">
            <div className="app-primary min-h-0 min-w-0 space-y-4 overflow-visible">
```

Change the aside:

```tsx
            <aside className="app-secondary min-h-0 min-w-0 overflow-visible">
```

- [ ] **Step 2: Add Liquid Glass shell CSS**

Append this block to `src/styles.css`:

```css
.app-root {
  position: relative;
  isolation: isolate;
}

.app-shell {
  position: relative;
  z-index: 1;
}

.lg-refraction-layer {
  display: none;
}

:root[data-app-theme="liquid-glass-light"] .lg-refraction-layer {
  pointer-events: none;
  position: fixed;
  inset: 0;
  z-index: 0;
  display: block;
  overflow: hidden;
  opacity: 0.72;
  filter: saturate(1.08);
}

:root[data-app-theme="liquid-glass-light"] .lg-refraction-layer svg {
  width: 100%;
  height: 100%;
}

:root[data-app-theme="liquid-glass-light"] .app-shell {
  max-width: 1500px;
}

:root[data-app-theme="liquid-glass-light"] .app-header,
:root[data-app-theme="liquid-glass-light"] .theme-picker-trigger,
:root[data-app-theme="liquid-glass-light"] .update-status-button {
  border-color: rgba(255, 255, 255, 0.58) !important;
  background:
    linear-gradient(135deg, rgba(255, 255, 255, 0.34), rgba(255, 255, 255, 0.08) 48%),
    rgba(255, 255, 255, 0.08) !important;
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.92),
    inset -1px 0 0 rgba(116, 151, 255, 0.22),
    0 18px 38px rgba(47, 58, 52, 0.13) !important;
  backdrop-filter: blur(18px) saturate(1.8) contrast(1.04);
}

:root[data-app-theme="liquid-glass-light"] .panel-ring {
  border-color: rgba(255, 255, 255, 0.58) !important;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.66), rgba(255, 255, 255, 0.34)), rgba(255, 255, 255, 0.3) !important;
  box-shadow:
    0 24px 70px rgba(45, 58, 51, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 0.88);
  backdrop-filter: blur(14px) saturate(1.25);
}

:root[data-app-theme="liquid-glass-light"] .status-hero {
  background:
    linear-gradient(120deg, rgba(255, 255, 255, 0.74), rgba(255, 255, 255, 0.38)),
    radial-gradient(circle at 82% 82%, rgba(207, 255, 53, 0.42), transparent 34%) !important;
}

:root[data-app-theme="liquid-glass-light"] .status-hero-select-shell,
:root[data-app-theme="liquid-glass-light"] .status-hero-side-panel,
:root[data-app-theme="liquid-glass-light"] .status-hero-metric,
:root[data-app-theme="liquid-glass-light"] .status-hero-side-icon,
:root[data-app-theme="liquid-glass-light"] .history-filter-active,
:root[data-app-theme="liquid-glass-light"] .history-filter-button {
  border-color: rgba(255, 255, 255, 0.58) !important;
  background: rgba(255, 255, 255, 0.22) !important;
  backdrop-filter: blur(18px) saturate(1.55);
}
```

- [ ] **Step 3: Run build**

Run:

```bash
pnpm run build
```

Expected: PASS.

- [ ] **Step 4: Commit Liquid Glass shell styling**

Run:

```bash
git add src/App.tsx src/styles.css
git commit -m "feat: style liquid glass shell"
```

## Task 7: Manual Verification And Final Cleanup

**Files:**

- Modify after inspection: `src/styles.css`
- Modify after inspection: `src/App.tsx`

- [ ] **Step 1: Run all automated checks**

Run:

```bash
pnpm run test
pnpm run build
```

Expected: both commands PASS.

- [ ] **Step 2: Start the dev server**

Run:

```bash
pnpm run dev
```

Expected: Vite starts on `http://127.0.0.1:1420/`.

- [ ] **Step 3: Verify theme switching manually**

Open `http://127.0.0.1:1420/` and verify:

- `经典亮色` keeps the original light look.
- `经典暗色` keeps the original dark look.
- `液态玻璃` shows the light colorful background, visible refraction layer, and glass-like top controls.
- Refresh keeps the selected theme.

- [ ] **Step 4: Verify old storage migration manually**

In the browser console, run:

```js
localStorage.removeItem("anyrouter-keeper-app-theme");
localStorage.setItem("anyrouter-keeper-theme-mode", "dark");
location.reload();
```

Expected: app loads in `经典暗色`.

Then run:

```js
localStorage.removeItem("anyrouter-keeper-app-theme");
localStorage.setItem("anyrouter-keeper-theme-mode", "light");
location.reload();
```

Expected: app loads in `经典亮色`.

- [ ] **Step 5: Verify functional flows**

Use the UI and verify these controls still call their original handlers:

- Start or pause guardian.
- Run one probe.
- Save settings.
- Switch settings/runtime tabs.
- Filter history.
- Open update panel.
- Dismiss startup notice.

- [ ] **Step 6: Stop the dev server**

Stop the running Vite process with `Ctrl+C`.

Expected: terminal returns to the prompt.

- [ ] **Step 7: Commit verification adjustments**

If manual verification required CSS or App shell fixes, commit them:

```bash
git add src/App.tsx src/styles.css
git commit -m "fix: polish theme switching visuals"
```

If no files changed, record the clean state:

```bash
git status --short
```

Expected: no output.
