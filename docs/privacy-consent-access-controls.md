# Privacy, Consent, and Access Controls

This policy is the release gate for sensitive Meetily automation. It applies to recording, transcription, calendar access, assisted meeting join, screenshots, exports, Apple Notes automation, local MCP access, meeting chat indexes, agent skill setup, and post-meeting agent orchestration.

Speaker identification and screenshot implementation details are defined in
[Speaker Identification and Screenshots](speaker-identification-screenshots.md).
That document is the data-model and consent contract for CHA-1667 work.

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
| Speaker identification | Off until enabled | Per meeting prompt tied to the source that creates labels: screenshot confirmation when labels use screenshots, or diarization setup when labels use a future audio model | Meeting detail | Label source disclosure, label confidence disclosure, edit/confirm identities, delete labels, label history |
| External exports | Off; manual export only on user action | Per export destination, auto-export requires explicit destination setup | Export UI and Settings | Destination preview, export history, retry, revoke destination |
| Apple Notes automation | Off | Global Apple Events permission plus destination confirmation | Settings and export UI | Notes folder/account preview, last export status, export history, disconnect |
| Apple Reminders follow-ups | Off | Global provider opt-in plus explicit per-create approval | Settings and meeting detail | Reminders list preview, editable draft review, create selected only, creation history, disconnect |
| Local MCP access | Off | Global server enablement plus per-client authorization | Settings | Client list, per-tool permissions, revoke, audit log |
| Meeting chat index | Off until chat/search feature is enabled | Global opt-in plus per-meeting inclusion control | Settings and meeting detail | Rebuild, exclude meeting, delete index, index build history |
| Agent skill setup | Off | Per install/update action | Settings | Source preview, install log, uninstall, revoke file access |
| Post-meeting agent orchestration | Off | Global trigger mode plus per-run approval unless auto-trigger is explicitly enabled | Settings and meeting detail | Preferred agent, content scope, context budget, approval, fallback prompt, run history, disable |

Consent must be informed and reversible. The first-run path can explain benefits, but it must not preselect or silently enable sensitive automation. If a permission is denied or later revoked, Meetily must show the affected feature as unavailable, keep existing local meeting data accessible, and provide a path to retry or disable the feature.

Revoking consent must stop future automation immediately. Derived artifacts created only for the revoked feature, such as screenshots, chat indexes, speaker labels, export records, or MCP client tokens, must be deleted or invalidated by default. Audit logs may keep minimal event metadata needed for accountability, but must not retain meeting content or prompts.

During a meeting, recording, transcription, screenshots, assisted join, and exports must expose immediate pause/stop/disable controls where the user is already working. Post-meeting views must disclose which sensitive automations ran, what artifacts were created, and where external writes were attempted.

Post-meeting agent orchestration stores local run metadata only: meeting id,
agent target, trigger mode, template id, context budget, content-source flags,
status, timestamps, user-safe messages, audit event type, and user-entered
outcome links. It must not persist raw transcript text, screenshot OCR, full
prompts, MCP authorization headers, provider secrets, or third-party auth
responses. Auto-trigger consent is consent to prepare or hand off a prompt, not
consent for GitHub, GitLab, Linear, Jira, repository, file, or message writes.
Those writes remain agent-owned and must follow the selected agent's approval
flow.

## Threat Model

Primary risks:

* Accidental capture of private conversations, screens, or unrelated app content.
* Silent export of meeting data to files, Apple Notes, calendars, or cloud LLM providers.
* Malicious or over-broad local clients reading meeting content through MCP.
* Stale derived artifacts, such as screenshots or chat indexes, surviving meeting deletion.
* Confusing speaker labels with verified identity.

