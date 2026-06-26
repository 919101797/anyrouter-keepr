# Release

This project uses GitHub Actions to build signed or unsigned Tauri bundles for
macOS, Windows, and Linux.

## Workflows

- `CI` runs lint, formatting, frontend tests, Rust tests, and a Linux smoke build
  on pushes and pull requests to `main`.
- `Release` builds desktop installers and uploads them to a GitHub Release.

The release workflow runs automatically when any git tag is pushed. It can also
be started from the GitHub Actions tab with `workflow_dispatch`.

## Required Repository Settings

In GitHub, open `Settings -> Actions -> General` and make sure workflow
permissions allow `Read and write permissions`, or keep the workflow-level
`contents: write` permission enabled.

## Updater Signing Secrets

The in-app updater uses Tauri's signed update flow. The public key is committed
in `src-tauri/tauri.conf.json`; the private key must stay out of git and be
stored as a GitHub Actions secret.

The local private key generated for this app is:

```bash
~/.tauri/anyrouter-keepr-updater.key
```

Add these repository secrets before pushing a release tag:

| Secret                               | Description                                                  |
| ------------------------------------ | ------------------------------------------------------------ |
| `TAURI_SIGNING_PRIVATE_KEY`          | The full contents of `~/.tauri/anyrouter-keepr-updater.key`. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Optional. Empty for the currently generated no-password key. |

Example with GitHub CLI:

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/anyrouter-keepr-updater.key
```

The release workflow uploads Tauri's `latest.json` updater metadata to the
GitHub Release. The app checks:

```text
https://github.com/919101797/anyrouter-keepr/releases/latest/download/latest.json
```

## macOS Signing Secrets

Without an Apple certificate the workflow uses ad-hoc signing on macOS. This is
enough to produce GitHub release artifacts, but it is not notarized.

For Developer ID signing and notarization, add these repository secrets:

| Secret                       | Description                                                                                                           |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`          | Base64 encoded `.p12` certificate. Generate with `openssl base64 -A -in certificate.p12 -out certificate-base64.txt`. |
| `APPLE_CERTIFICATE_PASSWORD` | Password used when exporting the `.p12`.                                                                              |
| `KEYCHAIN_PASSWORD`          | Optional CI keychain password. If omitted, the workflow creates a temporary one.                                      |
| `APPLE_ID`                   | Apple ID email used for notarization.                                                                                 |
| `APPLE_PASSWORD`             | Apple app-specific password.                                                                                          |
| `APPLE_TEAM_ID`              | Apple Team ID.                                                                                                        |

## Windows Signing Secrets

Without a Windows certificate the workflow produces unsigned Windows bundles.

For Windows code signing, add these repository secrets:

| Secret                         | Description                                                                 |
| ------------------------------ | --------------------------------------------------------------------------- |
| `WINDOWS_CERTIFICATE`          | Base64 encoded `.pfx` certificate.                                          |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password used when exporting the `.pfx`.                                    |
| `WINDOWS_TIMESTAMP_URL`        | Optional timestamp server URL. Defaults to `http://timestamp.digicert.com`. |

The workflow imports the `.pfx` into the runner certificate store, extracts the
thumbprint, and injects the signing settings into `src-tauri/tauri.conf.json`
only inside CI.

## Manual Release

```bash
git tag v0.1.4
git push origin v0.1.4
```

Or use the GitHub Actions tab and run the `Release` workflow manually. Manual
runs require a `release_tag` input and default to a draft release.
