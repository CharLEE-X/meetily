# Privacy, Consent, and Access Controls

This policy is the release gate for sensitive Meetily automation. It applies to recording, transcription, calendar access, assisted meeting join, screenshots, exports, Apple Notes automation, local MCP access, meeting chat indexes, and agent skill setup.

No sensitive automation may ship enabled by default. Every implementation issue for these areas must link to this policy and state how the feature satisfies the relevant section.

Implementation issues must include this checklist before shipping a sensitive feature:

* Default state and the exact opt-in action.
* Consent scope, including whether consent is global, per meeting, per provider, or per destination.
* Runtime indicator and pause, stop, or disable control.
* Revoke path and what happens to derived artifacts after revocation.
* Audit surface for access, export, automation, or generated artifacts.

## Consent and Safe Defaults

Sensitive features use explicit consent states:

| Feature | Default | Consent scope | UX owner | Required controls |
| --- | --- | --- | --- | --- |
| Microphone recording | Off until the user starts or schedules recording | Per meeting, with remembered device permission only after OS approval | Recording controls and onboarding | Start, pause, resume, stop, device selection, denied-permission recovery |
| System audio recording | Off until selected | Per meeting | Recording controls | Source preview, start, pause, stop, device unavailable recovery |
| Live transcription | Off until recording/import begins | Per meeting | Recording controls and meeting detail | Visible transcription status, pause through recording pause, delete transcript |
| Calendar sync | Off | Global provider opt-in plus per-calendar selection | Settings | Provider connect, selected calendars, disconnect, sync status, sync history |
| Auto-detect meetings | Off | Global opt-in | Settings and tray | Enable/disable, provider scope, last detection status, detection history |
| Assisted join | Off | Per meeting or per calendar source opt-in | Calendar/automation UI | Confirm before join, cancel, failed join status, join attempt history |
| Screenshots | Off | Per meeting confirmation, even if globally allowed | Recording controls | Visible capture indicator, next capture time, pause, stop, delete, capture history |
| Speaker identification | Off until enabled | Per meeting or model setting | Meeting detail | Label confidence disclosure, edit/confirm identities, delete labels, label history |
| External exports | Off; manual export only on user action | Per export destination, auto-export requires explicit destination setup | Export UI and Settings | Destination preview, export history, retry, revoke destination |
| Apple Notes automation | Off | Global Apple Events permission plus destination confirmation | Settings and export UI | Notes folder/account preview, last export status, export history, disconnect |
| Local MCP access | Off | Global server enablement plus per-client authorization | Settings | Client list, per-tool permissions, revoke, audit log |
| Meeting chat index | Off until chat/search feature is enabled | Global opt-in plus per-meeting inclusion control | Settings and meeting detail | Rebuild, exclude meeting, delete index, index build history |
| Agent skill setup | Off | Per install/update action | Settings | Source preview, install log, uninstall, revoke file access |

Consent must be informed and reversible. The first-run path can explain benefits, but it must not preselect or silently enable sensitive automation. If a permission is denied or later revoked, Meetily must show the affected feature as unavailable, keep existing local meeting data accessible, and provide a path to retry or disable the feature.

Revoking consent must stop future automation immediately. Derived artifacts created only for the revoked feature, such as screenshots, chat indexes, speaker labels, export records, or MCP client tokens, must be deleted or invalidated by default. Audit logs may keep minimal event metadata needed for accountability, but must not retain meeting content or prompts.

During a meeting, recording, transcription, screenshots, assisted join, and exports must expose immediate pause/stop/disable controls where the user is already working. Post-meeting views must disclose which sensitive automations ran, what artifacts were created, and where external writes were attempted.

## Threat Model

Primary risks:

* Accidental capture of private conversations, screens, or unrelated app content.
* Silent export of meeting data to files, Apple Notes, calendars, or cloud LLM providers.
* Malicious or over-broad local clients reading meeting content through MCP.
* Stale derived artifacts, such as screenshots or chat indexes, surviving meeting deletion.
* Confusing speaker labels with verified identity.

Controls:

* Default-off sensitive features and per-meeting confirmation for the highest-risk capture paths.
* Visible runtime indicators and immediate pause/stop controls.
* Local-only defaults for storage, transcription, MCP, and summaries.
* Auditable export and MCP histories without storing unnecessary prompt or content payloads.
* Consistent deletion semantics for source meetings and derived artifacts.
* Provider disclosure whenever summaries or chat use cloud APIs.

Any implementation that weakens these controls must be treated as release-blocking unless explicitly waived in the release checklist defined in [Security Review Checklist](security-review-checklist.md).

## Retention and Deletion Model

Meetily stores supported app data locally through the desktop app: SQLite for meeting records and metadata, plus app-data files for recordings, generated exports, screenshots, model caches, logs, and derived indexes. Deletion must cover both storage paths. OS-level backups or user-synced folders outside Meetily's control, such as Time Machine or iCloud Drive, are out of scope and should be disclosed when users choose external destinations.