Severity is assigned by the Critical, Important, and Minor tiers in [Security Review Checklist](security-review-checklist.md). Any primary risk that can expose meeting content, credentials, screenshots, exports, prompts, or tokens without consent is Critical until fixed or explicitly blocked from release.

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
| Calendar metadata | Event links remain until provider disconnect, calendar deselect, or meeting deletion; sync history keeps minimal status for 30 days by default | Meeting deletion removes only that meeting's event link; calendar deselect removes calendar mappings; provider disconnect removes provider tokens and sync cursors | Disconnect provider, deselect calendar, delete meeting, sync history retention window | Revoked calendar access stops sync and hides stale automation prompts |
| Apple Notes export records | Until user deletes export record, disconnects Notes automation, or deletes meeting | Remove export metadata and local status; do not delete Notes content without explicit confirmation | Disconnect, delete export record, retry failed export | Disconnect stops future writes and preserves visible prior-write status |
| Apple Reminders drafts and created-reminder links | Drafts remain until dismissed, created, regenerated, or the meeting is deleted; created links remain until provider disconnect, history deletion, or meeting deletion | Remove local drafts and link metadata; do not delete external reminders without explicit confirmation | Dismiss draft, create selected, disconnect, delete local history, delete meeting | Duplicate prevention works after retry; meeting deletion shows linked external reminders before any external modification |
| MCP client tokens | Until user revokes a client or disables MCP | Revoke token/trust record and terminate active sessions | Revoke client, disable MCP | Revoked clients cannot call tools without reauthorization |
| MCP access logs | Retain minimal metadata for 30 days by default | Delete expired logs automatically; meeting deletion redacts meeting title/content references while preserving minimal event accountability | Clear logs, retention window | Logs never contain transcript text, prompts, screenshots, or summary bodies |
| Agent skill installer logs | Retain minimal metadata for 30 days by default | Remove install/update logs and source references on clear; uninstall removes app-managed files | Uninstall skill, clear logs | Cleared logs cannot expose source paths or meeting content |
| Provider request metadata | Keep minimal provider name, timestamp, and status for 30 days by default | Remove request IDs, provider names, and timestamps tied to the meeting; never store raw prompts beyond generated local output | Settings > AI Providers: clear provider history, delete meeting, per-provider retention window | Cloud-provider summaries disclose provider use without retaining prompts |
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

## MCP Access Control and Audit Policy

The local Meetily MCP server must not expose meeting content or sensitive meeting metadata until the user explicitly enables MCP in Settings and authorizes at least one client. For MCP policy, content means transcripts, summaries, screenshots, prompts, embeddings, exports, and speaker labels. Sensitive metadata includes meeting titles, dates, statuses, file paths, and calendar links when a client could use them to infer private context.

Server rules:

* Bind only to loopback by default: `127.0.0.1` for IPv4 and `::1` for IPv6. Binding to any non-loopback interface is prohibited unless a future enterprise setting adds a separate explicit warning, access control review, and release gate waiver.
* Start disabled. The server may start only after the user enables MCP, and it must stop when MCP is disabled, the app exits, or the user revokes the last active client.
* Use an OS-assigned local port by default or a user-selected local port with conflict handling. The active port must be visible in Settings.
* Apply conservative per-client and per-tool rate limits to sensitive calls. Rate-limit denials must not include meeting content.
* Never print tokens, meeting content, transcript text, screenshots, summaries, prompts, or embeddings to stdout, logs, crash reports, or audit records.

Client authorization:

* Each client must complete an in-app authorization flow before tool access. The app must show the client name when available, requested permission scopes, expiration, and whether the client can read or write data.
* Client credentials expire after 30 days by default, with an optional shorter expiration. Renewal requires an in-app confirmation that repeats the requested scopes.
* Client trust is represented by a revocable random token or equivalent OS-protected credential. Tokens must be stored in app-managed secure storage where available. If secure storage is unavailable, MCP must either store encrypted credentials protected by OS user-scope file permissions or refuse to enable MCP; plaintext token storage is prohibited.
* Authorization is per client and per scope. Revoking a client terminates active sessions and invalidates the credential immediately.
* A global "Disable MCP" control must stop the server, revoke active sessions, and leave prior audit metadata visible until the configured retention window expires or the user clears logs.

Initial tool policy:

