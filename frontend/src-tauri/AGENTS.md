# Tauri Core Agent Instructions

This tree contains the supported native backend for Meetily. It owns Tauri commands/events, audio capture, transcription, local storage, summaries, notifications, updates, and platform integration.

## Routing

| Task | Start here |
| --- | --- |
| Command registration, app state, plugins, shutdown | `src/lib.rs`, `src/state.rs`, `src/main.rs`, `src/tray.rs` |
| Audio device discovery and permissions | `src/audio/devices/`, `src/audio/device_detection.rs`, `src/audio/permissions.rs`, `src/audio/system_detector.rs` |
| Audio capture and processing | `src/audio/capture/`, `src/audio/stream.rs`, `src/audio/system_audio_stream.rs`, `src/audio/pipeline.rs`, `src/audio/vad.rs` |
| New audio backend work | `src/audio_v2/`, `src/audio/recording_preferences.rs`, `src/audio/capture/backend_config.rs` |
| Recording lifecycle | `src/audio/recording_commands.rs`, `src/audio/recording_manager.rs`, `src/audio/recording_state.rs`, `src/audio/recording_saver.rs`, `src/audio/incremental_saver.rs` |
| Audio import | `src/audio/import.rs`, frontend callers under `../src/components/ImportAudio/` |
| Retranscription and recovery | `src/audio/retranscription.rs`, `src/audio/transcription/`, frontend callers under `../src/components/MeetingDetails/` |
| Whisper models and GPU acceleration | `src/whisper_engine/`, `Cargo.toml`, `../scripts/auto-detect-gpu.js`, `../../docs/GPU_ACCELERATION.md` |
| Parakeet models | `src/parakeet_engine/`, `src/audio/transcription/parakeet_provider.rs` |
| Local database and repositories | `src/database/`, especially `src/database/repositories/` and `src/database/setup.rs` |
| API-style app commands | `src/api/api.rs`, `src/api/commands.rs` |
| Summaries, templates, local sidecar | `src/summary/`, `src/summary/summary_engine/`, `templates/`, `../../llama-helper/` |
| Provider integrations | `src/ollama/`, `src/openai/`, `src/anthropic/`, `src/groq/`, `src/openrouter/` |
| Notifications and DND | `src/notifications/`, `NOTIFICATION_TESTING.md` |
| Analytics | `src/analytics/`, `../src/components/Analytics*` |
| Console and system utilities | `src/console_utils/`, `src/utils.rs` |
| Legacy comparison only | `src/lib_old_complex.rs`, `src/audio/core-old.rs`, `src/audio/recording_saver_old.rs` |

## Conventions

- Current supported behavior belongs here, not in the archived Python backend.
- Add frontend-facing behavior through Tauri commands/events and register commands in `src/lib.rs`.
- Keep command names stable when frontend code already invokes them.
- Use Tauri path APIs and app handles for filesystem and shell integration.
- Preserve platform guards for macOS, Windows, and Linux behavior.
- Treat audio and permission changes as runtime-sensitive; compile success is not enough.
- Use existing `anyhow::Result`, structured command errors, logging, and state-management patterns.
- Avoid editing `lib_old_complex.rs` or `*-old.rs` unless explicitly working on legacy cleanup.

## Verification

- Run `cargo check` from this directory for Rust changes.
- Run `pnpm run lint` from `frontend/` when frontend IPC callers changed.
- Use `pnpm run tauri:dev` or platform helper scripts for native audio, permission, recording, import, retranscription, model download, notification, or tray behavior.
