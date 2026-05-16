# Tauri Backend Architecture

SpeakEx uses Tauri v2 with a Rust backend that owns recording, transcription, persistence, tray behavior, notifications, shortcuts, and secret storage. The backend is a local native core, not a network service.

## Runtime Shape

- `src-tauri/src/main.rs` starts the desktop application.
- `src-tauri/src/lib.rs` bootstraps the Tauri app, manages shared state, registers plugins, and wires grouped command handlers.
- `src-tauri/src/commands/` groups Tauri commands by domain while preserving the same invoke names used by the frontend.
- Rust modules under `src-tauri/src/` implement the actual domain and platform behavior.

## Main Backend Responsibilities

- `history_database.rs` and `history_repository.rs` own SQLite access.
- `manual_flow.rs` coordinates completed-recording transcription side effects.
- `recorder.rs` owns audio capture and recording lifecycle.
- `transcription.rs` owns provider abstractions and the Gemini implementation.
- `shortcut.rs`, `tray.rs`, and `notifications.rs` own desktop integrations.
- `secret_store.rs` owns OS-backed secret persistence.

## Stable Contracts

The following interfaces should remain stable unless a task explicitly changes them:

- Tauri command names invoked by the frontend
- Serialized field names returned to TypeScript
- Settings store keys in `settings.json`
- Existing SQLite schema behavior and migration ordering

## Refactor Direction

The command surface is now split by domain under `src-tauri/src/commands/`. The next structural backend step is to keep `src-tauri/src/lib.rs` as a thin facade and then decompose larger internals such as `recorder.rs` and `transcription.rs` behind the same public API.

## Validation

- `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

Desktop-library prerequisites can block Rust validation on some Linux hosts. Capture those failures as environment evidence instead of compensating with unrelated code changes.