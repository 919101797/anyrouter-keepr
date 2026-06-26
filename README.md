# AnyRouter Claude Keeper

AnyRouter Claude Keeper is a Tauri desktop app that keeps a Claude Code based
AnyRouter connection active during a configurable time window. It runs lightweight
`claude -p` probes, classifies gateway responses, and shows a 24-hour activity
view with bounded local storage.

By default it uses the existing local Claude Code configuration, including
setups managed by tools such as cc-switch. Endpoint, model, and token fields are
optional overrides, not required setup.

## Features

- Random probe loop, defaulting to every 60-120 seconds from 05:00 to 24:00.
- Treats `429`, `503`, `524`, `ECONNRESET`, and overloaded responses as queue misses; timeouts are tracked separately but still keep probing without backoff.
- Pauses only on configuration errors such as auth failure or missing Claude CLI.
- Tauri + React + TypeScript app with shadcn-style components.
- SQLite event storage with in-memory buffering and bounded retention.
- Optional token override storage through the platform credential store.
- Redacted stdout/stderr summaries with truncation.
- History table with status and recent-window filters.

## Development

```bash
npm install
npm run dev
npm run tauri:dev
```

Useful checks:

```bash
npm run build
npm test
cd src-tauri && cargo test
cd src-tauri && cargo check
```

## Release

GitHub Actions builds desktop bundles for macOS, Windows, and Linux. See
[`docs/RELEASE.md`](docs/RELEASE.md) for workflow triggers and signing secrets.

## Storage Safety

The app does not write countdown ticks, render ticks, scheduler idle ticks, or
unbounded debug traces. Probe results are buffered and flushed in batches. Text
logs are disabled by default; repeated long errors are compacted before storage.