| Artifact | Default retention | Deletion behavior | User controls | QA requirement |
| --- | --- | --- | --- | --- |
| Meeting metadata | Until user deletes the meeting | Remove the SQLite meeting row and cascade-delete app-managed child rows | Delete meeting | Deleting a meeting removes it from the meeting list and detail route |
| Audio recordings | Until user deletes the meeting or recording file | Delete local recording files and clear playback references | Delete meeting, delete recording when available | Deleted meetings cannot play stale audio files |
| Transcripts | Until user deletes the meeting or transcript | Remove transcript segments and search references | Delete meeting, clear transcript when available | Deleted meetings have no retrievable transcript segments |
| Summaries | Until user deletes the meeting or summary | Remove generated summary text, template output, and provider metadata | Delete meeting, regenerate summary | Deleted meetings have no summary payload or provider metadata |
| Template outputs | Until user deletes the meeting or output | Remove generated local output and references | Delete output, delete meeting | Regenerated templates do not restore deleted output unexpectedly |
| Exported PDF, DOCX, Markdown, or shared files | User-managed external files after export | Remove Meetily export records and optional local app-managed copies; do not delete user-chosen external files without confirmation | Export history, delete export record, reveal destination | Export record deletion does not silently delete unrelated user files |
| Screenshots | Until user deletes screenshots, disables the feature, or deletes the meeting | Delete app-managed image files and SQLite references | Pause screenshots, delete screenshot, delete meeting, retention limit | Meeting deletion removes screenshot files and timeline references |
| Speaker labels | Until user deletes labels or meeting | Remove labels, confidence metadata, and user-confirmed identity links | Edit label, clear labels, delete meeting | Clearing labels removes confirmed and detected identities |
| Chat and transcript search indexes | Until user excludes a meeting, disables search/chat, or deletes the meeting | Delete vector/search index files and meeting-to-index mapping | Exclude meeting, rebuild index, delete index, delete meeting | Deleted or excluded meetings cannot be returned by search or chat |
| Calendar metadata | Until provider disconnect, calendar deselect, or meeting deletion | Meeting deletion removes only that meeting's event link; calendar deselect removes calendar mappings; provider disconnect removes provider tokens and sync cursors | Disconnect provider, deselect calendar, delete meeting | Revoked calendar access stops sync and hides stale automation prompts |
| Apple Notes export records | Until user deletes export record, disconnects Notes automation, or deletes meeting | Remove export metadata and local status; do not delete Notes content without explicit confirmation | Disconnect, delete export record, retry failed export | Disconnect stops future writes and preserves visible prior-write status |
| MCP client tokens | Until user revokes a client or disables MCP | Revoke token/trust record and terminate active sessions | Revoke client, disable MCP | Revoked clients cannot call tools without reauthorization |
| MCP access logs | Retain minimal metadata for 30 days by default | Delete expired logs automatically; meeting deletion redacts meeting title/content references while preserving minimal event accountability | Clear logs, retention window | Logs never contain transcript text, prompts, screenshots, or summary bodies |
| Agent skill installer logs | Retain minimal metadata for 30 days by default | Remove install/update logs and source references on clear; uninstall removes app-managed files | Uninstall skill, clear logs | Cleared logs cannot expose source paths or meeting content |
| Provider request metadata | Keep minimal provider name, timestamp, and status for 30 days by default | Remove request IDs, provider names, and timestamps tied to the meeting; never store raw prompts beyond generated local output | Clear provider history, delete meeting, retention window | Cloud-provider summaries disclose provider use without retaining prompts |
| Model caches | Until user removes a model or resets model storage | Delete app-managed Whisper, Parakeet, sidecar, or downloaded model files and clear model registry entries | Delete model, reset model storage | Removing a model frees the app-managed file and does not delete unrelated user files |
| General app and diagnostic logs | Retain minimal logs for 30 days by default | Delete expired logs automatically; meeting deletion redacts meeting-specific identifiers where practical | Clear logs, retention window | Logs do not contain transcript text, prompts, screenshots, or summary bodies |

Retention settings should default to "keep until deleted" for core meeting records and 30 days for provider history, access logs, installer logs, and diagnostic logs. Any automatic cleanup must show the retention window in Settings before it deletes user-visible content or minimal accountability metadata.

Meeting deletion is authoritative. It must remove or invalidate all app-managed derived artifacts for that meeting, including transcripts, summaries, screenshots, indexes, speaker labels, local export records, provider metadata, and MCP log meeting references. When an artifact was written outside Meetily's app-managed storage, such as a user-selected export file or Apple Notes note, deletion must show the external destination and ask before deleting or modifying it.

Future implementation and QA issues must include regression cases for:

* Deleting a meeting with transcripts, summaries, screenshots, chat indexes, and export records.
* Revoking calendar, Apple Notes, MCP, screenshot, and chat-index consent.
* Expiring MCP access logs, provider history, diagnostic logs, and agent skill installer logs without deleting meeting content.
* Removing model caches without deleting unrelated user-selected files.
* Handling missing local files gracefully when metadata remains.
