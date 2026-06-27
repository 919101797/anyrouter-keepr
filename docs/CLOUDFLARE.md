# Cloudflare Updater Feed

Cloudflare Pages hosts the public Tauri updater feed for the desktop app:

```text
https://anyrouter-claude-keeper.pages.dev/latest.json
```

The Pages project is not used as the product frontend. It stores the release
metadata and update bundles that installed desktop apps download anonymously.

## First-time setup

```bash
pnpm install
pnpm run cloudflare:login
pnpm run cloudflare:create
```

If the Pages project already exists, skip `pnpm run cloudflare:create`.

## GitHub Actions Secrets

Add these repository secrets before pushing a release tag:

- `CLOUDFLARE_ACCOUNT_ID`
- `CLOUDFLARE_API_TOKEN`

The token needs Cloudflare Pages edit access for the target account. Wrangler
uses these environment variables in CI to deploy the generated updater feed.

## Release Flow

1. Push a git tag such as `v0.1.10`.
2. The `Release` workflow builds signed Tauri bundles for macOS, Windows, and
   Linux.
3. The publish job downloads the GitHub Release assets into CI, rewrites
   `latest.json` so every platform URL points to Cloudflare Pages, and deploys
   the staged updater feed.
4. The workflow verifies `https://anyrouter-claude-keeper.pages.dev/latest.json`.
5. Only after the Cloudflare feed is valid does the workflow publish the GitHub
   Release.

Regular branch pushes do not deploy Cloudflare Pages. Manual release workflow
runs only publish to Cloudflare when `release_draft` is `false`.

## Useful commands

```bash
pnpm run cloudflare:whoami
pnpm run cloudflare:deploy
```
