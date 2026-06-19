<div align="center" style="border-bottom: none">
    <h1>
        <img src="docs/Meetily-6.png" style="border-radius: 10px;" />
        <br>
        Privacy-First AI Meeting Assistant
    </h1>
    <a href="https://trendshift.io/repositories/21958" target="_blank"><img src="https://trendshift.io/api/badge/repositories/21958" alt="Zackriya-Solutions%2Fmeetily | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/></a>
    <br>
    <br>
    <a href="https://github.com/CharLEE-X/meetily/releases/latest"><img src="https://img.shields.io/badge/Latest_Release-Download-brightgreen" alt="Latest Release"></a>
    <a href="https://github.com/CharLEE-X/meetily"><img alt="GitHub Repo stars" src="https://img.shields.io/github/stars/CharLEE-X/meetily?style=flat">
</a>
 <a href="https://github.com/CharLEE-X/meetily/releases"> <img alt="GitHub Downloads (all assets, all releases)" src="https://img.shields.io/github/downloads/CharLEE-X/meetily/total?style=plastic"> </a>
    <a href="https://github.com/CharLEE-X/meetily/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="License"></a>
    <a href="https://github.com/CharLEE-X/meetily/releases"><img src="https://img.shields.io/badge/Supported_OS-macOS,_Windows-white" alt="Supported OS"></a>
    <a href="https://github.com/CharLEE-X/meetily/releases"><img alt="GitHub Tag" src="https://img.shields.io/github/v/tag/CharLEE-X/meetily?include_prereleases&color=yellow">
</a>
    <br>
    <h3>
    <br>
    Open Source • Privacy-First • Enterprise-Ready
    </h3>
    <p align="center">
    Get latest <a href="https://www.zackriya.com/meetily-subscribe/"><b>Product updates</b></a> <br><br>
    <a href="https://meetily.ai"><b>Website</b></a> •
    <a href="https://www.linkedin.com/company/106363062/"><b>LinkedIn</b></a> •
    <a href="https://discord.gg/crRymMQBFH"><b>Meetily Discord</b></a> •
    <a href="https://discord.com/invite/vCFJvN4BwJ"><b>Privacy-First AI</b></a> •
    <a href="https://www.reddit.com/r/meetily/"><b>Reddit</b></a>
</p>
    <p align="center">

A privacy-first AI meeting assistant that captures, transcribes, and summarizes meetings entirely on your infrastructure. Built by expert AI engineers passionate about data sovereignty and open source solutions. Perfect for enterprises that need advanced meeting intelligence without compromising on privacy, compliance, or control.

</p>

<p align="center">
    <img src="docs/meetily_demo.gif" width="650" alt="Meetily Demo" />
    <br>
    <a href="https://youtu.be/6FnhSC_eSz8">View full Demo Video</a>
</p>

</div>

---

