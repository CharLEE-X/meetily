# Call-Window Snapshot Capture Release Notes

This release adds opt-in call-window snapshot capture for meeting recordings.
Snapshots are local, frame-based context images for timeline review, speaker
label correction, and future meeting grounding. They are not continuous video
recordings.

## Added

* Screenshot settings for disabled, interval, speech-event, and manual-only
  capture modes.
* Default call-window capture that targets the detected meeting window instead
  of the full screen.
* macOS window detection for supported meeting surfaces, including Google Meet,
  Zoom, Microsoft Teams, Slack huddles/calls, FaceTime, Webex, and browser
  meeting tabs where window metadata is available.
* Relevance filtering that keeps supported meeting frames and stores skipped
  metadata for unsupported, sensitive, low-confidence, missing-permission, or
  failed capture attempts.
* Timeline cards that distinguish kept, skipped, failed, and image-deleted
  snapshots.
* Per-snapshot deletion controls for removing only the image while keeping safe
  tombstone metadata, or removing the metadata row as well.
* Local speaker-evidence badges when a kept snapshot contributed visible-name
  context to speaker labeling.

## Privacy and Consent

Snapshot capture is disabled by default. Enabling the global setting only makes
the feature available; each meeting still requires confirmation before capture
starts.

Call-window mode captures only the detected call window when fresh, supported
window bounds are available. If bounds are missing, stale, too small, tied to an
unsupported app, or blocked by macOS permissions, Meetily records a skipped or
failed timeline entry instead of silently falling back to full-screen capture.

Snapshots stay in app-managed local storage. Screenshot metadata is local and
contains provider, trigger, recording time, relevance status, and deletion state.
Skipped and image-deleted rows do not retain OCR text. Image-only deletion
removes the image file, clears file paths, scrubs OCR names/text and window-title
free text from metadata, and removes the row from downstream speaker/chat
evidence.

Meeting chat, exports, MCP tools, cloud providers, and post-meeting agent
handoffs must not include snapshot images or OCR-derived facts unless that
destination has its own explicit content-inclusion consent and preview.

## QA Matrix

| Area | Scenario | Expected result | Evidence |
| --- | --- | --- | --- |
| Google Meet browser | Focus an active Meet window in Chrome, Arc, Safari, Edge, or Firefox with call-window snapshots enabled. | Kept snapshot is cropped to the meeting window, provider is Google Meet, trigger and recording time are shown. | Code-path covered by provider detection and screenshot analysis tests; manual provider pass required before external release. |
| Zoom app | Focus an active Zoom meeting window. | Kept snapshot is cropped to the Zoom window; if bounds are unavailable, capture is skipped with no full-screen fallback. | Detection path supports Zoom app/window metadata; manual provider pass required before external release. |
| Zoom browser | Focus a `zoom.us` or `zoom.com` meeting tab. | Browser meeting window is treated as Zoom when window metadata indicates a meeting. | Detection path supports Zoom browser metadata; manual provider pass required before external release. |
| Teams app | Focus an active Teams meeting/call window. | Kept snapshot is cropped to Teams; process-only Teams presence does not force capture. | Detection path supports Teams app/window metadata; manual provider pass required before external release. |
| Teams browser | Focus a `teams.microsoft.com` or `teams.live.com` meeting tab. | Browser meeting window is treated as Teams when metadata indicates a meeting. | Detection path supports Teams browser metadata; manual provider pass required before external release. |
| Slack huddle | Focus a Slack huddle/call window. | Kept snapshot is allowed only for Slack-qualified huddle/call metadata. | Detection path supports Slack huddle/call metadata; manual provider pass required before external release. |
| FaceTime | Focus an active FaceTime call window. | Kept snapshot is allowed only when FaceTime call-window metadata is active and bounds are usable. | Detection path supports FaceTime metadata; manual provider pass required before external release. |
| Webex | Focus an active Webex meeting window or browser meeting. | Kept snapshot is allowed only when Webex meeting metadata is active and bounds are usable. | Detection path supports Webex metadata; manual provider pass required before external release. |
| Unsupported window | Focus Finder, Apple Developer, docs, or a non-call browser page. | Capture is skipped and no image file is retained. | `cargo test screenshots --lib` covers unsupported and sensitive-frame skips. |
| Permission denied | Remove or deny macOS Accessibility or Screen Recording permission. | Recording can continue; screenshot capture records unavailable/skipped/failed state and does not capture full screen. | Code path returns permission-specific skip/fail reasons; manual macOS permission pass required before release. |
| Bounds unavailable | Supported provider is present but active-window bounds or window id are missing. | Capture is skipped with visible reason; no image placeholder file is created. | Code path stores skipped metadata for build-plan failures. |
| Paused recording | Pause recording while screenshots are active. | Screenshot scheduler pauses and resumes only when recording resumes. | `cargo test screenshots --lib` covers scheduler pause/rate-limit behavior. |
| Long meeting limits | Leave interval/speech-event capture running through many triggers. | Scheduler enforces minimum trigger gaps and maximum snapshots per meeting. | `cargo test screenshots --lib` covers rate limits and max-capture stop behavior. |
| Deletion | Remove image only, then remove metadata. | Image-only deletion removes files, scrubs free text, and removes downstream evidence; metadata deletion removes the row. | `cargo test screenshots --lib` covers metadata scrubbing; code review verified downstream filters. |

## Operational Notes

* Recommended default: call-window target, interval mode, 60 seconds.
* Minimum interval: 30 seconds.
* Maximum snapshots per meeting: 240.
* Full-screen capture remains available only as an explicit user-selected
  setting and should be treated as higher risk.
* Before any external release build, run a manual provider pass for Google Meet,
  Zoom, Teams, Slack, FaceTime, and Webex on the target macOS version and attach
  results to the release checklist.
