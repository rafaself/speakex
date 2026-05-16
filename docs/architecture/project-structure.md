# Project Structure

This document is the repository map for SpeakEx. It explains where code belongs today and which directories are the source of truth for architecture, UI, infrastructure, tests, and tooling.

## Top-Level Layout

```text
.
├─ README.md                     Product overview and quick start
├─ AGENTS.md                     Repository-wide agent instructions
├─ docs/                         Architecture and contributor documentation
├─ scripts/                      Developer and build wrappers
├─ src/                          SvelteKit frontend
├─ src-tauri/                    Rust and Tauri desktop backend
├─ static/                       Static web assets
├─ build/                        Generated frontend build output
├─ .svelte-kit/                  Generated SvelteKit output
├─ node_modules/                 Installed frontend dependencies
└─ plan.md                       Historical implementation plan
```

## Frontend Layout

```text
src/
├─ routes/                       Route entrypoints
├─ integration/                  UI integration tests
└─ lib/
   ├─ components/               UI components and section views
   ├─ features/                 Feature logic, controllers, presenters, page helpers, tests
   ├─ native/                   Frontend adapters for Tauri/native commands
   ├─ settings/                 Settings schema and normalization
   ├─ stores/                   Shared frontend state
   └─ types/                    UI-facing shared types
```

- Keep route files focused on route composition.
- Put domain logic under `src/lib/features/`.
- Keep Tauri invoke wrappers behind `src/lib/native/` so the UI has a stable adapter boundary.

## Backend Layout

```text
src-tauri/
├─ src/                          Rust app bootstrap and backend modules
│  └─ commands/                  Grouped Tauri command handlers by domain
├─ migrations/                   SQLite schema migrations
├─ capabilities/                 Tauri capability manifests
├─ icons/                        Desktop app icons
├─ tauri.conf.json               Tauri application configuration
└─ Cargo.toml                    Rust crate and dependency configuration
```

- `src/main.rs` is the desktop entrypoint.
- `src/lib.rs` wires the Tauri app and registered commands.
- Rust modules under `src/` own recording, transcription, tray behavior, shortcuts, notifications, secret storage, and persistence.

## Generated Artifacts

These directories are build outputs, not source-of-truth code:

- `build/`
- `.svelte-kit/`
- `node_modules/`
- `src-tauri/target/`

Do not treat generated output as implementation surface during refactors.

## Current Structural Priorities

The current codebase is functional, but two files are the main orchestration hotspots:

- `src/routes/+page.svelte`
- `src-tauri/src/lib.rs` before command registration is fully reduced to a thin facade

When refactoring structure, reduce those files first before splitting medium-sized files that are still reasonably cohesive.