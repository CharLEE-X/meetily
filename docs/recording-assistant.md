# Recording Assistant

Meetily's recording assistant keeps meeting capture explicit and reviewable. It
does not start external writes in Apple Notes, Apple Reminders, Calendar, Codex,
Claude, or other agents without a user review step.

## Preflight

Before recording, the Home screen shows a preflight panel with:

* meeting context: detected call, selected calendar event, or manual recording;
* audio readiness: microphone and system audio state;
* screenshot scope: off, detected call window, full screen with warning,
  interval only, speech-event assisted, or manual only;
* speaker labeling state: automatic high-confidence suggestions or review-first;
* missing setup warnings.

Start recording remains a single explicit button. Review settings opens the
relevant settings pages when you want to change sensitive capture scope before
recording.

## Runtime Controls

While recording, the recording controls show:

* audio recording state and duration;
* screenshot state: off, active, paused, stopped, or manual-only;
* next scheduled screenshot time when the backend has one;
* speaker labeling state;
* nonfatal screenshot warnings, such as missing call-window permissions.

Screenshot capture can be paused, resumed, or stopped without stopping audio
recording. If the call window cannot be captured, Meetily keeps audio recording
running and shows a warning instead of silently falling back to broader capture.

## Post-Recording Review

After recording, the meeting detail page can show a post-recording checklist
when there is context to review. It guides the user through:

* screenshots: review, remove image payloads, or delete metadata;
* speaker labels: confirm, rename, merge, assign, clear, or undo suggestions;
* summary context: check context before exports or agent handoffs;
* Apple Calendar: manually create or update Meetily-owned records;
* Apple Notes: preview destination and content before export;
* Apple Reminders: edit and select drafts before creating reminders;
* agents: inspect or intentionally trigger Codex/Claude handoffs.

Checklist progress is stored locally per meeting. Skipping the checklist only
hides the local checklist for that meeting; it does not delete meeting data or
perform external actions.

## Audit History

Recording privacy history is visible in Recording settings and meeting details.
Audit events store event type, meeting id when available, timestamp, actor, and
safe metadata such as capture mode or enabled state.

Audit history must not store transcript text, OCR text, screenshot image
payloads, raw tokens, private calendar descriptions, attendees, account
identifiers, device names, paths, URLs, prompts, or meeting content.

## Disabling Sensitive Features

Use Settings > Recordings to disable screenshot capture and automatic visual
speaker labels. Use Settings > Calendar, Notes, Reminders, and MCP to disconnect
or disable destination-specific review flows. Audio recording remains available
when sensitive context features are off.
