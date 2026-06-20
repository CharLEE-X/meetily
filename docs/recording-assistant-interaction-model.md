# Recording Assistant Interaction Model

Version: 1
Last updated: 2026-06-20
Owner: CHA-1819

This document defines the consent copy and interaction model for the recording
assistant. It was created for CHA-1819 and acts as the product contract for
CHA-1815 implementation children:

* CHA-1822: preflight panel.
* CHA-1827: privacy audit events.
* CHA-1831: runtime indicators and capture controls.
* CHA-1835: post-recording review checklist.
* CHA-1838: QA, privacy docs, and release notes.

## Principles

* Fast path first: a user who understands the capture scope can start recording
  without stepping through a wizard.
* Reviewable by default: sensitive capture features show state, scope, and a
  clear pause/stop path before and during recording.
* No hidden writes: Notes, Reminders, and agent automation are prepared for
  review and never write externally without explicit user action. Calendar
  access is read-only in this epic; Meetily may read selected event metadata but
  must not write back to the calendar provider.
* Local-first evidence: screenshots and speaker evidence stay local unless a
  destination has its own consent and preview.

## Preflight States

| State | When shown | Primary copy | Required actions |
| --- | --- | --- | --- |
| Detected call | A supported meeting app/window or browser call is active with usable bounds. | "Meetily found an active call window and can record audio now." | Show provider, window title, audio devices, screenshot scope, speaker labeling state, and missing permissions. |
| Possible call | Meeting signals exist, but bounds, provider, or audio activity is uncertain. | "This looks like a meeting. Start audio-only now, or confirm the call window before using meeting context." | Let the user start audio-only or confirm the detected window manually. |
| Scheduled meeting | Calendar metadata matches the current time, but no active call window is confirmed. | "A calendar meeting is starting. Start recording when you join the call." | Show calendar title/time and audio devices; screenshots remain off until a call window is detected or chosen. |
| Manual recording | No meeting signal is available or the user starts from the main record button. | "Start a manual recording. Optional context can be reviewed before enabling it." | Audio recording can start immediately; context features remain explicit toggles. |

## Sensitive Feature Copy

| Feature | Enabled copy | Boundary copy | Stop/clear copy |
| --- | --- | --- | --- |
| Screenshots | "Capture local snapshots of the detected call window for timeline review." | "Meetily skips capture if the call window is not clearly detected. It will not silently fall back to full screen." | "Pause screenshots now" and "Delete captured images after recording." |
| Speaker labeling | "Suggest transcript speaker labels from audio timing and local call-window display names." | "Labels are suggestions until you confirm or edit them. No face recognition is used. This follows the CHA-1814/CHA-1818 visual evidence contract." | "Clear generated labels" keeps transcript text and confirmed/manual labels. |
| Calendar metadata | "Use the selected calendar event for the meeting title and prompt context." | "Calendar access is read-only. Meetily does not write back to your calendar." | "Disconnect calendar context for this recording." |
| Notes and Reminders | "Offer post-meeting exports and follow-up reminders for review." | "Meetily prepares drafts; Apple Notes and Reminders writes require your confirmation." | "Skip this step" leaves the meeting unchanged. |
| Agent automation | "Prepare a post-meeting package for Codex or Claude." | "The agent sees only the package you preview. External tools are triggered by the agent, not hidden app actions." | "Do not run automation" and "Rerun manually from the meeting." |

## Quick Start Flow

1. Show a compact preflight summary above the main recording controls.
2. Default to audio recording only plus already-confirmed safe metadata.
3. Keep screenshots, speaker labels, and automations as visible toggles with
   one-line scope text.
4. If permissions are missing, allow audio-only recording and show the missing
   permission in the runtime status.
5. Start recording with the current scope when the user clicks Record.

Sensitive feature toggles changed during an active recording take effect
immediately for future capture/automation in that recording. They do not delete
previously captured local evidence unless the user chooses a delete/clear
action.

## Expanded Privacy Review Flow

The user can open "Review capture scope" from preflight or runtime. The expanded
view shows:

* Detected meeting source and confidence.
* Active call window/provider and whether bounds are fresh.
* Microphone and system audio devices.
* Screenshot mode, next capture trigger, and retention choice.
* Speaker labeling source and auto-apply setting.
* Calendar event used for title/prompt context.
* Post-meeting destinations that may be offered later.
* Audit history for sensitive toggles in this recording.

No expanded review item blocks audio recording unless the user enabled a
sensitive feature that cannot run safely.

## Runtime Indicators and Controls

| Indicator | Meaning | Control |
| --- | --- | --- |
| Recording | Audio capture is active, paused, or stopping. | Existing pause/stop controls remain primary. |
| Screenshots | Disabled, active, paused, skipped, permission missing, or failed. | Pause/resume screenshots and delete captured images. |
| Call window | Detected, stale, unsupported, or missing permission. | Re-detect or switch to audio-only. |
| Speaker labels | Off, suggested, auto-applied, low-confidence, or needs review. | Turn off future suggestions or clear generated labels. |
| Calendar | Selected, unavailable, or disconnected. | Disconnect context for this recording. |
| Automation | Not configured, prepared, pending review, or skipped. | Disable automation for this recording. |

Emergency controls are a required subset of the runtime status surface, not a
separate wizard or duplicate panel. The compact runtime surface must always keep
these actions visible while recording:

* Pause screenshots.
* Stop all sensitive context capture.
* Continue audio-only.
* Delete captured screenshots.

## Post-Recording Review Checklist

The checklist appears after recording stops and can be skipped entirely. Each
step must be individually skippable.

| Step | Purpose | Skippable |
| --- | --- | --- |
| Review screenshots | Inspect kept/skipped/deleted snapshots and remove image payloads. | Yes |
| Confirm speaker labels | Accept, rename, merge, assign, clear, or undo generated labels. | Yes |
| Add summary context | Add typed or voice context before regenerating the summary. | Yes |
| Review calendar link | Confirm the meeting title/event link used in exports. | Yes |
| Notes export | Preview Apple Notes export and linked meeting assets. | Yes |
| Reminders | Review detected follow-ups before creating reminders. | Yes |
| Agent automation | Preview the Codex/Claude package and trigger or skip automation. | Yes |

The checklist records local audit events for sensitive choices but does not
perform external writes until the user confirms the specific destination.

## Audit Event Vocabulary

Sensitive assistant actions should use stable event names:

* `recording_preflight_shown`
* `recording_started_with_scope`
* `screenshot_capture_enabled`
* `screenshot_capture_paused`
* `screenshot_images_deleted`
* `speaker_labeling_enabled`
* `speaker_labels_cleared`
* `calendar_context_attached`
* `calendar_context_detached`
* `notes_export_reviewed`
* `reminders_reviewed`
* `agent_automation_reviewed`
* `agent_automation_disabled`
* `sensitive_capture_stopped`

Audit payloads should include local IDs, state, timestamps, and user-selected
scope only. They must not include transcript body text, OCR text, screenshot
image payloads, window titles, app titles, calendar event titles, attendees,
device names, account identifiers, or external account secrets.
