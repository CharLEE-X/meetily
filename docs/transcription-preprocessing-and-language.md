# Transcription Preprocessing and Language Handling

This document records the CHA-1677 accuracy work on audio preprocessing presets, language handling, and timestamp continuity validation.

## Current Native Pipeline

Meetily already applies several preprocessing steps before local transcription:

- Device audio is converted to mono.
- Non-48 kHz capture devices are resampled through a persistent buffered resampler to avoid Bluetooth and variable chunk-size drift.
- Microphone audio runs through an 80 Hz high-pass filter to reduce low-frequency rumble.
- Microphone audio is normalized with EBU R128 loudness normalization.
- Mixed audio is segmented by Silero VAD at 16 kHz with pause bridging so natural pauses do not split every utterance.
- Whisper receives 16 kHz speech segments and applies the configured language preference.
- Parakeet currently uses automatic language detection only.

## User Presets

The transcription settings page now exposes three preprocessing presets:

| Preset | Purpose | Default behavior |
| --- | --- | --- |
| Balanced clarity | Normal meeting capture with stable timestamps and default filtering. | This is the default and preserves existing recording behavior. |
| Noisy room | Provides setup guidance for noisy laptop or office recordings without changing native filtering. | Keeps native preprocessing stable and recommends Whisper language pinning when possible. |
| Raw capture | Diagnostic mode for before/after benchmark runs. | Keeps the capture path unchanged while documenting the intended validation workflow. |

The presets are stored locally under `transcriptionPreprocessingPreset`. They do not migrate or rewrite existing meetings.

## Language Handling

Whisper supports:

- `auto`: detect the source language and keep the transcript in that language.
- `auto-translate`: detect the source language and translate transcript output to English.
- ISO language codes such as `en`, `es`, `pl`, or `de` for stronger recognition on known-language meetings.

Parakeet is kept to automatic detection in the UI because the current Parakeet path does not support manual language selection. The transcription settings page now surfaces this limitation next to the profile and preprocessing choices.

## Benchmark Expectations

Use `docs/transcription-benchmark-fixtures.json` for before/after comparisons. Record these metrics for each fixture:

- `word_error_rate`
- `character_error_rate`
- `realtime_factor`
- `average_confidence`
- `timestamp_drift_ms`
- `failure_mode`

Expected stability improvement for this slice is operational rather than model-weight based:

- Users can explicitly choose a preset before recording, reducing accidental use of auto language detection for known-language Whisper meetings.
- Parakeet users see the auto-detection limitation before recording.
- Timestamp validation remains anchored to VAD segment start/end values and the persistent resampler path; no timestamp arithmetic changed in this slice.

## Timestamp Continuity Checklist

For long recordings, verify:

- VAD segment start timestamps are monotonically increasing.
- Segment end timestamps are greater than or equal to start timestamps.
- Flush-generated final segments preserve the same 16 kHz timestamp basis.
- Retranscription/import still writes transcript rows in chronological order.
- No preset selection changes saved transcript timestamps for existing meetings.
