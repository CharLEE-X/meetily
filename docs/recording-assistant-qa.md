# Recording Assistant QA Evidence

Issue: CHA-1838
Date: 2026-06-20

## Automated Checks

| Check | Result | Evidence |
| --- | --- | --- |
| Recording audit redaction tests | Pass | `pnpm run test:recording-audit` |
| Post-recording checklist persistence tests | Pass | `pnpm run test:post-recording-checklist` |
| Screenshot backend regression tests | Pass | `cargo test screenshots --lib` |
| Tauri compile check | Pass | `cargo check` |
| Whitespace check | Pass | `git diff --check` |
| TypeScript check | Baseline fail only | Existing `.ts` test import issues and `bun:test` type gap remain outside CHA-1815 scope. |

## Flow Matrix

The flows below are covered by implementation review and targeted checks. They
remain the manual QA matrix to run during app-level validation.

| Flow | Expected behavior | Evidence type |
| --- | --- | --- |
| Detected call | Preflight shows detected/scheduled/manual context, screenshot scope, speaker setting, audio devices, and missing setup warnings. | Code review: `frontend/src/app/page.tsx` and related services. |
| Manual recording | Start recording remains explicit and records `recording_started_with_scope` with manual source. | Code review: `useRecordingStart` audit event path. |
| Missing permissions | Preflight and runtime surfaces show nonfatal warnings while audio can continue when possible. | Code review: preflight missing setup copy and screenshot runtime `lastError` display. |
| Screenshots enabled | Runtime shows active/paused/stopped state and allows pause, resume, or stop without stopping audio. | Code review and tests: `RecordingControls`, `get_meeting_screenshot_capture_status`, `cargo test screenshots --lib`. |
| Speaker labels enabled | Runtime shows speaker label mode; meeting details expose suggestion review, accept, rename, merge, assign, clear, and undo controls. | Code review: `SpeakerScreenshotPanel` and runtime speaker preference display. |
| All sensitive features off | Audio recording remains available, screenshot status reads off, speaker labels stay review/off according to settings, and external panels require explicit action. | Code review: settings defaults, preflight copy, and review-first destination panels. |

## Audit Payload Verification

Audit history is implemented in `frontend/src/services/recordingAuditService.ts`
with an allow-list and sensitive-key deny pattern. Tests verify that sensitive
payload-shaped metadata is omitted for:

* transcript text;
* screenshot image payloads;
* raw tokens and secrets;
* calendar descriptions;
* private meeting titles;
* attendee emails;
* device names.

The audit UI displays only sanitized metadata and never renders raw meeting
content from audit events.

## Manual QA Steps To Run

Use these steps for app-level manual validation:

1. Disable screenshots and auto speaker labels, start a manual recording, stop,
   and confirm no screenshot controls appear.
2. Enable call-window screenshots, start recording, use pause/resume/stop shots,
   and confirm audio recording continues.
3. Record with a visible meeting window, stop, open meeting details, review
   screenshot timeline and speaker suggestions, then delete/confirm items.
4. Generate a summary and confirm the post-recording checklist appears with
   Calendar, Notes, Reminders, and agent steps.
5. Preview/export/create only after pressing the destination-specific buttons.

No external write should occur from the checklist itself.
