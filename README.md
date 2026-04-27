# SpeakEx

Release 0.1 bootstrap for a local-first desktop transcription app built with Tauri v2, SvelteKit, and TypeScript.

## Scripts

- `npm run dev` — start the frontend dev server
- `npm run tauri dev` — run the desktop app in development
- `npm run check` — run the Svelte/TypeScript checks
- `npm run build` — build the frontend bundle
- `npm run tauri build` — build the desktop application

## Notes

This scaffold keeps the native surface area minimal. The app currently exposes a single explicit Rust `ping` command so the frontend can verify the Tauri bridge before any transcription features are added.
