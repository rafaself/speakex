# SpeakEx

SpeakEx is a local-first desktop transcription app built with Tauri v2, SvelteKit, TypeScript, and Rust.

The current MVP lets you record audio from the main window, tray, or global shortcut, send completed recordings to Gemini for transcription, automatically copy results to the clipboard, and keep a local transcription history on your machine.

## MVP highlights

- Local-first desktop workflow with no separate backend service
- Manual recording controls in the app, tray, and through a global shortcut
- Gemini-powered transcription for completed recordings
- Local history, clipboard copy, and settings for retention behavior
- Privacy-aware hidden-window notifications for transcription outcomes

## Repository docs

- `docs/architecture/project-structure.md` — repository map and directory responsibilities
- `docs/architecture/frontend.md` — frontend boundaries and refactor direction
- `docs/architecture/tauri-backend.md` — backend boundaries and refactor direction
- `docs/contributing/validation.md` — validation commands and environment notes
- `AGENTS.md` — short operational map for agents and contributors

## Development

- `npm install` — install frontend dependencies
- `npm run dev` — start the frontend dev server
- `npm run tauri dev` — run the desktop app in development; on Linux hosts this automatically uses the `speakex-dev` Toolbox container

## Scripts

- `npm run dev` — start the frontend dev server
- `npm run tauri dev` — run the desktop app in development; on Linux hosts this automatically uses the `speakex-dev` Toolbox container
- `npm run test` — run the unit and integration test suite
- `npm run coverage` — run the test suite with coverage reporting
- `npm run check` — run the Svelte/TypeScript checks
- `npm run build` — build the frontend bundle
- `npm run tauri build` — build the desktop application; on Linux hosts this automatically uses the `speakex-dev` Toolbox container

## Cross-platform validation

GitHub Actions validates the desktop build surface on Linux, macOS, and Windows through `.github/workflows/cross-platform-validation.yml`.

Use `docs/contributing/validation.md` as the source of truth for local and CI validation commands. Keep README brief and operational.

## Linux packaging

SpeakEx is currently configured for a Linux-first AppImage packaging path. The operational details for Toolbox, packaging validation, and environment prerequisites live in `docs/contributing/validation.md`.

## Notes

SpeakEx is currently packaged and validated as a desktop MVP with a Linux-first release path and cross-platform no-bundle CI checks. Detailed transcription state, settings, and history stay on the local machine, while Gemini is the current transcription provider for completed recordings.
