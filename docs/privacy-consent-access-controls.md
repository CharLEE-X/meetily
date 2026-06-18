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

Revoking consent must stop future automation immediately. Derived artifacts created only for the revoked feature, such as screenshots, chat indexes, speaker labels, export records, or MCP client tokens, must be deleted or invalidated by default unless the user explicitly chooses to retain them. Audit logs may keep minimal event metadata needed for accountability, but must not retain meeting content or prompts.

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
