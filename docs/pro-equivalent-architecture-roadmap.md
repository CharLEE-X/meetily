# Pro-Equivalent Feature Architecture Roadmap

This document is the delivery plan for the Meetily Community Pro feature track.
It covers CHA-1673 and its architecture children:

* CHA-1716: README fork upgrade coverage.
* CHA-1717: current codebase boundary and native capability audit.
* CHA-1718: shared meeting artifact model and migration plan.
* CHA-1719: backend service contracts.
* CHA-1720: frontend navigation and settings architecture.
* CHA-1721: phased implementation roadmap and dependency gates.
* CHA-1722: macOS packaging, entitlements, and cross-platform impact.

The supported product surface is the Tauri desktop app:
`frontend/src` for Next.js/React/TypeScript and `frontend/src-tauri` for
Rust/Tauri services. The archived Python backend is not part of this roadmap.

## Architecture Principles

* Keep Meetily local-first. Core meeting records, generated artifacts, indexes,
  and audit metadata live in app-managed local storage unless the user explicitly
  chooses an external destination or cloud provider.
* Put source-of-truth persistence and sensitive native behavior in Rust/Tauri.
  Frontend code owns presentation, user intent capture, and optimistic UI only.
* Build one meeting artifact graph. Exports, chat, Apple Notes, MCP, screenshots,
  summaries, and calendar links must read from shared repositories instead of
  creating feature-specific storage silos.
* Sensitive automation is default-off and reversible. Implementation issues must
  follow `docs/privacy-consent-access-controls.md` and
  `docs/security-review-checklist.md`.
* Long-running operations use a common job/progress/cancellation pattern so
  transcription, summaries, exports, indexing, calendar sync, screenshots, and
  agent workflows feel consistent.

## Current Codebase Boundary Audit

### Frontend Modules

| Area | Current owner | Planned attachment points |
| --- | --- | --- |
| Main recording workspace | `frontend/src/app/page.tsx`, `frontend/src/components/RecordingControls.tsx`, `frontend/src/contexts/RecordingStateContext.tsx` | Recording profile selector, meeting detection prompts, screenshot runtime indicator, speaker-label status |
| App shell and providers | `frontend/src/app/layout.tsx`, `frontend/src/components/MainNav/`, `frontend/src/components/MainContent/` | Navigation entries for settings, calendar, templates, exports, and meeting chat |
| Meeting detail | `frontend/src/app/meeting-details/`, `frontend/src/components/MeetingDetails/`, `frontend/src/hooks/meeting-details/` | Export menu, chat panel, artifact timeline, speaker labels, screenshot gallery, template regeneration |
| Settings and provider setup | `frontend/src/components/SettingTabs.tsx`, model/provider settings components, `frontend/src/components/McpSettings.tsx` | Templates, export destinations, calendar providers, Apple Notes, chat/index controls, agent workflows |
| Tauri wrappers | `frontend/src/services/`, `frontend/src/lib/tauri.ts` | One service wrapper per native domain, with browser-safe fallbacks where needed |
| Import/retranscription | `frontend/src/components/ImportAudio/`, `frontend/src/components/MeetingDetails/RetranscribeDialog.tsx` | Transcription profile reuse and artifact regeneration jobs |

Frontend rule: every new Tauri command call should live behind a focused service
or hook. UI components should not know raw filesystem paths, token bodies, or
provider secrets.

### Rust/Tauri Modules

