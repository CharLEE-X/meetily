# Recording Assistant Release Notes

Meetily now includes a review-first recording assistant for privacy-sensitive
meeting context.

## Highlights

* Preflight panel summarizes meeting context, audio readiness, screenshot scope,
  speaker-labeling state, and missing setup before recording.
* Runtime recording controls show screenshot capture state, speaker-labeling
  mode, next capture timing when available, and nonfatal capture warnings.
* Screenshot capture can be paused, resumed, or stopped without stopping audio.
* Local audit history records sensitive recording decisions without storing
  transcript text, screenshots, OCR text, tokens, private calendar details, or
  other meeting content.
* Meeting details include a post-recording checklist for screenshots, speaker
  labels, summary context, Calendar, Apple Notes, Apple Reminders, and agents.

## Privacy Contract

Meetily does not perform external writes without review. Apple Notes exports,
Apple Reminders creation, Apple Calendar event creation, and Codex/Claude
handoffs all require a destination-specific review or explicit trigger. The
post-recording checklist guides these steps but does not write externally on its
own.

## Disable Controls

Screenshot capture and automatic visual speaker-label application can be
disabled from Settings > Recordings. Calendar, Notes, Reminders, and MCP/agent
flows can be disconnected or disabled from their settings pages.
