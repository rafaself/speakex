# Speakex Agent Guide

## Non-negotiable
- Ask for confirmation before adding new production dependencies.
- Avoid implementations or changes that can introduce security vulnerabilities. If a risky change is genuinely necessary, stop and warn the user before proceeding.

## Working Style
- When convenient, use TDD: start from a failing or missing test, implement the change, then refactor.
- Keep changes aligned with the current product, architecture, and API documents under `docs/`.
- Prefer the smallest change that fits the existing monorepo boundaries and shared package structure.
- If behavior, contracts, or architecture change, update the relevant documentation in the same task when practical.
- Never consider backward compatibilities neither keep legacy code or dead code.

## Overrides
- More specific `AGENTS.md` files inside subdirectories override this file for their subtree.
