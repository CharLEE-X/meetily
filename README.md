<div align="center" style="border-bottom: none">
    <h1>
        RecallX
        <br>
        Private Meeting Memory with AI
    </h1>
    <a href="https://github.com/CharLEE-X/meetily/releases/latest"><img src="https://img.shields.io/badge/Latest_Release-Download-brightgreen" alt="Latest Release"></a>
    <a href="https://github.com/CharLEE-X/meetily"><img alt="GitHub Repo stars" src="https://img.shields.io/github/stars/CharLEE-X/meetily?style=flat"></a>
    <a href="https://github.com/CharLEE-X/meetily/releases"><img alt="GitHub Downloads (all assets, all releases)" src="https://img.shields.io/github/downloads/CharLEE-X/meetily/total?style=plastic"></a>
    <a href="https://github.com/CharLEE-X/meetily/blob/main/LICENSE.md"><img src="https://img.shields.io/badge/License-MIT-blue" alt="License"></a>
    <a href="https://github.com/CharLEE-X/meetily/releases"><img src="https://img.shields.io/badge/Supported_OS-macOS,_Windows-white" alt="Supported OS"></a>
    <br>
    <h3>Local-first • Privacy-first • Agent-ready</h3>
    <p align="center">
        RecallX captures, transcribes, summarizes, and recalls meetings on local-first infrastructure.
        It is built for people and teams who need searchable meeting memory, reviewable follow-ups,
        and AI-assisted context without silent cloud capture.
    </p>
</div>

---

RecallX is a fork of Meetily and preserves the original MIT license attribution in
[`LICENSE.md`](LICENSE.md). See [`NOTICE.md`](NOTICE.md) for attribution details.

---

<details>
<summary>Table of Contents</summary>

