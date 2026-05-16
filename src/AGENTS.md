# Frontend Agent Guide

## Scope
- This file applies to everything under `src/`.
- Follow the root guide first, then these frontend-specific rules.

## Frontend Map
- `routes/` contains SvelteKit route entrypoints and route-local composition.
- `lib/features/` contains feature logic such as controllers, presenters, status mapping, and feature tests.
- `lib/components/` contains UI components. Keep domain decisions out of presentational components when possible.
- `lib/native/` is the frontend adapter boundary for Tauri invokes and native integrations.
- `lib/stores/`, `lib/settings/`, and `lib/types/` hold shared frontend state, settings schema, and UI-facing types.
- `integration/` holds UI integration tests that exercise the route surface.

## Guardrails
- Prefer route files that compose existing feature modules instead of accumulating more orchestration logic.
- Do not call `@tauri-apps/api` directly from random components when a `lib/native/` adapter already exists or should exist.
- Keep controller and presenter APIs stable unless the task explicitly changes the contract.
- Keep tests close to the feature they validate whenever practical.

## Validation
- Use focused Vitest runs for touched slices before broader checks.
- Run `npm run check` after frontend structural changes.
- Run `npm run build` when route structure, Svelte components, or build-facing imports change.

## Current Hotspots
- `routes/+page.svelte` currently owns too much page orchestration. When touching that flow, prefer extracting composition helpers or page-level modules instead of adding more logic there.