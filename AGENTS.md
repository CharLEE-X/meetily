# Agent Instructions

This repository is Meetily, a privacy-first AI meeting assistant. The supported product is the Tauri desktop app: Next.js/React/TypeScript in `frontend/src` plus Rust/Tauri in `frontend/src-tauri`. The old Python/FastAPI and Docker backend under `backend/` is archived legacy context only.

This file is also available as `CLAUDE.md` via symlink for tools that expect that name.

## Knowledge Base and AI Routing

Start with the narrowest relevant context before deep work:

| Task | Start here |
| --- | --- |
| Product overview, setup, supported architecture | `README.md`, `frontend/README.md`, `docs/architecture.md` |
| Root development commands, build scripts, contribution flow | `CONTRIBUTING.md`, this file |
| Frontend app routing, UI, hooks, services | `frontend/AGENTS.md`, then `frontend/src/app/`, `frontend/src/components/`, `frontend/src/hooks/`, `frontend/src/services/` |
| Tauri command/event registration and native app behavior | `frontend/src-tauri/AGENTS.md`, then `frontend/src-tauri/src/lib.rs` |
| Audio capture, mixing, VAD, import, retranscription | `frontend/src-tauri/AGENTS.md`, then `frontend/src-tauri/src/audio/`, `frontend/src-tauri/src/audio_v2/` |
| Meeting persistence and SQLite repositories | `frontend/src-tauri/AGENTS.md`, then `frontend/src-tauri/src/database/` and `frontend/src/services/` |
| Summary templates, local sidecar model, LLM providers | `frontend/src-tauri/AGENTS.md`, then `frontend/src-tauri/src/summary/`, `frontend/src-tauri/templates/`, provider modules under `frontend/src-tauri/src/{ollama,anthropic,groq,openai,openrouter}/` |
| Whisper, Parakeet, GPU acceleration, model downloads | `docs/GPU_ACCELERATION.md`, `frontend/src-tauri/AGENTS.md`, `frontend/src-tauri/src/whisper_engine/`, `frontend/src-tauri/src/parakeet_engine/` |
| Onboarding, permissions, first-run setup | `frontend/AGENTS.md`, then `frontend/src/components/onboarding/`, `frontend/src/contexts/OnboardingContext.tsx`, `frontend/src-tauri/src/onboarding.rs` |
| Notifications and DND behavior | `frontend/src-tauri/AGENTS.md`, `frontend/src-tauri/NOTIFICATION_TESTING.md`, `frontend/src-tauri/src/notifications/` |
| Analytics and privacy | `PRIVACY_POLICY.md`, `frontend/src-tauri/src/analytics/`, `frontend/src/components/Analytics*` |
| Build, packaging, CI, platform setup | `docs/BUILDING.md`, `docs/building_in_linux.md`, `.github/workflows/*.md`, `frontend/scripts/`, `frontend/src-tauri/Cargo.toml` |
| Legacy migration reference only | `backend/AGENTS.md`, `backend/README.md`, then files under `backend/` if needed |
| Llama sidecar helper | `llama-helper/AGENTS.md`, `llama-helper/Cargo.toml`, `llama-helper/llama-helper/src/main.rs` |
| Code review or wrap-up workflow | `.agents/agents/review.md`, `.agents/commands/review-and-fix.md`, `.agents/commands/fix-check-commit-close.md` |

Treat `backend/` as historical material. Do not add new supported behavior there unless the task explicitly asks for legacy archive changes.

**Agent workflow routing:** `.agents/` is the canonical repo-local agent workflow tree. `.agents/plugins/marketplace.json` exposes the local bridge, and `plugins/meetily-codex-bridge/` contains Codex-facing command wrappers. Do not use `.cursor/` for Meetily routing.

## Project Overview

Meetily captures, transcribes, and summarizes meetings primarily on local infrastructure.

- Desktop app: Tauri 2.x, Rust, Next.js 14, React 18, TypeScript.
- Audio: Rust capture and processing with microphone and system audio paths.
- Transcription: local Whisper/whisper-rs and Parakeet flows in the Tauri app.
- LLM integrations: local Ollama plus Claude, Groq, OpenRouter, and OpenAI provider paths.
- Persistence: local app storage and SQLite through Rust/Tauri services.

The current app does not require a separate FastAPI tier. Meeting persistence, local transcription, and summary orchestration are handled through the desktop app.

## Essential Commands

Run these from `frontend/` unless noted:

```bash
pnpm install
pnpm run dev              # Next.js dev server on port 3118
pnpm run tauri:dev        # Full Tauri development mode
pnpm run tauri:build      # Production Tauri build
pnpm run lint             # Next lint
```

Platform helper scripts:

```bash
./clean_run.sh            # macOS clean dev run
./clean_run.sh debug      # macOS dev run with debug logging
./clean_build.sh          # macOS production build
clean_run_windows.bat     # Windows dev run
clean_build_windows.bat   # Windows production build
```

GPU-specific Tauri scripts exist for `cpu`, `cuda`, `vulkan`, `metal`, `coreml`, `openblas`, and `hipblas` variants. Prefer the auto-detecting `pnpm run tauri:dev` / `pnpm run tauri:build` unless validating a specific backend.