- [Introduction](#introduction)
- [Why RecallX?](#why-recallx)
- [Features](#features)
- [Installation](#installation)
- [Key Features in Action](#key-features-in-action)
- [System Architecture](#system-architecture)
- [For Developers](#for-developers)
- [Community Roadmap](#community-roadmap)
- [Contributing](#contributing)
- [License](#license)

</details>

## Introduction

RecallX is a private AI memory layer for every meeting. It captures live audio,
creates local transcripts, generates summaries, and keeps meeting context
searchable so decisions, action items, screenshots, exports, and agent handoffs
stay attached to one local meeting record.

The product is designed around explicit consent. Recording, screenshots,
external exports, Apple automation, local MCP access, and post-meeting agent
workflows stay reviewable and auditable.

## Why RecallX?

Cloud meeting assistants are convenient, but they often move sensitive meeting
content into infrastructure you do not control. RecallX keeps the default posture
local-first and review-before-write.

- **Private by default:** Transcripts, recordings, summaries, and meeting indexes
  live on your machine unless you choose a provider or export destination.
- **Local AI where possible:** Use local transcription and Ollama-backed summary
  paths when you need maximum data control.
- **Meeting memory, not just notes:** Ask source-cited questions, review
  decisions, create follow-ups, and prepare agent handoffs from the same record.
- **Explicit automation:** Calendar sync, assisted join, screenshots, Apple
  Notes, Apple Reminders, MCP, and agent workflows require opt-in.

Sensitive automation is required to be explicit opt-in and auditable before it
ships. This includes calendar sync, assisted join, screenshots, external
exports, Apple Notes automation, Apple Reminders follow-ups, local MCP access,
meeting chat indexes, and agent skill setup. External integrations preview
destinations before writing data, and cloud AI providers receive selected
meeting content only after provider opt-in. See the
[privacy, consent, and access controls policy](docs/privacy-consent-access-controls.md).

## Features

- **Local First:** Meeting data is stored and processed locally by default.
- **Real-time Transcription:** Capture a live transcript while the meeting runs.
- **Transcription Quality Profiles:** Choose local accuracy profiles and
  preprocessing presets for live recordings or retranscription. See
  [Transcription Accuracy Release Notes](docs/transcription-accuracy-release-notes.md).
- **AI-Powered Summaries:** Generate summaries using Ollama, Claude, Groq,
  OpenRouter, OpenAI, or an OpenAI-compatible endpoint.
- **Custom Summary Templates:** Create local summary templates, preview markdown
  structure, import/export template JSON, and remember the selected template per
  meeting. See [Custom Summary Templates](docs/custom-summary-templates.md).
- **Advanced Exports:** Export completed meetings as Markdown, PDF, or DOCX with
  selectable sections and local auto-export preferences. See
  [Advanced Exports](docs/advanced-exports.md).
- **Meeting Chat:** Ask source-cited follow-up questions using local retrieval
  over transcripts, summaries, actions, notes, and enabled screenshot metadata.
  See [Meeting Chat Release Notes](docs/meeting-chat-release-notes.md).
- **Calendar Integration:** Connect Apple Calendar on macOS, sync upcoming event
  metadata locally, and select an event to prefill the next recording title and
  meeting-detection metadata. See
  [Calendar Integration Release Notes](docs/calendar-integration-release-notes.md).
- **Meeting Detection and Assisted Join:** Review local meeting prompts from
  approved calendar/window signals, score likely meetings, and optionally open
  supported meeting links with explicit user approval. See
  [Meeting Detection Release Notes](docs/meeting-detection-release-notes.md).
- **Recording Assistant:** Use recording preflight, runtime capture controls,
  privacy audit trails, and a post-recording review checklist. See
  [Recording Assistant Release Notes](docs/recording-assistant-release-notes.md).
- **Apple Notes Export:** Export meeting summaries to Apple Notes after a
  destination preview, update app-managed notes instead of duplicating them, and
  link exports with Apple Calendar artifacts. See
  [Apple Notes Release Notes](docs/apple-notes-release-notes.md).
- **Apple Reminders Follow-Ups:** Review editable follow-up drafts and create
  only selected reminders. See
  [Apple Reminders Release Notes](docs/apple-reminders-release-notes.md).
- **Speaker Labels and Active Speaker Timeline:** Review speaker confidence,
  align visual cues from meeting snapshots, and rename labels as user-confirmed
  corrections. See [Active Speaker Timeline](docs/active-speaker-timeline.md).
- **Optional Meeting Screenshots:** Enable periodic screenshots or call-window
  snapshots per meeting. Screenshots stay local, irrelevant captures can be
  filtered, and retained screenshots can be deleted from the timeline. See
  [Call-Window Snapshot Release Notes](docs/call-window-snapshot-release-notes.md).
- **Local MCP and Post-Meeting Agents:** Start an opt-in local MCP server,
  prepare meeting context packages, and run reviewable Codex or Claude
  post-meeting workflows with run history. See
  [Post-Meeting Agent Orchestration](docs/post-meeting-agent-orchestration.md).
- **Startup and Background Preferences:** On supported macOS installs, enable
  login startup and hidden-at-launch behavior without starting recordings,
  joining calls, or enabling microphone capture.
- **Flexible AI Provider Support:** Choose local or cloud providers per workflow.

## Installation

### Windows

1. Download the latest Windows installer from
   [Releases](https://github.com/CharLEE-X/meetily/releases/latest).
2. Run the installer.

### macOS

1. Download the latest Apple Silicon `.dmg` from
   [Releases](https://github.com/CharLEE-X/meetily/releases/latest).
2. Open the downloaded `.dmg` file.
3. Drag **RecallX** to your Applications folder.
4. Open **RecallX** from Applications.

### Linux

Build from source:

- [Building on Linux](docs/building_in_linux.md)
- [General Build Instructions](docs/BUILDING.md)

```bash
git clone https://github.com/CharLEE-X/meetily
cd meetily/frontend
pnpm install
./build-gpu.sh
```

## Key Features in Action

### Local Transcription

Transcribe meetings on your device using Whisper or Parakeet models.

<p align="center">
    <img src="docs/home.png" width="650" style="border-radius: 10px;" alt="RecallX local transcription" />
</p>

### Import and Retranscribe

Import existing audio files to generate transcripts, or re-transcribe recorded
meetings with a different model or language.

<p align="center">
    <img src="docs/meetily-export.gif" width="650" style="border-radius: 10px;" alt="Import and retranscribe" />
</p>

### AI-Powered Summaries

Generate meeting summaries with local or selected cloud providers.

<p align="center">
    <img src="docs/summary.png" width="650" style="border-radius: 10px;" alt="Summary generation" />
</p>

### Privacy-First Design

Transcription models, recordings, transcripts, meeting chat indexes, and
automation history are stored locally by default.

<p align="center">
    <img src="docs/settings.png" width="650" style="border-radius: 10px;" alt="Privacy and local settings" />
</p>

### Professional Audio Mixing

Capture microphone and system audio simultaneously with ducking and clipping
prevention.

<p align="center">
    <img src="docs/audio.png" width="650" style="border-radius: 10px;" alt="Device selection" />
</p>

### GPU Acceleration

Built-in support for hardware acceleration across platforms:

- **macOS:** Apple Silicon Metal and CoreML
- **Windows/Linux:** NVIDIA CUDA and AMD/Intel Vulkan

## System Architecture

RecallX is a single, self-contained desktop application built with
[Tauri](https://tauri.app/). A Rust native layer handles audio capture,
transcription, storage, notifications, summaries, and platform integrations. A
Next.js frontend provides the desktop UI.

For more details, see [Architecture](docs/architecture.md).

## For Developers

Start with the supported desktop app in `frontend/`:

```bash
cd frontend
pnpm install
pnpm run dev
pnpm run tauri:dev
```

See [CONTRIBUTING.md](CONTRIBUTING.md), [frontend/README.md](frontend/README.md),
and [docs/BUILDING.md](docs/BUILDING.md).

## Community Roadmap

RecallX is adding a local-first set of workflow features so storage, exports,
chat, Apple Notes, MCP, and agent workflows share the same meeting artifact
model instead of creating duplicated silos.

Shipped and planned feature areas:

- **Transcription accuracy and model selection:** higher-accuracy local profiles,
  preprocessing presets, language handling, and QA benchmarks.
- **Custom summary templates:** reusable local templates with per-meeting
  selection, regeneration, import/export, and editable copies of built-in
  templates.
- **Advanced exports:** Markdown, PDF, and DOCX exports with preview,
  destination history, and retryable failures.
- **Meeting detection and assisted join:** calendar-backed prompts, window-aware
  meeting signals, explainable scoring, and opt-in assisted join.
- **Recording assistant:** preflight checks, runtime capture controls, privacy
  audit records, and post-recording review.
- **Speaker identification and screenshots:** speaker labels, confidence review,
  user corrections, irrelevant-screenshot filtering, call-window snapshots, and
  deletion controls.
- **Meeting chat:** local-first retrieval over transcripts, summaries, and
  enabled artifacts with citations back to meeting sources.
- **Apple Notes and Apple Reminders:** review-before-write automation for
  summaries and follow-ups.
- **Local MCP and AI agents:** opt-in local agent access, scoped read-only tools,
  revocation, audit logs, run history, and manual rerun controls.

The implementation order and packaging gates are tracked in
[Pro-Equivalent Feature Architecture Roadmap](docs/pro-equivalent-architecture-roadmap.md).

## Contributing

Contributions are welcome. Please follow the established project structure and
guidelines in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT License. RecallX preserves the original Meetily MIT attribution in
[`LICENSE.md`](LICENSE.md).

## Acknowledgments

- RecallX is based on Meetily, originally created by Zackriya Solutions.
- We borrowed some code from [Whisper.cpp](https://github.com/ggerganov/whisper.cpp).
- We borrowed some code from [Screenpipe](https://github.com/mediar-ai/screenpipe).
- We borrowed some code from [transcribe-rs](https://crates.io/crates/transcribe-rs).
- Thanks to NVIDIA for developing the Parakeet model.
- Thanks to [istupakov](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx)
  for providing the ONNX conversion of the Parakeet model.