| Tool category | Initial status | Permission scope | Notes |
| --- | --- | --- | --- |
| Server health and capability discovery | Allowed after client authorization | `mcp:read_status` | No meeting content or file paths |
| List meeting IDs | Allowed after explicit list scope | `meetings:list_ids` | Return opaque IDs only |
| List meeting metadata | Disabled until explicit metadata scope | `meetings:list_metadata` | Return titles, dates, and status only after metadata consent |
| Read transcript or summary | Disabled until user grants meeting read scope | `meetings:read_content` | Client must request content access and logs must include meeting IDs |
| Search/chat over meeting index | Disabled until chat/index feature and MCP scope are enabled | `meetings:query_index` | Must not expose excluded meetings |
| Export or write meeting data | Disabled initially | `meetings:write` | Requires a later issue and separate security review |
| Delete or mutate meetings | Prohibited initially | None | Requires a future release gate and stronger confirmation model |

MCP audit logs must record minimal accountability metadata:

* Timestamp.
* Client ID or token fingerprint.
* Tool name and permission scope.
* Opaque meeting IDs accessed, when applicable. Audit records must not mirror meeting titles, dates, transcript snippets, or summary text.
* Result status, such as allowed, denied, revoked, failed, or rate-limited.
* Reason for denial when applicable.

MCP audit logs must not store prompts, transcript text, summary bodies, screenshots, embeddings, exported file content, or raw authorization tokens.

Implementation and QA issues for MCP must include tests for:

* Server disabled by default and not listening before enablement.
* Unauthorized clients receive no meeting content.
* Revoked clients cannot reuse old credentials.
* Per-tool scopes deny transcript, summary, query, export, and mutation calls without explicit permission.
* Audit logs record allowed and denied sensitive calls without content payloads.
* Server binds only to loopback by default.
* Port conflicts are reported without falling back to non-loopback binding.
* Non-loopback IPv4 and IPv6 bind attempts are rejected.
* Disabling MCP terminates in-flight sessions.
* Tokens, scopes beyond explicit permission names, meeting titles, transcript text, summaries, and prompts never appear in logs or UI beyond allowed token fingerprints and opaque IDs.

## External Data Boundary Policy

Meetily is local-first by default. Meeting data leaves app-managed local storage only after the user enables an integration, selects a destination, or chooses a cloud provider for summaries or chat.

| Integration | Data read | Data written | Local metadata retained | Required preview and controls |
| --- | --- | --- | --- | --- |
| Apple Calendar | Selected calendar event title, start/end time, meeting URL, calendar ID, and event ID | Meetily-owned event title, start/end time, summary status, transcript availability, and linked Notes metadata only after explicit event-creation opt-in | Provider/account label, selected calendar IDs, event IDs linked to meetings, Apple event identifiers for Meetily-created events, last sync status, sync history, revocation status | Calendar/account selector, selected calendars, target calendar, event-creation toggle, disconnect, last sync/status health |
| Google Calendar, if added | Selected calendar event title, start/end time, meeting URL, calendar ID, event ID, and provider sync cursor | No calendar writes unless explicitly implemented and consented | Provider/account label, selected calendar IDs, event IDs, sync cursor, last sync status, sync history, revocation status | Provider consent screen, calendar selector, disconnect, last sync status, sync history |
| Auto-detect and assisted join | Calendar metadata and meeting URLs from selected calendars | Join attempts only; no meeting content written externally | Meeting ID, event ID, join attempt timestamp, result status | Upcoming meeting preview, join confirmation, cancel, disable automation |
| PDF, DOCX, Markdown, or share exports | Selected meeting title, transcript, summary, speaker labels, timestamps, and selected template output | User-selected destination file or share target | Export type, destination path or target label, timestamp, result status, retry state | Destination preview, file name preview, export confirmation, reveal destination, retry failed export |
| Auto-export | Same as manual export for the selected template and format | User-configured destination only | Destination config, format, template, last export status, retry state | Explicit destination setup, sample destination preview, disable auto-export |
| Apple Notes automation | Selected meeting title, summary, transcript excerpt or configured template output | User-selected Notes account/folder/note | Notes account/folder label, note identifier if available, timestamp, result status | Notes destination preview, content preview, retry, disconnect |
| Apple Reminders follow-ups | Selected meeting action items, summary follow-ups, and short source evidence snippets | User-selected Reminders list after draft review | Provider account/list label, app-created reminder identifier, meeting id, draft id, dedupe key, creation/status timestamps, result status | Reminders list preview, editable reminder draft review, create selected only, retry failed creation, disconnect |
| Cloud LLM summaries or chat | The prompt payload required by the selected provider, including selected transcript/summary/context | Provider receives request payload and returns generated text | Provider name, timestamp, model where available, result status, generated output stored locally if user keeps it | Provider disclosure, local-provider recommendation, model selector, clear provider history, provider history retention window |
| MCP clients | Meeting metadata or content only within authorized scopes | Local response to authorized client; no external network by default | Client fingerprint, scopes, meeting IDs, tool name, result status | Enable MCP, authorize client, revoke client, audit log |
| Post-meeting agent orchestration | Approved summary, action items, bounded transcript excerpts, selected artifact links, and source ids | Handoff package to selected local agent or copyable prompt; no direct Meetily writes to GitHub/GitLab/Linear/Jira/repos | Run id, meeting id, agent, trigger mode, template id, context budget, content scopes, hashes, status, timestamps, user-safe errors, outcome links | Trigger mode, preferred agent, content scope, context preview/approval, fallback prompt, run history, disable |