| Area | Current owner | Planned attachment points |
| --- | --- | --- |
| Command registration and app state | `frontend/src-tauri/src/lib.rs`, `frontend/src-tauri/src/state.rs` | Register new domain commands and shared job state |
| Audio capture and recording | `frontend/src-tauri/src/audio/`, `frontend/src-tauri/src/audio_v2/` | Transcription profiles, preprocessing presets, screenshot pause coupling |
| Transcription engines | `frontend/src-tauri/src/whisper_engine/`, `frontend/src-tauri/src/parakeet_engine/`, `frontend/src-tauri/src/audio/transcription/` | Quality profile metadata, benchmarking hooks, model readiness states |
| Summary and templates | `frontend/src-tauri/src/summary/`, `frontend/src-tauri/templates/` | Custom template storage, render contexts, regeneration jobs |
| Persistence | `frontend/src-tauri/src/database/`, `frontend/src-tauri/migrations/` | Shared artifact tables, cleanup transactions, index/export/calendar repositories |
| MCP | `frontend/src-tauri/src/mcp/mod.rs`, `frontend/src/services/mcpService.ts` | Read-only meeting tools already exist; future write/export tools require another release gate |
| Notifications and tray | `frontend/src-tauri/src/notifications/`, `frontend/src-tauri/src/tray.rs` | Background job completion, meeting detection prompts, active capture status |
| Packaging | `frontend/src-tauri/tauri.conf.json`, `frontend/src-tauri/tauri.appstore.conf.json`, `frontend/scripts/` | Entitlements, App Store/TestFlight packaging, sidecar signing, unsupported-state behavior |

Rust rule: native services own permission checks, app-managed paths, database
transactions, generated files, and deletion cleanup.

### Existing Native Capabilities

| Capability | Current status | Notes |
| --- | --- | --- |
| Microphone and system audio capture | Available | Recording commands already manage device selection and notifications. |
| File IO and app data paths | Available | `DatabaseManager` uses Tauri app data directory; export work must also use Tauri path APIs. |
| SQLite migrations | Available | `sqlx::migrate!("./migrations")` runs at startup. New schema changes should be additive first. |
| Local model sidecars | Available | Whisper, Parakeet, and summary sidecar paths exist; sidecar signing must stay part of packaging checks. |
| Local MCP server | Available | Server is opt-in, loopback-only, token-scoped, and read-only for meeting content. |
| Notifications | Available | Use for background job completion and recoverable failures. |
| AppleScript/Shortcuts automation | Not implemented | Required for Apple Notes/Calendar automation if chosen; needs Apple Events entitlement and UX disclosure. |
| Screenshots | Not implemented | Requires macOS Screen Recording permission and a visible capture state. |
| Calendar access | Not implemented | Needs provider decision: EventKit/Apple Calendar, Google Calendar, ICS, or a staged combination. |
| Meeting chat index | Not implemented | Needs artifact model, retention/deletion rules, and provider/local model routing. |

### Refactoring Before Feature Work

* Add a shared artifact repository layer before building screenshots, exports,
  chat indexes, or Apple Notes records.
* Add a common job runner before multiple long-running features independently
  invent progress/cancel/error state.
* Keep provider API keys out of feature-specific tables; use existing provider
  settings patterns and add history/retention metadata separately.
* Normalize timestamp ownership. Transcript segment timing, screenshot timing,
  export timestamps, and index chunk timestamps should all be recording-relative
  where possible and UTC for persisted lifecycle events.

## Shared Meeting Artifact Model

The canonical model is a graph rooted at `meetings.id`. Feature tables should
reference meeting IDs and, where useful, artifact IDs. Large binaries and
generated documents should be stored as app-managed files with database metadata,
not inline blobs.

### Existing Tables

| Table | Current role |
| --- | --- |
| `meetings` | Meeting metadata, title, created/updated timestamps, optional folder path |
| `transcripts` | Transcript segments with text, timestamps, action items, key points, optional audio timing |
| `transcript_chunks` | Full transcript text and chunking/model metadata |
| `summary_processes` | Summary job status, summary JSON result, error, timing, backup |
| `meeting_notes` | User notes per meeting |
| `settings`, `transcript_settings` | Provider/model settings and API keys |

### New Artifact Tables

Add these tables through `frontend/src-tauri/migrations/` when each feature
slice starts. The first storage implementation should introduce the common base
tables and no-op repositories even before feature UIs are complete.

