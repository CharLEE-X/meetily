# Speaker Identification and Screenshots Release Notes

This release adds the first local speaker-label and optional screenshot workflow for CHA-1667.

## Shipped Behavior

* Speaker labeling runs locally from existing transcript timing and source metadata. It does not send audio, transcripts, screenshots, or labels to an external service.
* Meeting details now includes a speaker panel with a Detect action. Detected labels appear inline in the transcript.
* Renaming a speaker label marks it as a user-confirmed manual correction and records a correction audit row.
* Periodic screenshots are disabled by default. Users must enable Meeting Screenshots in Settings and confirm screenshot capture for each meeting before the first capture.
* Screenshot capture is macOS-only in this release and uses the system `screencapture` command.
* Screenshots are stored under the app-managed data directory and attached to the saved meeting after recording is persisted.
* Meeting details shows a screenshot timeline. Users can delete individual screenshots, which marks metadata deleted and removes the local image file.

## Privacy Notes

* Screenshot capture can include unrelated windows, notifications, private documents, browser tabs, or credentials visible on screen.
* Speaker labels are labels, not verified identities. The app does not infer a person's real identity from screenshots.
* Screenshots and speaker labels are not exposed to MCP, cloud providers, chat indexes, or exports unless a future feature adds a separate explicit inclusion control.
* Disabling the global screenshot setting prevents future meeting screenshot prompts and capture.

## QA Evidence

Commands run for this branch:

```bash
cargo test --manifest-path frontend/src-tauri/Cargo.toml speaker::tests
cargo test --manifest-path frontend/src-tauri/Cargo.toml screenshots::tests
cargo check --manifest-path frontend/src-tauri/Cargo.toml
pnpm run build
```

Known repository-level verification notes:

* `pnpm run lint` still invokes the old interactive `next lint` setup on this branch instead of running a configured linter.
* `pnpm exec tsc --noEmit` is blocked by an existing `tests/lib/blocknote-markdown.test.ts` dependency on `bun:test` types. `pnpm run build` completes successfully.

## Current Limits

* The first speaker labeling pass is heuristic and dependency-free. It groups by transcript source and timing, not by a dedicated diarization model.
* Screenshot capture is implemented for macOS first.
* Pause/resume coupling uses recording stop/start lifecycle today; finer screenshot pause/resume controls should remain a follow-up before a broader release.
* Speaker labels are displayed in summaries/exports only after those downstream surfaces add explicit inclusion controls.
