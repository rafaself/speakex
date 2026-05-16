# Tauri Backend Agent Guide

## Scope
- This file applies to everything under `src-tauri/`.
- Follow the root guide first, then these backend-specific rules.

## Backend Map
- `src/main.rs` stays as the thin desktop entrypoint.
- `src/lib.rs` should stay focused on app bootstrap, shared state wiring, and command registration.
- `src/` modules hold domain and platform logic such as recording, transcription, tray, shortcuts, persistence, and secrets.
- `migrations/` is the source of truth for SQLite schema changes.
- `capabilities/` and `tauri.conf.json` define Tauri capabilities and app configuration.

## Guardrails
- Preserve Tauri invoke names, serialized field names, settings store keys, and SQL schema behavior unless the task explicitly changes them.
- Keep platform wiring thin; move business logic into focused modules or services when extracting code.
- Do not mix structural refactors with plugin or OS-behavior changes unless required by the task.
- Add or edit migrations only when behavior or persisted data actually changes.

## Validation
- Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` after Rust edits.
- Run `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo test --manifest-path src-tauri/Cargo.toml` for the touched slice.
- Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` before closing a broader backend refactor.

## Environment Notes
- Rust validation can fail on hosts that do not provide the desktop libraries required by Tauri, WebKitGTK, GTK, or ALSA. If that happens, capture the failure clearly instead of changing code blindly.