| Entity | Suggested table | Owner | Purpose |
| --- | --- | --- | --- |
| Artifact registry | `meeting_artifacts` | Rust database | One row per generated or linked artifact with `id`, `meeting_id`, `kind`, `status`, `created_at`, `updated_at`, `deleted_at`, `metadata_json` |
| Generated files | `meeting_artifact_files` | Rust database + app data files | App-managed file path, format, byte size, checksum, retention class |
| Template outputs | `summary_template_outputs` | Summary service | Template ID/version, render input hash, output artifact ID |
| Export records | `meeting_exports` | Export service | Format, destination kind, destination label/path, included content, status, retry/error |
| Apple Notes records | `apple_notes_exports` | Apple Notes service | Account/folder/note identifiers where available, destination preview, last write status |
| Screenshots | `meeting_screenshots` | Screenshot service | Capture timestamp, display/window label, file artifact ID, redaction status |
| Speaker labels | `speaker_labels`, `transcript_speaker_segments` | Diarization/speaker service | Detected vs user-confirmed labels, confidence, segment mapping |
| Chat/index chunks | `meeting_index_chunks` | Chat/index service | Source artifact ID, text span/timestamp, embedding/index reference, model metadata |
| Calendar links | `meeting_calendar_links` | Calendar service | Provider, event ID, calendar ID, meeting URL, sync status |
| Automation runs | `meeting_automation_runs` | Job/agent services | Agent/workflow/template invocation status without prompts or meeting bodies |
| Access audit | existing MCP audit JSON, future `meeting_access_audit` | MCP and integration services | Minimal content-free access/export/sync metadata |

The existing `transcripts.speaker` column from
`frontend/src-tauri/migrations/20251110000001_add_speaker_field.sql` remains a
legacy/simple source-label field during the transition. Phase 6 speaker work
should treat it as backwards-compatible input, backfill richer
`speaker_labels` and `transcript_speaker_segments` rows where possible, and then
keep it as a denormalized compatibility field until all readers use the richer
tables.

The detailed CHA-1667 data model, screenshot retention rules, correction
history, consent states, and manual-only visual identification boundary are
defined in [Speaker Identification and Screenshots](speaker-identification-screenshots.md).

The current MCP audit precedent is implemented in
`frontend/src-tauri/src/mcp/mod.rs` and persisted as the local
`mcp_audit_log.json` file. A future `meeting_access_audit` table should mirror
that content-free shape rather than storing transcript text, summary bodies,
prompts, screenshots, or local file paths.

### File Storage

Use app-managed storage under the Tauri app data directory:

```text
meeting_minutes.sqlite
artifacts/
  meetings/<meeting-id>/
    recordings/
    screenshots/
    exports/
    indexes/
    templates/
logs/
models/
```

External exports chosen by the user remain user-managed files. Meetily stores
metadata and status only; it must not delete external files without explicit
confirmation.

### Migration Strategy

1. Add new tables with nullable/defaulted columns and indexes by `meeting_id`,
   `kind`, `status`, and `created_at`.
2. Backfill artifact rows lazily when a meeting is opened, exported, indexed, or
   used by MCP. Do not block app startup on a full-library migration.
3. Preserve backwards-compatible reads from `transcripts`, `transcript_chunks`,
   and `summary_processes` until all callers use repositories.
4. Implement meeting deletion as a transaction plus file cleanup queue:
   database rows are marked/deleted first, then app-managed files are removed,
   and orphan cleanup retries on next startup.
5. Rollback expectation: additive migrations do not need destructive rollback.
   If a later migration requires data movement, ship a backup/export path and a
   read compatibility layer before removing old columns.

### Deletion and Retention

Deleting a meeting must delete or invalidate all app-managed artifacts for that
meeting: transcripts, summaries, template outputs, screenshots, speaker labels,
chat indexes, export records, provider metadata, automation runs, and MCP/access
meeting references. External destinations such as user-selected files or Apple
Notes content require destination preview and explicit deletion confirmation.

## Backend Service Contracts

All contracts should expose Tauri commands for user actions, Tauri events for
progress, and repository methods for persistence. Long-running jobs use this
shape:

```ts
type JobStatus = "queued" | "running" | "completed" | "failed" | "cancelled"

interface MeetilyJobEvent {
  jobId: string
  meetingId?: string
  kind: string
  status: JobStatus
  progress?: number
  message?: string
  errorCode?: string
}
```

