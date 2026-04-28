# SpeakEx

SpeakEx is a local-first desktop transcription app built with Tauri v2, SvelteKit, TypeScript, and Rust.

The current MVP lets you record audio from the main window, tray, or global shortcut, send completed recordings to Gemini for transcription, automatically copy results to the clipboard, and keep a local transcription history on your machine.

## MVP highlights

- Local-first desktop workflow with no separate backend service
- Manual recording controls in the app, tray, and through a global shortcut
- Gemini-powered transcription for completed recordings
- Local history, clipboard copy, and settings for retention behavior
- Privacy-aware hidden-window notifications for transcription outcomes

## Scripts

- `npm run dev` — start the frontend dev server
- `npm run tauri dev` — run the desktop app in development; on Linux hosts this automatically uses the `speakex-dev` Toolbox container
- `npm run test` — run the unit and integration test suite
- `npm run coverage` — run the test suite with coverage reporting
- `npm run check` — run the Svelte/TypeScript checks
- `npm run build` — build the frontend bundle
- `npm run tauri build` — build the desktop application; on Linux hosts this automatically uses the `speakex-dev` Toolbox container

## Cross-platform validation

SpeakEx now includes a GitHub Actions workflow at `.github/workflows/cross-platform-validation.yml` that validates the existing desktop build surface on:

- `ubuntu-latest`
- `macos-latest`
- `windows-latest`

On every push and pull request, the workflow runs:

- `npm ci`
- `npm run check`
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- Linux only: `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`
- Linux only: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `npm run tauri build -- --debug --no-bundle`

This validates that the frontend, Rust backend, and Tauri desktop app still compile across Linux, macOS, and Windows without requiring platform-specific release bundling in CI.

## Linux packaging

SpeakEx is currently configured for a Linux-first AppImage packaging path.

- On Linux hosts, run the packaging build from the repo root with `npm run tauri build`; the wrapper script automatically uses the `speakex-dev` Toolbox container.
- If you are already inside that Toolbox environment, `npm run tauri build` runs locally inside the container without nesting Toolbox calls.
- The Linux build wrapper automatically sets `NO_STRIP=1` for `tauri build` so AppImage bundling skips the failing `linuxdeploy` strip step on `.relr.dyn` libraries.
- When AppImage bundling succeeds, expect the final bundle under `src-tauri/target/release/bundle/appimage/`, with the release artifact appearing there as `SpeakEx_0.1.0_amd64.AppImage`.

## Notes

SpeakEx is currently packaged and validated as a desktop MVP with a Linux-first release path and cross-platform no-bundle CI checks. Detailed transcription state, settings, and history stay on the local machine, while Gemini is the current transcription provider for completed recordings.