## Architecture Notes

Frontend-to-Rust calls use Tauri commands:

```typescript
await invoke("start_recording", {
  mic_device_name: "Built-in Microphone",
  system_device_name: "BlackHole 2ch",
  meeting_name: "Team Standup",
});
```

Rust-to-frontend updates use Tauri events, for example transcript updates emitted from Rust and listened to in React state/hooks.

The audio system has parallel concerns:

- Recording path: mixed audio suitable for saved meeting playback.
- Transcription path: speech-focused chunks, including VAD filtering, suitable for local transcription.

When working on audio:

- Device detection or platform quirks: `frontend/src-tauri/src/audio/devices/`, `device_detection.rs`, `hardware_detector.rs`, `system_detector.rs`, and platform-specific capture modules.
- Capture streams: `frontend/src-tauri/src/audio/capture/`, `stream.rs`, `system_audio_stream.rs`, `audio_v2/stream.rs`.
- Mixing, synchronization, VAD, and processing: `frontend/src-tauri/src/audio/pipeline.rs`, `frontend/src-tauri/src/audio/transcription/`, `audio_v2/`.
- Recording workflow: `frontend/src-tauri/src/audio/recording_manager.rs`, `recording_commands.rs`, `recording_saver.rs`, `incremental_saver.rs`.
- Import and retranscription: `frontend/src-tauri/src/audio/import.rs`, `frontend/src-tauri/src/audio/retranscription.rs`, with UI callers under `frontend/src/components/ImportAudio/` and `frontend/src/components/MeetingDetails/`.

## Development Conventions

- Follow the nearest existing pattern before introducing a new abstraction.
- Keep changes scoped to the request. Do not reformat or refactor unrelated code.
- Prefer user-friendly frontend error messages; do not expose raw provider, backend, or diagnostic strings to users.
- Add or update docs when commands, environment requirements, public APIs, Tauri commands, or package structure change.
- Use Tauri path APIs for cross-platform filesystem locations; do not hardcode user paths.
- Request and handle permissions explicitly, especially microphone and macOS screen recording permissions.
- Keep hot-path Rust logging low overhead. Use existing performance logging conventions where present.
- For shared async Rust state, follow existing `Arc`, lock, and atomic patterns in nearby modules.
- Frontend changes should respect the current component system, Tailwind usage, Radix/lucide patterns, and existing context/hooks.

## Supported vs Legacy Backend

Current supported behavior belongs in:

- `frontend/src` for UI and TypeScript services.
- `frontend/src-tauri/src` for native commands, audio, transcription, storage, notifications, and summaries.

Do not add new endpoints to `backend/app/main.py` or reintroduce Docker/FastAPI/standalone whisper-server as a supported requirement. The archived backend had development-oriented unauthenticated CORS behavior; treat that as obsolete legacy context, not a production security model.

## Testing and Verification

Choose the smallest verification that exercises the changed behavior:

- Frontend-only changes: `pnpm run lint` and targeted manual UI checks if relevant.
- Rust/Tauri changes: `cargo check` from `frontend/src-tauri` plus targeted app run or tests when available.
- Build or packaging changes: `pnpm run tauri:build` or the relevant platform helper script when feasible.
- Audio or permissions changes: run the app and verify the affected platform flow, since compile-only checks miss device and permission behavior.

Before finishing substantive work, use the repo-local wrap-up routing:

- Review only: `.agents/agents/review.md`
- Review and fix: `.agents/commands/review-and-fix.md`
- Review, fix, check, and commit: `.agents/commands/fix-check-commit-close.md`

## Key Files

- `frontend/src-tauri/src/lib.rs`: Tauri entry point and command registration.
- `frontend/src-tauri/src/audio/mod.rs`: audio module exports.
- `frontend/src-tauri/src/audio/pipeline.rs`: audio mixing, synchronization, and VAD.
- `frontend/src-tauri/src/audio/transcription/`: transcription providers and worker.
- `frontend/src-tauri/src/audio/recording_manager.rs`: recording orchestration.
- `frontend/src-tauri/src/audio/recording_saver.rs`: audio file writing.
- `frontend/src-tauri/src/database/repositories/`: SQLite persistence access.
- `frontend/src-tauri/src/whisper_engine/whisper_engine.rs`: Whisper model loading and transcription.
- `frontend/src-tauri/src/parakeet_engine/`: Parakeet model loading and transcription.
- `frontend/src-tauri/src/summary/summary_engine/`: local summary sidecar and built-in AI model management.
- `frontend/src/app/page.tsx`: main app surface.
- `frontend/src/app/meeting-details/`: meeting detail route.
- `frontend/src/components/`: UI components.
- `frontend/src/contexts/`: shared React state.
- `frontend/src/services/`: frontend service wrappers.

## Learned User Preferences

- Keep summaries concise.
- Working language is English.
- When unsure about product details, pick a sensible default that is reliable and explain the tradeoff briefly rather than blocking.
- Avoid touching unrelated user changes in the working tree.
