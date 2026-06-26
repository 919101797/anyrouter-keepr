# Testing

## Frontend

```bash
npm run lint
npm run format:check
npm run build
npm test
```

## Backend

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Release Build

```bash
npm run tauri:build
```

This currently builds the optimized application without bundling installers.