Command errors should be user-safe strings or typed error codes. Logs must not
contain transcript text, prompts, screenshots, raw tokens, API keys, or exported
content.

| Service | Rust module target | Frontend wrapper | Commands/events |
| --- | --- | --- | --- |
| Transcription profiles | `audio/transcription/`, `whisper_engine/`, `parakeet_engine/` | `transcriptService.ts`, new `transcriptionProfileService.ts` | `list_transcription_profiles`, `set_default_transcription_profile`, `retranscribe_with_profile`; events `transcription-profile-progress` |
| Summary templates | `summary/templates/`, `summary/template_commands.rs` | meeting detail hooks and settings service | `list_summary_templates`, `save_summary_template`, `render_summary_template`, `generate_summary_with_template` |
| Export renderer | new `export/` module | new `exportService.ts` | `preview_meeting_export`, `render_meeting_export`, `open_export_destination`; events `export-progress` |
| Apple Notes | new `apple_notes/` module | `exportService.ts` plus settings | `check_apple_notes_availability`, `preview_apple_notes_export`, `export_to_apple_notes`, `disconnect_apple_notes` |
| Calendar sync | new `calendar/` module | new `calendarService.ts` | `list_calendar_providers`, `connect_calendar_provider`, `sync_calendar_events`, `link_meeting_calendar_event` |
| Meeting automation | new `meeting_automation/` module | recording and calendar hooks | `list_detected_meetings`, `dismiss_detected_meeting`, `start_assisted_join` |
| Screenshots | new `screenshots/` module | recording controls and meeting detail | `request_screenshot_permission`, `start_meeting_screenshots`, `pause_meeting_screenshots`, `delete_meeting_screenshot` |
| Speaker labels | new `speaker/` module | meeting detail hooks | `run_speaker_labeling`, `update_speaker_label`, `clear_speaker_labels` |
| Meeting chat/index | new `meeting_chat/` module | new chat service | `build_meeting_index`, `ask_meeting`, `delete_meeting_index`; events `meeting-index-progress` |
| MCP | existing `mcp/` module | `mcpService.ts` | Existing read-only tools are Phase 0 complete; write/export tools require the gates in `docs/privacy-consent-access-controls.md` and `docs/security-review-checklist.md` |
| Agent skills/workflows | new `agent_workflows/` module | MCP/settings service | `list_agent_workflows`, `save_agent_workflow_settings`, `prepare_agent_context`, `run_agent_workflow` |

Sensitive services must call permission/consent checks before doing work, not
only from the UI. Commands that create external writes must support preview
before execution.

## Frontend Navigation and Settings Plan

### Global Settings

| Settings section | Features |
| --- | --- |
| AI Models | Existing provider/model setup, transcription quality profiles, local/cloud provider disclosure |
| Templates | Custom summary template CRUD, import/export template JSON, default template |
| Exports | Default export formats, app-managed export folder, external destination history, Apple Notes setup |
| Calendar | Provider connection, selected calendars, sync status/history, meeting detection preferences |
| Privacy | Retention windows, deletion policy, screenshot availability, chat/index inclusion, provider history |
| MCP and Agents | Existing MCP status/client/audit UI, agent setup, default agent, post-meeting workflows |
| Packaging/About | Version, update channel, diagnostics, unsupported platform notes |

### Meeting Detail

* Keep transcript and summary as primary panels.
* Add an artifact rail or tab group for screenshots, exports, Notes status,
  calendar link, speaker labels, and chat.
* Export actions should start with `Preview export`, then `Export`.
* Chat should cite transcript timestamps and artifact IDs; it must not include
  excluded meetings or deleted artifacts.
* Destructive actions require confirmation and state the affected artifacts.

### Runtime Recording Surface

* Recording remains the main action.
* If screenshots are enabled for the meeting, show active indicator, next capture
  time, pause/resume, and stop controls.
* Meeting detection prompts must be dismissible and must not auto-start
  recording or assisted join unless the user has explicitly enabled that mode.

