# SpeakEx Agent Guide

## Guardrails
- Ask for confirmation before adding new production dependencies.
- Avoid implementations that can introduce security vulnerabilities. If a risky change is genuinely necessary, stop and warn before proceeding.
- Preserve public behavior, exported names, serialized shapes, settings keys, and Tauri command names unless the task explicitly changes them.

## Canonical Tooling
- Package manager: `npm`.
- Frontend validation: `npm run test`, `npm run check`, `npm run build`.
- Rust/Tauri validation: `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- On Linux, `npm run tauri dev` and `npm run tauri build` go through `scripts/run-tauri.mjs`, which may use the `speakex-dev` Toolbox container automatically.

## Project Map
- Product overview and quick start stay in `README.md`.
- Architecture and repository structure live under `docs/`.
- Frontend code lives under `src/`; local instructions are in `src/AGENTS.md`.
- Rust/Tauri code lives under `src-tauri/`; local instructions are in `src-tauri/AGENTS.md`.
- Utility wrappers and developer scripts live under `scripts/`.

## Source Of Truth
- Use `docs/architecture/project-structure.md` for the repository map.
- Use `docs/architecture/frontend.md` for frontend boundaries.
- Use `docs/architecture/tauri-backend.md` for backend boundaries.
- Use `docs/contributing/validation.md` for validation and environment prerequisites.

## Working Style
- Prefer the smallest cohesive change that fits the current monorepo boundaries.
- Use TDD when it is convenient and the touched slice already has a test seam.
- If behavior, contracts, or architecture change, update the relevant document in `docs/` in the same task when practical.
- Do not keep legacy code or backward-compatibility shims unless the task requires them.

## Overrides
- More specific `AGENTS.md` files inside subdirectories override this file for their subtree.
