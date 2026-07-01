# Liquid Glass Theme Design

Date: 2026-07-01

## Goal

Add theme-style switching to AnyRouter Keeper so users can choose the visual style they prefer. Keep the current light and dark UI as first-class themes, and add one new Liquid Glass light theme inspired by the approved V5 visual direction.

This is a frontend style and structure refactor. It must not change scheduler behavior, profile persistence, probe execution, updater behavior, event history data, or Tauri backend contracts.

## Confirmed Theme Model

Use one theme preference instead of separate color mode and visual-theme state:

```ts
type AppTheme = "classic-light" | "classic-dark" | "liquid-glass-light";
```

Themes:

- `classic-light`: the current original light appearance.
- `classic-dark`: the current original dark appearance.
- `liquid-glass-light`: a new light Liquid Glass-inspired theme.

The root document should expose a single app theme attribute:

```html
<html data-app-theme="liquid-glass-light">
```

The existing `data-theme` / `data-theme-mode` approach can be migrated or bridged during implementation, but the final public styling contract should be `data-app-theme`.

## Design References

Apple describes Liquid Glass as a translucent material that reflects and refracts its surroundings while transforming to focus attention on content. Apple Developer's Liquid Glass session frames it as a dynamic design layer with optical and physical properties. This project should use those ideas as inspiration, not attempt to clone Apple system components exactly.

References:

- Apple Newsroom: "Apple introduces a delightful and elegant new software design" — https://www.apple.com/newsroom/2025/06/apple-introduces-a-delightful-and-elegant-new-software-design/
- Apple Developer: "Meet Liquid Glass" WWDC25 — https://developer.apple.com/videos/play/wwdc2025/219/
- Apple Human Interface Guidelines: Liquid Glass — https://developer.apple.com/design/human-interface-guidelines/liquid-glass

## Visual Direction

The approved direction is the V5 SVG refraction prototype:

- Light-first, colorful, airy background.
- Functional controls feel like transparent lenses rather than frosted white blocks.
- Glass areas show visible refraction, edge highlights, and prism-like color spill.
- Content panels remain readable and mostly non-glass.
- Liquid Glass appears on navigation, toolbar, floating filters, transient controls, and action clusters.

The theme should avoid:

- Turning all cards into glass.
- Dark neon monitoring-console styling.
- White fog overlays that obscure the background without convincing refraction.
- Glass-on-glass stacking.
- A Liquid Glass dark mode in the first implementation.

## Information Architecture

Business-facing layout should remain familiar:

- Status hero remains the primary top-level status surface.
- Stats, activity, history, settings, and runtime panels remain available.
- Existing tabs for settings/runtime can remain, unless implementation finds a small shell-level rearrangement necessary for the new theme.

The Liquid Glass theme may alter shell structure and panel presentation:

- Top controls may become a floating toolbar.
- Theme switching may move into a dedicated theme picker.
- Filter buttons and primary actions may become floating lens controls.
- Background may include a non-interactive signal/refraction layer.

## Component Boundaries

Introduce theme-specific structure without duplicating business logic.

Recommended new/changed modules:

- `src/lib/appTheme.ts`: theme enum, storage key, normalization, labels.
- `src/lib/useAppTheme.ts`: reads/writes theme preference and applies `data-app-theme`.
- `src/components/ThemePicker.tsx`: user-facing theme switcher.
- `src/components/AppShell.tsx` or equivalent: optional wrapper for shared shell/background behavior.

Existing components should keep their data props and callbacks:

- `StatusHero`
- `StatStrip`
- `ActivityHeatmap`
- `ProbeHistoryTable`
- `SettingsPanel`
- `RuntimePanel` currently inside `App.tsx`
- `UpdatePanel`
- `StartupNotice`

Where styling is too embedded in literal Tailwind color classes, introduce shared semantic classes and CSS variables instead of duplicating components.

## Styling Architecture

Use theme tokens and semantic class hooks.

Root theme blocks:

```css
:root[data-app-theme="classic-light"] {
  /* current light tokens */
}

:root[data-app-theme="classic-dark"] {
  /* current dark tokens */
}

:root[data-app-theme="liquid-glass-light"] {
  /* Liquid Glass light tokens */
}
```

Shared semantic classes should cover:

- App background and shell.
- Panel surfaces.
- Floating controls.
- Buttons and icon buttons.
- Tabs/segmented filters.
- Status and history tone colors.
- Form controls.

Liquid Glass-only classes can include:

- `.lg-wallpaper`
- `.lg-refraction-layer`
- `.lg-lens`
- `.lg-floating-toolbar`
- `.lg-floating-filter`
- `.lg-action-cluster`

The first implementation can approximate V5 with CSS/SVG layers. If pure CSS cannot deliver enough refraction quality, the design allows a focused SVG or canvas background/refraction layer for the Liquid Glass theme only.

## Theme Picker UX

Replace the current cycle-only theme button with a more explicit picker. The control should make all three available choices discoverable:

- 经典亮色
- 经典暗色
- 液态玻璃

Use a compact menu triggered from the existing top-right theme control. It should show the current theme and persist immediately on selection.

Migration behavior:

- Existing stored `"light"` maps to `"classic-light"`.
- Existing stored `"dark"` maps to `"classic-dark"`.
- Existing stored `"system"` resolves once from current system preference: dark preference maps to `"classic-dark"`, light/no preference maps to `"classic-light"`.
- Unknown values normalize to `"classic-light"`.

## Accessibility And Usability

Liquid Glass must remain usable:

- Text contrast in panels and controls must remain readable.
- Glass controls must not sit on busy content unless the lens effect preserves legibility.
- Motion or animated refraction should be subtle and avoid constant distraction.
- History table remains scrollable and scan-friendly.
- Settings form remains dense but legible.
- Narrow windows must not overlap text or controls.

## Out Of Scope

- Liquid Glass dark theme.
- User-custom color palettes.
- Backend changes.
- New scheduler, updater, profile, or event-history features.
- Pixel-perfect Apple system component cloning.
- Replacing the existing Classic themes.

## Verification

Run at minimum:

```bash
pnpm run test
pnpm run build
```

Manual checks:

- Switch between all three themes.
- Refresh and confirm theme persistence.
- Confirm old stored theme values migrate safely.
- Check status hero, history table, settings panel, runtime panel, updater panel, and startup notice in each theme.
- Check a narrow viewport for text overlap and unusable controls.

## Implementation Notes For Future Plan

Prefer an incremental internal refactor:

1. Add `AppTheme` model and tests.
2. Replace theme hook/button with explicit theme picker.
3. Add root theme tokens for the three themes while keeping Classic visually stable.
4. Convert hard-coded color classes that block theming into semantic classes.
5. Add Liquid Glass shell/background/lens styling.
6. Verify all three themes.

Implementation should stop and reassess if Liquid Glass requires broad component duplication. The intended boundary is styling and shell structure, not parallel business components.