### Shared UI States

Every feature surface should define:

* Disabled by default.
* Not configured.
* Configured but unavailable because of OS/platform/provider permission.
* Running with progress.
* Failed with retry.
* Revoked/disconnected.
* Deleted or externally unavailable.

## Delivery Sequence and Dependency Gates

### Phase 0: Completed Foundations

| Issue | Status | Gate |
| --- | --- | --- |
| CHA-1674 privacy/consent/access controls | Done | Sensitive feature policy exists |
| CHA-1671 local MCP meeting access | Done | Read-only MCP server, scopes, audit, setup UI |
| CHA-1716 README upgrade coverage | Done in this branch | README lists all planned upgrade areas and links this roadmap |
| CHA-1673 architecture plan | This branch | Shared roadmap before feature implementation |

### Phase 1: Storage, Jobs, and Export Foundation

Primary candidates:

1. CHA-1718 shared artifact storage model and migrations.
2. New child: common job runner/progress model.
3. CHA-1665 advanced exports: start with Markdown and app-managed export
   records before PDF/DOCX.
4. Keep README roadmap coverage updated whenever a future slice ships.

Exit gates:

* Artifact tables and repositories exist.
* Meeting deletion cleanup covers new app-managed artifact rows/files.
* Export preview exists before external writes.
* `cargo check`, `pnpm run build`, and targeted repository tests pass.

### Phase 2: Transcription Quality and Summary Templates

Primary candidates:

1. CHA-1675 current transcription benchmark baseline.
2. CHA-1676 high-accuracy model profiles.
3. CHA-1677 preprocessing/language handling.
4. CHA-1664 custom summary templates.
5. CHA-1678 QA/docs/release notes.

Parallelization:

* Benchmark fixtures can run in parallel with template UI.
* Model profile persistence must land before per-meeting overrides.

Exit gates:

* Quality/speed tradeoffs are visible.
* Existing fast/default mode remains available.
* Template render output is stored as artifacts.

### Phase 3: Advanced External Destinations

Primary candidates:

1. Continue CHA-1665 PDF/DOCX renderers.
2. CHA-1670 Apple Notes export.
3. If Notes folder naming depends on event context, land only the minimal
   provider-neutral calendar metadata slice from CHA-1669 first; otherwise
   Apple Notes can ship without waiting for full calendar sync.

Exit gates:

* Destination preview before export or Notes write.
* Failed writes remain visible and retryable.
* App Store/TestFlight entitlements are verified for Apple Events if used.

### Phase 4: Meeting Chat and Retrieval

Primary candidates:

1. CHA-1668 meeting chat.
2. Index chunk storage from CHA-1718.
3. Provider disclosure and local/cloud routing controls from privacy settings.

Exit gates:

* Indexed content respects meeting deletion and exclusion.
* Answers cite transcript timestamps or artifact references.
* Cloud provider use is explicit.

### Phase 5: Calendar and Meeting Automation

Primary candidates:

1. CHA-1669 calendar integration.
2. CHA-1666 auto-detect meetings and assisted join.

Exit gates:

* Provider disconnect stops sync.
* Assisted join requires confirmation unless explicitly configured.
* Stale prompts disappear after disconnect or event deletion.

### Phase 6: Diarization and Screenshots

Primary candidates:

1. CHA-1667 speaker identification, diarization, screenshots.

Exit gates:

* Screenshots require per-meeting confirmation.
* Active capture state is visible.
* Permission denial/revocation does not break recording.
* Detected speaker labels are distinct from user-confirmed identities.

### Phase 7: Agent Skills and Post-Meeting Workflows

Primary candidates:

1. CHA-1672 agent skill setup epic.
2. CHA-1735 default agent/post-meeting settings.
3. CHA-1736 workflow runner after summary completion.
4. CHA-1737 Linear follow-up action template.
5. CHA-1738 supported agent invocation docs.

Exit gates:

* Agent setup is reversible.
* Post-meeting actions are off or ask-first by default.
* Context packages prefer MCP references and bounded summaries instead of
  dumping full meeting content into prompts.