External-boundary rules:

* Auto-export must remain disabled until the user selects a concrete destination, format, and template.
* Destination previews must appear before the first external write and whenever the configured destination changes.
* External write failures must be visible in-app with a retry path, destination details, and a safe failure state that does not mark the export complete.
* Permission revocation must stop future reads/writes immediately and mark dependent automation unavailable until the user reconnects or disables it.
* Local metadata must be limited to what is needed to show status, audit history, retry state, and revocation state.
* Cloud-provider summaries or chat must clearly state that selected meeting content is sent to the configured provider, even though Meetily stores its own meeting records locally.
* Cloud-provider settings must separate local Ollama from cloud providers. Ollama remains local by default. Anthropic, Groq, OpenAI, OpenRouter, and any OpenAI-compatible cloud endpoint must each expose provider-specific history retention, clear-history controls, and provider disclosure. If a provider offers a training-data opt-out or zero-retention mode, the setting or setup copy must surface it; Meetily must not imply it can control provider-side retention beyond the provider's own terms.
* External files, Notes content, calendar events, and provider-side logs may remain outside Meetily's deletion control. Meetily must disclose this before writing externally.

Implementation and QA issues for external integrations must include tests for:

* First export requires a destination preview and explicit confirmation.
* Auto-export cannot run without destination configuration.
* Failed or partially completed external writes remain visible and retryable.
* Revoked calendar or Notes permission stops future sync/export attempts.
* Clearing local export history does not silently delete user-managed external files.
* Cloud-provider mode shows provider disclosure before sending meeting content.

## Screenshot Capture Controls

Periodic screenshot capture is high risk because it can include unrelated apps, private tabs, notifications, credentials, or participants who did not expect screen capture. Screenshots must never start silently.

The implementation data model, runtime state machine, storage layout, and
speaker-label boundary rules are specified in
[Speaker Identification and Screenshots](speaker-identification-screenshots.md).
Release notes and the provider QA matrix are maintained in
[Call-Window Snapshot Capture Release Notes](call-window-snapshot-release-notes.md).
Call-window snapshot capture must prefer the narrower call-window-only contract
defined there. When fresh call-window bounds are unavailable, Meetily must skip
capture or show an explicit fallback warning; it must not silently capture the
full screen as a substitute.

Consent and startup:

