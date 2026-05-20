# Validation Guide

This document is the source of truth for repository validation commands and environment assumptions.

## Package Manager

SpeakEx currently uses `npm` as the canonical frontend package manager.

- Local install: `npm install`
- CI install: `npm ci`

Repository scripts, README examples, and the GitHub Actions workflow are aligned to `npm`.

## Frontend Validation

- Full test suite: `npm run test`
- Focused tests: `npm run test -- src/lib/features/history/controller.test.ts src/lib/features/logs/controller.test.ts src/lib/features/recording/controller.test.ts src/lib/features/settings/controller.test.ts src/integration/page.integration.test.ts`
- Type and Svelte checks: `npm run check`
- Frontend bundle build: `npm run build`

For structural frontend changes, prefer this order:

1. Focused tests for the touched feature slice.
2. `npm run check`.
3. `npm run build` when routes, components, or build-facing imports changed.

## Rust And Tauri Validation

- Format check: `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`
- Type and dependency check: `cargo check --manifest-path src-tauri/Cargo.toml`
- Rust tests: `cargo test --manifest-path src-tauri/Cargo.toml`
- Lint: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

For broader desktop validation, also run:

- `npm run tauri build -- --debug --no-bundle`

## Linux Notes

- On Linux hosts, `npm run tauri dev` and `npm run tauri build` go through `scripts/run-tauri.mjs`.
- That wrapper expects the `speakex-dev` Toolbox container unless it is already running inside a compatible container.
- The build wrapper sets `NO_STRIP=1` for Linux packaging builds to avoid the known AppImage strip failure on `.relr.dyn` libraries.
- On Fedora, the tray runtime dependency is `libayatana-appindicator-gtk3`.
- On Fedora Workstation with GNOME, the tray icon is not shown by default; users need the AppIndicator/KStatusNotifierItem shell extension (`gnome-shell-extension-appindicator`) for the SpeakEx tray to appear.

## Environment Limitations

Rust validation can fail in this environment when desktop system libraries required by Tauri are missing, especially WebKitGTK, GTK, or ALSA dependencies. If that happens:

1. Record the exact command that failed.
2. Record the missing system dependency reported by the toolchain.
3. Do not change unrelated code to work around the host environment.

## CI Surface

The repository includes `.github/workflows/cross-platform-validation.yml` for cross-platform no-bundle validation on Linux, macOS, and Windows. Keep contributor guidance aligned with that workflow when commands change.
