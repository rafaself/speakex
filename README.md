# SpeakEx

Release 0.1 bootstrap for a local-first desktop transcription app built with Tauri v2, SvelteKit, and TypeScript.

## Scripts

- `npm run dev` — start the frontend dev server
- `npm run tauri dev` — run the desktop app in development
- `npm run check` — run the Svelte/TypeScript checks
- `npm run build` — build the frontend bundle
- `npm run tauri build` — build the desktop application

## Linux packaging (Release 1.7)

SpeakEx is currently configured for a Linux-first AppImage packaging path.

- Run the packaging build from the repo root with `toolbox run --container speakex-dev npm run tauri build`.
- If you are already inside that Toolbox environment, run `npm run tauri build`.
- When AppImage bundling succeeds, expect the final bundle under `src-tauri/target/release/bundle/appimage/`, with the release artifact appearing there as `SpeakEx_0.1.0_amd64.AppImage`.

### Current environment limitation

In this Fedora 43 packaging environment, the build reaches the AppImage bundling phase and stages `src-tauri/target/release/bundle/appimage/SpeakEx.AppDir`, but `linuxdeploy` then fails during its `strip` step on bundled libraries that contain `.relr.dyn` ELF sections:

- `unknown type [0x13] section '.relr.dyn'`

This is an environment-specific `linuxdeploy`/toolchain incompatibility, not a SpeakEx runtime failure.

## Notes

This scaffold keeps the native surface area minimal. The app currently exposes a single explicit Rust `ping` command so the frontend can verify the Tauri bridge before any transcription features are added.
