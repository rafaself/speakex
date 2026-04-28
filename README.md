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
- When AppImage bundling succeeds, expect the final bundle under `src-tauri/target/release/bundle/appimage/`, with the release artifact appearing there as `SpeakEx_0.1.0_amd64.AppImage`.

### Current environment limitation

In this Fedora 43 packaging environment, the build reaches the AppImage bundling phase and stages `src-tauri/target/release/bundle/appimage/SpeakEx.AppDir`, but `linuxdeploy` then fails during its `strip` step on bundled libraries that contain `.relr.dyn` ELF sections:

- `unknown type [0x13] section '.relr.dyn'`

This is an environment-specific Linux AppImage bundling incompatibility in the current packaging toolchain, not a SpeakEx runtime failure and not evidence of a macOS or Windows build problem. The cross-platform workflow avoids this known packaging-only issue by validating `npm run tauri build -- --debug --no-bundle` instead of installer or AppImage bundling.

## Notes

SpeakEx is currently packaged and validated as a desktop MVP with a Linux-first release path and cross-platform no-bundle CI checks. Detailed transcription state, settings, and history stay on the local machine, while Gemini is the current transcription provider for completed recordings.
