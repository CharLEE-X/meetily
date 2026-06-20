# Transcription Accuracy Release Notes

## Summary

This upgrade adds visible local transcription quality controls without changing the default recording path for existing users.

## What Changed

- Added fast, balanced, and high-accuracy transcription profiles in settings.
- Preserved the existing Parakeet int8 default for new and existing installs.
- Added readiness badges for profile models so users can see whether a model is ready, missing, downloading, or needs attention.
- Added preprocessing presets that document the native capture path and guide noisy-room or diagnostic QA runs.
- Documented Whisper language pinning, Whisper auto-translate, and Parakeet automatic-language limitations.
- Added a benchmark fixture manifest for repeatable quality, runtime, memory, and timestamp checks.

## Migration Behavior

- Existing recordings and transcripts are not migrated or rewritten.
- Existing transcript configuration remains valid.
- If no transcript config exists, the app continues to default to `parakeet-tdt-0.6b-v3-int8`.
- Selecting a higher-accuracy Whisper profile may require downloading the selected model before recording.
- Preprocessing preset selection is local app preference metadata and does not change saved meeting timestamps.

## QA Evidence

Targeted checks run during the epic:

- `node -e "JSON.parse(require('fs').readFileSync('docs/transcription-benchmark-fixtures.json','utf8')); console.log('fixture manifest ok')"`
- `node --test --experimental-strip-types src/lib/transcriptionProfiles.test.ts`
- `pnpm run test:transcription-preprocessing`
- Scoped `git diff --check` runs for each committed slice.
- Scoped Claude reviews for baseline docs, quality profile readiness, preprocessing preset wording, and defaults.

Known repo-level check limits:

- `pnpm run lint` currently opens Next's interactive ESLint setup prompt.
- `pnpm exec tsc --noEmit --pretty false` still reports pre-existing test configuration/type issues outside the transcription accuracy work.

## Troubleshooting

- If a profile says `Download required`, select the profile and download the model in the model manager before recording.
- If Whisper is slow, use `Balanced accuracy` before `Highest accuracy`; `large-v3` has the largest download and slowest runtime.
- If a known-language Whisper meeting has poor recognition, pin the language instead of using `Auto Detect`.
- If using Parakeet, keep language selection on automatic detection; switch to Whisper when manual language control is required.
- For noisy-room tests, compare results against `docs/transcription-benchmark-fixtures.json` before changing native pipeline thresholds.

## Related Docs

- `docs/transcription-accuracy-baseline.md`
- `docs/transcription-benchmark-fixtures.json`
- `docs/transcription-preprocessing-and-language.md`