* Screenshot capture is off by default.
* A global screenshot preference may only make the feature available. Every meeting still requires per-meeting confirmation before the first screenshot.
* The confirmation must show capture interval, storage location, retention behavior, deletion path, whether screenshots may be used for speaker labeling or meeting chat context, and a warning that screenshots may include visible screen content outside the meeting app.
* Call-window snapshot confirmation must name the detected provider/window scope
  and explain that missing or stale bounds skip capture instead of falling back
  to full-screen capture.
* If OS screen-recording permission is missing, revoked, or unavailable before capture starts, screenshots must remain disabled, the meeting must continue without screenshots, and the UI must show screenshots as unavailable.

Runtime controls:

* Show a persistent visible indicator whenever screenshot capture is active.
* Show the next scheduled capture time or countdown.
* Provide immediate pause, resume, and stop controls from the meeting surface.
* Pausing recording must also pause screenshot capture. Screenshots may resume only when the meeting recording resumes and the screenshot controls still show active capture.
* Stopping screenshot capture must stop future captures immediately without ending audio recording.
* If OS screen-recording permission is revoked during an active session, screenshot capture must stop immediately, the UI must show screenshots as unavailable, and existing screenshots remain subject to the meeting deletion and screenshot deletion controls.
* The user must be able to delete individual screenshots and all screenshots for a meeting.

Capture limits and storage:

* The default interval must not be more frequent than every 60 seconds.
* A user-configurable interval must enforce an absolute minimum of 30 seconds unless a future release gate approves a shorter interval. The default remains at least 60 seconds.
* Screenshots are stored only in app-managed local storage by default and linked to meeting ID, capture timestamp, source display/window when available, and deletion status.
* Call-window snapshots must also store provider, window title, bounds,
  recording time, relevance confidence, source trigger, redaction state, and any
  skip reason as metadata without storing OCR or transcript content in metadata.
* Screenshots inherit the meeting retention policy and must be deleted when the meeting is deleted.
* Redaction, if added, must run before screenshots are exposed to summaries, chat indexes, exports, or MCP clients.

Private and unsupported capture contexts:

* Meetily cannot reliably identify every private/incognito window or sensitive app. The confirmation and active indicator must state that screenshots may include visible screen content outside the meeting app.
* If the OS or target app blocks screen capture, the app must record a visible skipped-capture status instead of retrying aggressively or capturing a different source silently.
* Notifications or error states must not include screenshot thumbnails or captured text.

Speaker labeling:

* Screenshot-derived speaker labels are detected labels, not verified identities.
* Visual speaker evidence is limited to the allowed signal and evidence model in
  [Speaker Identification and Screenshots](speaker-identification-screenshots.md);
  face recognition and appearance-based identity inference are prohibited.
* The per-meeting prompt must say: "Allow Meetily to create detected speaker labels for this meeting?" and name the label source, such as screenshots or a future diarization model.
* The prompt must disclose that labels are local derived data, can be wrong, can be edited or confirmed by the user, and can be cleared without deleting the meeting.
* UI and exports must distinguish "detected speaker label" from "user-confirmed identity."
* Users must be able to edit, confirm, clear, and delete speaker labels independently from screenshots.
* Revoking speaker-label consent stops new labels and deletes detected labels by default. User-confirmed identities may be retained only if the user explicitly chooses to keep them.
* Speaker labels must not be sent to external exports, chat indexes, MCP clients, or cloud providers unless the relevant feature is enabled under the external data boundary rules and the destination preview includes speaker labels explicitly.

Implementation and QA issues for screenshots must include tests for:

* Screenshots do not start from a global preference alone.
* Per-meeting confirmation appears before the first capture.
* Active indicator, next-capture time, pause, resume, stop, and delete controls are visible during capture.
* Pausing recording pauses screenshots, and screenshots resume only after recording resumes with active screenshot state visible.
* Permission denial or revocation disables screenshots without breaking recording.
* Meeting deletion removes screenshot files and timeline references.
* Speaker labels clearly distinguish detected labels from user-confirmed identities.
