# Architecture

AnyRouter Claude Keeper is split into a Rust backend and a React frontend.

## Backend

- `core/claude_runner.rs`: runs `claude -p` without shell interpolation.
- `core/classifier.rs`: maps Claude CLI output into `success`, `queue_miss`, `timeout`, and `config_error`.
- `core/scheduler.rs`: owns the 05:00-24:00 window and 60-120 second randomized loop.
- `core/redactor.rs`: redacts secrets and truncates stdout/stderr summaries.
- `storage/db.rs`: SQLite persistence, retention, and activity summaries.
- `storage/event_buffer.rs`: bounded write buffer and repeated-error compaction.
- `security/keychain.rs`: optional platform credential storage for token overrides.

## Frontend

- `src/App.tsx`: application shell.
- `src/components/StatusHero.tsx`: current state and controls.
- `src/components/ActivityHeatmap.tsx`: 24-hour activity view.
- `src/components/ProbeHistoryTable.tsx`: probe history.
- `src/components/SettingsPanel.tsx`: profile and storage configuration.

## Probe Flow

1. Scheduler checks the active time window.
2. Runner invokes Claude Code with `--no-session-persistence`, disabled tools, and JSON output.
   It uses the local Claude Code / cc-switch configuration by default. Endpoint,
   model, and token environment variables are passed only when explicitly
   configured as overrides.
3. Output is classified and summarized.
4. Event is pushed into the in-memory buffer.
5. Buffer flushes after the configured count or interval.
6. UI polls status and aggregated history.

## Disk Write Policy

The app does not persist render ticks, countdown ticks, idle scheduler ticks, or unbounded debug traces. Stable operation writes only structured probe events, configuration changes, and occasional lifecycle events.
