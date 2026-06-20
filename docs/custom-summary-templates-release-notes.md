# Custom Summary Templates Release Notes

This release adds custom summary templates for local, reusable meeting summary
formats.

## What changed

- Added **Settings → Templates** for custom template creation, editing,
  validation, preview, import/export, deletion, and restoring editable built-in
  copies.
- Added built-in template registry coverage for standups, standard meetings,
  project syncs, retrospectives, psychiatric sessions, and sales/client calls.
- Summary generation validates the selected template before model invocation.
- Completed summaries record template ID, schema version, display name, and a
  template fingerprint in local summary metadata.
- Meeting details remember the selected template per meeting.

## Privacy and storage

Templates are local files in the Meetily app data directory. Import/export works
with template JSON only; it does not export transcripts, summaries, screenshots,
calendar metadata, notes, reminders, or agent prompts.

## QA notes

- `cargo test --manifest-path frontend/src-tauri/Cargo.toml summary::templates --lib`
- `cargo test --manifest-path frontend/src-tauri/Cargo.toml summary::service --lib`
- `cargo check --manifest-path frontend/src-tauri/Cargo.toml`
- `pnpm run build`

`pnpm run lint` currently prompts to configure Next ESLint interactively in this
checkout, so release validation used the production Next build for frontend type
and compile coverage.
