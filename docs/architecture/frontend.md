# Frontend Architecture

SpeakEx uses SvelteKit for the desktop UI shell. The frontend is intentionally thin on native concerns and should talk to the desktop runtime through adapter modules instead of raw Tauri calls scattered across the UI.

## Boundaries

- `src/routes/` holds route entrypoints.
- `src/lib/features/` holds feature orchestration, controllers, presenters, page helpers, and feature-level tests.
- `src/lib/components/` holds reusable UI sections and presentational components.
- `src/lib/native/` holds Tauri-facing adapters such as recording, transcription, history, settings, logs, shortcuts, and secrets.
- `src/lib/stores/` and `src/lib/settings/` hold shared UI state and settings normalization.

## Rules Of Thumb

- Prefer route files that compose feature modules over route files that grow into application controllers.
- Keep Tauri `invoke` usage behind `src/lib/native/` so the UI surface can evolve without broad call-site churn.
- Put formatting, presenter, and mapping logic in feature modules instead of embedding it in Svelte markup.
- Keep shared stores small and explicit. Do not turn them into a catch-all for every page concern.

## Tests

- Keep unit tests close to the feature module they validate.
- Use `src/integration/` for route-level integration tests that exercise the composed UI surface.
- When structural refactors touch feature composition, validate the relevant controller tests first, then the route integration test, then `npm run check`.

## Current Refactor Target

`src/routes/+page.svelte` still coordinates settings, recording, history, logs, and Tauri window chrome. The first extraction step already moved pure page helpers into `src/lib/features/home/`. Future structural work should continue reducing route-local orchestration there while preserving the public UI behavior.