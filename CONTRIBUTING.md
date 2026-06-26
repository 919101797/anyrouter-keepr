# Contributing

## Quality Gates

Run the full local checks before submitting changes:

```bash
npm run lint
npm run format:check
npm run build
npm test
cd src-tauri && cargo fmt --all -- --check
cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings
cd src-tauri && cargo test
```

## Storage Safety Rules

- Do not write countdown ticks, render ticks, or scheduler idle ticks to disk.
- Probe execution may create at most one structured event.
- Use `EventBuffer` for batched writes instead of synchronous per-loop logging.
- Keep stdout/stderr summaries bounded and redacted.
- Keep text debug logs disabled by default.