## macOS Packaging, Entitlements, and Cross-Platform Impact

### Current Packaging State

* Standard Tauri build config: `frontend/src-tauri/tauri.conf.json`.
* Mac App Store config: `frontend/src-tauri/tauri.appstore.conf.json`.
* App Store entitlements:
  `frontend/src-tauri/entitlements.appstore.plist` and
  `frontend/src-tauri/entitlements.appstore.nested.plist`.
* App Store scripts:
  `frontend/scripts/build-appstore-macos.js` and
  `frontend/scripts/upload-appstore-macos.js`.
* Signing material belongs in `.signing/` and must stay untracked.

### Entitlement and Permission Matrix

| Feature | macOS permission/entitlement | Current state | Decision |
| --- | --- | --- | --- |
| Microphone recording | `NSMicrophoneUsageDescription`, audio-input entitlement for App Store | Present | Keep required |
| System audio capture | Screen/audio capture implementation dependent | Partially platform-specific | Re-test per release |
| Screenshots | Screen Recording privacy permission, possible user prompt only | Not present in app flow | Add feature-gated permission UX before implementation |
| Apple Notes automation | Apple Events automation entitlement/usage copy if AppleScript is used | Not present | Add only when Notes implementation path is chosen |
| Apple Calendar/EventKit | Calendar usage description/entitlement or Apple Events depending path | Not present | Decide provider path before coding |
| Local MCP | Network server entitlement for App Store | Present in App Store entitlements | Keep loopback-only and default-off |
| File exports | User-selected file paths or app-managed storage | Tauri fs plugin present | Preview destination and avoid broad file access |
| Sidecars | Code signing for nested binaries | App Store script signs known nested executables | Update script when adding sidecars |

### Cross-Platform Product Behavior

| Feature | macOS | Windows/Linux |
| --- | --- | --- |
| Apple Notes | Available only if Apple automation path is implemented | Show unsupported state, do not show setup as failed |
| Apple Calendar | macOS-only if EventKit/AppleScript path | Prefer provider-neutral calendar model so Google/ICS can work later |
| Screenshots | Use macOS Screen Recording permission | Use platform-specific capture later; keep feature unavailable until implemented |
| Local MCP | Supported | Supported with platform-specific config paths |
| Exports | Supported | Supported |
| Diarization/chat/templates | Supported if dependencies build | Supported if dependencies build |

Packaging release gates:

* `pnpm run build`.
* `cargo check --manifest-path frontend/src-tauri/Cargo.toml`.
* `pnpm run tauri:build` or target packaging script when packaging changes.
* Manual app run for permission, Apple Events, screenshots, tray, or local server
  behavior.
* App Store/TestFlight build when entitlements or nested binaries change.

## README Fork Upgrade Coverage

README must mention every planned fork upgrade feature and link here for details:

* Transcription accuracy and model selection.
* Custom summary templates.
* Advanced PDF, DOCX, and Markdown exports.
* Auto-detect meetings and assisted join.
* Speaker identification and screenshots.
* Meeting chat.
* Calendar integration.
* Apple Notes export.
* Local MCP server.
* AI-agent skill setup and post-meeting workflows.

Privacy-sensitive entries must say they are opt-in/default-off or require an
explicit destination/provider/client setup.

## Open Decisions and Follow-Ups

| Decision | Needed before | Current default |
| --- | --- | --- |
| Calendar provider path: Apple Calendar, Google, ICS, or staged mix | CHA-1669 and CHA-1666 | Stage provider-neutral metadata first |
| Apple Notes implementation path: AppleScript, Shortcuts, or native bridge | CHA-1670 | Do not implement until entitlement and UX are finalized |
| Chat index engine and embedding model | CHA-1668 | Start with local, deleteable index abstraction |
| PDF/DOCX renderer dependencies | CHA-1665 | Review license and bundle size before adding |
| Diarization model/dependency | CHA-1667 | Keep labels manual/detected distinction regardless of engine |
| Common job runner shape | Phase 1 | Add before multiple long-running features ship |
