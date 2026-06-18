# Meeting Detection Release Notes

## Added

* Opt-in meeting detection preferences in Settings.
* Local approved-event entry while full calendar sync is not connected.
* Prompt-only and auto-open modes for approved calendar event metadata.
* Google Meet, Zoom, and Microsoft Teams link extraction from event URL,
  location, or description fields.
* Upcoming meeting prompt with explicit Open meeting, Start recording, Dismiss,
  and Disable detection actions.
* Recording title prefill from selected calendar event title.
* Quiet hours, lookahead window, stale-event filtering, duplicate suppression,
  dismissed prompt tracking, and auto-open history.

## Privacy and Consent

Meeting detection is disabled by default. Prompt-only is the recommended mode.
Auto-open only launches the meeting URL after opt-in; it does not click provider
join buttons, start recording, or enable microphone/camera. Recording remains a
separate user action.

The first implementation reads provider-neutral event metadata supplied by an
approved calendar integration layer or the local approved-event form. It stores only local
detection settings, approved event metadata, dismissed candidate IDs, and
auto-open history.

## Supported Providers

* Google Meet: `meet.google.com`
* Zoom: `zoom.us`, `zoom.com`
* Microsoft Teams: `teams.microsoft.com`, `teams.live.com`

Other URLs are ignored until explicitly supported.

## QA Matrix

| Scenario | Expected result |
| --- | --- |
| No approved calendar events | No prompt appears. |
| One upcoming event with supported URL | Prompt appears within the lookahead window. |
| Overlapping events with duplicate link | One prompt per unique event/link combination. |
| Stale event after configured grace period | Prompt is hidden. |
| Detection disabled | No candidates are shown or opened. |
| Prompt-only mode | No URL opens until the user clicks Open meeting. |
| Auto-open mode | Link opens once per candidate; recording still requires Start recording. |
| Quiet hours enabled and current time inside window | Prompts are hidden. |