> **Meetily PRO Upgrade Offer** - Meetily PRO is available for users who need enhanced accuracy, advanced exports, custom summary workflows, and team-ready features. Use coupon code **LAUNCH20** for **20% off** until the next Meetily Community Edition release. Speaker diarization is also planned for PRO in mid-June. [Explore Meetily PRO →](https://meetily.ai/pro/)

---

<details>
<summary>Table of Contents</summary>

- [Introduction](#introduction)
- [Why Meetily?](#why-meetily)
- [Features](#features)
- [Installation](#installation)
- [Key Features in Action](#key-features-in-action)
- [System Architecture](#system-architecture)
- [For Developers](#for-developers)
- [Community Pro-Equivalent Roadmap](#community-pro-equivalent-roadmap)
- [Meetily PRO](#meetily-pro)
- [Contributing](#contributing)
- [License](#license)

</details>

## Introduction

Meetily is a privacy-first AI meeting assistant that runs entirely on your local machine. It captures your meetings, transcribes them in real-time, and generates summaries, all without sending any data to the cloud. This makes it the perfect solution for professionals and enterprises who need to maintain complete control over their sensitive information.

## Why Meetily?

While there are many meeting transcription tools available, this solution stands out by offering:

- **Privacy First:** All processing happens locally on your device.
- **Cost-Effective:** Uses open-source AI models instead of expensive APIs.
- **Flexible:** Works offline and supports multiple meeting platforms.
- **Customizable:** Self-host and modify for your specific needs.

Sensitive automation is required to be explicit opt-in and auditable before it ships. This includes calendar sync, assisted join, screenshots, external exports, Apple Notes automation, local MCP access, meeting chat indexes, and agent skill setup. External integrations must preview destinations before writing data, and cloud AI providers receive selected meeting content only after provider opt-in. See the [privacy, consent, and access controls policy](docs/privacy-consent-access-controls.md) for the default-off rules.

Local AI agents can connect to Meetily through the opt-in local MCP server. The
read-only MCP tool and authorization contract is documented in
[Meetily Local MCP Contract](docs/meetily-mcp.md).
Post-meeting Codex and Claude orchestration is documented separately in
[Post-Meeting Agent Orchestration](docs/post-meeting-agent-orchestration.md);
Meetily packages meeting context and hands it off, while agents own downstream
codebase, GitHub/GitLab, Linear/Jira, docs, and research work. Current release
limitations and QA gates are listed in
[Post-Meeting Agent Orchestration Release Notes](docs/post-meeting-agent-orchestration-release-notes.md).

Meeting detection and assisted join are documented in
[Meeting Detection and Assisted Join](docs/meeting-detection-assisted-join.md).
The feature is disabled by default, supports prompt-only and explicit auto-open
behavior, and never joins or records silently.

The Community fork upgrade roadmap is documented in
[Pro-Equivalent Feature Architecture Roadmap](docs/pro-equivalent-architecture-roadmap.md).
Sensitive automation in that roadmap is default-off and requires explicit user
opt-in before it captures, exports, indexes, syncs, or exposes meeting data.

Apple Reminders follow-up creation is designed in
[Apple Reminders Follow-Up Integration](docs/apple-reminders-follow-ups.md). The
workflow is default-off, local-first, and always review-before-write.

<details>
<summary>The Privacy Problem</summary>

Meeting AI tools create significant privacy and compliance risks across all sectors:

- **$4.4M average cost per data breach** (IBM 2024)
- **€5.88 billion in GDPR fines** issued by 2025
- **400+ unlawful recording cases** filed in California this year

Whether you're a defense consultant, enterprise executive, legal professional, or healthcare provider, your sensitive discussions shouldn't live on servers you don't control. Cloud meeting tools promise convenience but deliver privacy nightmares with unclear data storage practices and potential unauthorized access.

**Meetily solves this:** Complete data sovereignty on your infrastructure, zero vendor lock-in, and full control over your sensitive conversations.

</details>

## Features

- **Local First:** All processing is done on your machine. No data ever leaves your computer.
- **Real-time Transcription:** Get a live transcript of your meeting as it happens.
- **AI-Powered Summaries:** Generate summaries of your meetings using powerful language models.
- **Advanced Exports:** Export completed meetings as Markdown, PDF, or DOCX with selectable sections and local auto-export preferences. See [Advanced Exports](docs/advanced-exports.md).
- **Calendar Integration:** Connect Apple Calendar on macOS, sync upcoming event metadata locally, and select an event to prefill the next recording title and meeting-detection metadata. See [Calendar Integration Release Notes](docs/calendar-integration-release-notes.md).
- **Apple Reminders Follow-Ups:** On macOS, connect Apple Reminders, review editable follow-up drafts, create only selected reminders, and return to meeting-linked reminder history. See [Apple Reminders Release Notes](docs/apple-reminders-release-notes.md).
- **Apple Notes Export:** On macOS, export meeting summaries to Apple Notes after a destination preview, update app-managed notes instead of duplicating them, and link Notes exports with Apple Calendar artifacts when both integrations are enabled. See [Apple Notes Release Notes](docs/apple-notes-release-notes.md).
- **Speaker Labels:** Detect local speaker labels from transcript timing/source metadata, review them in the meeting transcript, and rename labels as user-confirmed corrections.
- **Optional Meeting Screenshots:** On macOS, periodic screenshots can be enabled in Settings and must be confirmed for each meeting before capture starts. Screenshots stay local and can be deleted from the meeting timeline. See [Speaker Identification and Screenshots Release Notes](docs/speaker-identification-release-notes.md).
- **Multi-Platform:** Works on macOS, Windows, and Linux.
- **Open Source:** Meetily is open source and free to use.
- **Flexible AI Provider Support:** Choose from Ollama (local), Claude, Groq, OpenRouter, or use your own OpenAI-compatible endpoint.

## Installation

### 🪟 **Windows**

1. Download the latest Windows installer from [Releases](https://github.com/CharLEE-X/meetily/releases/latest)
2. Run the installer

### 🍎 **macOS**

1. Download the latest Apple Silicon `.dmg` from [Releases](https://github.com/CharLEE-X/meetily/releases/latest)
2. Open the downloaded `.dmg` file
3. Drag **Meetily** to your Applications folder
4. Open **Meetily** from Applications folder

### 🐧 **Linux**

Build from source following our detailed guides:

- [Building on Linux](docs/building_in_linux.md)
- [General Build Instructions](docs/BUILDING.md)

**Quick start:**

```bash
git clone https://github.com/CharLEE-X/meetily
cd meetily/frontend
pnpm install
./build-gpu.sh
```

## Key Features in Action

### 🎯 Local Transcription

Transcribe meetings entirely on your device using **Whisper** or **Parakeet** models. No cloud required.

<p align="center">
    <img src="docs/home.png" width="650" style="border-radius: 10px;" alt="Meetily Demo" />
</p>

### 📥 Import & Enhance `Beta`

Import existing audio files to generate transcripts, or enhance to re-transcribe any recorded meeting with a different model or language, all processed locally.

> Contributed by [Jeremi Joslin](https://github.com/jeremi), improved by [Vishnu P S](https://github.com/p-s-vishnu) and [Mohammed Safvan](https://github.com/mohammedsafvan)

<p align="center">
    <img src="docs/meetily-export.gif" width="650" style="border-radius: 10px;" alt="Import and Enhance" />
</p>

### 🤖 AI-Powered Summaries

Generate meeting summaries with your choice of AI provider. **Ollama** (local) is recommended, with support for Claude, Groq, OpenRouter, and OpenAI.

<p align="center">
    <img src="docs/summary.png" width="650" style="border-radius: 10px;" alt="Summary generation" />
</p>

<p align="center">
    <img src="docs/editor1.png" width="650" style="border-radius: 10px;" alt="Editor Summary generation" />
</p>

### 🔒 Privacy-First Design

All data stays on your machine. Transcription models, recordings, and transcripts are stored locally.

<p align="center">
    <img src="docs/settings.png" width="650" style="border-radius: 10px;" alt="Local Transcription and storage" />
</p>

### 🌐 Custom OpenAI Endpoint Support

Use your own OpenAI-compatible endpoint for AI summaries. Perfect for organizations with custom AI infrastructure or preferred providers.

<p align="center">
    <img src="docs/custom.png" width="650" style="border-radius: 10px;" alt="Custom OpenAI Endpoint Configuration" />
</p>

### 🎙️ Professional Audio Mixing

Capture microphone and system audio simultaneously with intelligent ducking and clipping prevention.

<p align="center">
    <img src="docs/audio.png" width="650" style="border-radius: 10px;" alt="Device selection" />
</p>

### ⚡ GPU Acceleration

Built-in support for hardware acceleration across platforms:

- **macOS**: Apple Silicon (Metal) + CoreML
- **Windows/Linux**: NVIDIA (CUDA), AMD/Intel (Vulkan)

Automatically enabled at build time - no configuration needed.

## System Architecture

Meetily is a single, self-contained application built with [Tauri](https://tauri.app/). It uses a Rust-based backend to handle all the core logic, and a Next.js frontend for the user interface.

For more details, see the [Architecture documentation](docs/architecture.md).

## For Developers

If you want to contribute to Meetily or build it from source, you'll need to have Rust and Node.js installed. For detailed build instructions, please see the [Building from Source guide](docs/BUILDING.md).

## Community Pro-Equivalent Roadmap

This fork is adding a local-first set of Pro-equivalent workflow features in a
phased order so storage, exports, chat, Apple Notes, MCP, and agent workflows
share the same meeting artifact model instead of creating duplicated silos.

Planned feature areas:

- **Transcription accuracy and model selection:** higher-accuracy local profiles,
  preprocessing presets, language handling, and QA benchmarks.
- **Custom summary templates:** reusable local templates with per-meeting
  selection and regeneration.
- **Advanced exports:** Markdown, PDF, and DOCX exports with preview,
  destination history, and retryable failures.
- **Meeting detection and assisted join:** calendar-backed prompts and optional
  assisted join flows that remain opt-in. The first implementation supports
  Google Meet, Zoom, and Microsoft Teams URLs from approved event metadata.
- **Speaker identification and screenshots:** diarization/speaker labels and
  periodic screenshots with per-meeting confirmation, visible capture state, and
  deletion controls.
- **Meeting chat:** local-first retrieval over transcripts, summaries, and
  enabled artifacts with citations back to meeting sources.
- **Calendar integration:** selected-calendar sync that stores minimal event
  metadata, can be disconnected, starts with Apple Calendar on macOS, and keeps
  a provider-neutral path for future ICS and Google Calendar support. See
  [Calendar Integration](docs/calendar-integration.md) and
  [Calendar Integration Release Notes](docs/calendar-integration-release-notes.md).
- **Apple Notes export:** opt-in macOS automation with destination preview before
  writing meeting content, local export history, and Apple Calendar artifact
  linking. See [Apple Notes Export](docs/apple-notes-export.md) and
  [Apple Notes Release Notes](docs/apple-notes-release-notes.md).
- **Local MCP server:** opt-in local agent access with trusted clients, scoped
  read-only tools, revocation, and audit logs.
- **AI-agent skill setup:** reversible setup for supported agents and ask-first
  post-meeting workflows.

The implementation order, shared artifact model, service contracts, UI map, and
packaging gates are tracked in
[Pro-Equivalent Feature Architecture Roadmap](docs/pro-equivalent-architecture-roadmap.md).

## Meetily Pro

<p align="center">
    <img src="docs/pv2.1.png" width="650" style="border-radius: 10px;" alt="Upcoming version" />
</p>

**Meetily PRO** is a professional-grade solution with enhanced accuracy and advanced features for serious users and teams. Built on a different codebase with superior transcription models and enterprise-ready capabilities.

### Community Thank-You Offer

Meetily Community Edition will remain free and open source. PRO exists for users and teams who want a more advanced meeting workflow, including higher transcription accuracy, custom summary templates, advanced exports, auto-meeting detection, and self-hosted deployment options.

For the community that helped Meetily grow, we are making the upgrade easier: use coupon code **LAUNCH20** for **20% off Meetily PRO** until the next Meetily Community Edition release.

Speaker diarization is planned for mid-June, bringing automatic speaker separation to PRO meetings.

### Key Advantages Over Community Edition:

- **Enhanced Accuracy**: Superior transcription models for professional-grade accuracy
- **Custom Summary Templates**: Tailor summaries to your specific workflow and needs
- **Advanced Export Options**: PDF, DOCX, and Markdown exports with formatting
- **Auto-detect and Join Meetings**: Automatic meeting detection and joining
- **Speaker Identification**: Distinguish between speakers automatically *(Coming Soon)*
- **Chat with Meetings**: AI-powered meeting insights and queries *(Coming Soon)*
- **Calendar Integration**: Calendar-backed meeting context, now started in the Community fork with local Apple Calendar sync on macOS
- **Self-Hosted Deployment**: Deploy on your own infrastructure for teams
- **GDPR Compliance Built-In**: Privacy by design architecture with complete audit trails
- **Priority Support**: Dedicated support for PRO users

### Who is PRO for?

- **Professionals** who need the highest accuracy for critical meetings
- **Teams and organizations** (2-100 users) requiring self-hosted deployment
- **Power users** who need advanced export formats and custom workflows
- **Compliance-focused organizations** requiring GDPR readiness

> **Note:** Meetily Community Edition remains **free & open source forever** with local transcription, AI summaries, and core features. PRO is a separate professional solution for users who need enhanced accuracy and advanced capabilities.

For organizations needing 100+ users or managed compliance solutions, explore [Meetily Enterprise](https://meetily.ai/enterprise/).

**Learn more about pricing and features:** [https://meetily.ai/pro/](https://meetily.ai/pro/)

## Contributing

We welcome contributions from the community! If you have any questions or suggestions, please open an issue or submit a pull request. Please follow the established project structure and guidelines. For more details, refer to the [CONTRIBUTING.md](CONTRIBUTING.md) file.

Thanks for all the contributions. Our community is what makes this project possible.

## License

MIT License - Feel free to use this project for your own purposes.

## Acknowledgments

- We borrowed some code from [Whisper.cpp](https://github.com/ggerganov/whisper.cpp).
- We borrowed some code from [Screenpipe](https://github.com/mediar-ai/screenpipe).
- We borrowed some code from [transcribe-rs](https://crates.io/crates/transcribe-rs).
- Thanks to **NVIDIA** for developing the **Parakeet** model.
- Thanks to [istupakov](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) for providing the **ONNX conversion** of the Parakeet model.

## Star History

[![Star History Chart](https://api.star-history.com/chart?repos=Zackriya-Solutions/meetily&type=date&legend=top-left)](https://www.star-history.com/?repos=Zackriya-Solutions%2Fmeetily&type=date&legend=bottom-right)
