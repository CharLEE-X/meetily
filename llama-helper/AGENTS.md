# Llama Helper Agent Instructions

This workspace contains the Rust sidecar helper used by Meetily's local summary model flow.

## Routing

- Main sidecar entry: `llama-helper/llama-helper/src/main.rs`
- Workspace manifest: `Cargo.toml`
- Tauri summary integration: `../frontend/src-tauri/src/summary/summary_engine/`

## Conventions

- Keep the sidecar contract aligned with `frontend/src-tauri/src/summary/summary_engine/sidecar.rs`.
- Verify sidecar changes together with the Tauri summary flow.
- Do not change packaging assumptions without checking `frontend/src-tauri/tauri.conf.json` and frontend build scripts.
