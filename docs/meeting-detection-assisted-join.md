# Meeting Detection and Assisted Join

Meetily meeting detection is an opt-in helper for finding upcoming meetings from
approved calendar metadata and preparing recording setup. It must never join,
record, or expose meeting data without user consent.

## Automation Modes

| Mode | Behavior | Default |
| --- | --- | --- |
| Disabled | No meeting candidates are shown or opened. Calendar metadata is ignored by the prompt layer. | Yes |
| Prompt only | Upcoming candidates are shown in the app. The user must click Open meeting or Start recording. | No |
| Auto-open | Meetily may open the meeting URL during the configured time window after explicit opt-in. Recording still requires a separate user action. | No |
| Auto-join | Reserved for a future release. It is not implemented because joining can enter a live call context. | No |

Auto-open is limited to opening the meeting link. It must not click through a
provider's join confirmation, start recording, or enable microphone/camera.

## Consent Copy

Settings must make this clear:

* Meeting detection reads only approved calendar metadata.
* Prompt-only is the recommended mode.
* Auto-open can launch a meeting URL but never starts recording.
* Start recording remains explicit.
* Detection can be disabled quickly from Settings or from a prompt.

## Event Matching Rules

The first implementation accepts provider-neutral calendar event metadata from
future calendar sync and a local approved-event form in Settings:

* event ID, calendar ID, optional calendar name;
* title, start and end time;
* attendees, if supplied by an approved calendar source;
* description, location, or explicit meeting URL.

Meetily extracts only Google Meet, Zoom, and Microsoft Teams URLs. Candidates are
shown when the event starts within the configured lookahead window or is already
active. Events older than the configured stale window are hidden. Duplicate
event/link combinations are suppressed, and dismissed candidates remain hidden
until the source event changes.

## Stored Data

Meetily stores local detection preferences, dismissed candidate IDs, auto-open
history, and approved event metadata supplied by the calendar integration layer.
It does not store transcript text, summaries, screenshots, provider tokens, or
meeting chat content for meeting detection.

## Implementation References

* `frontend/src/services/meetingDetectionService.ts` owns matching, filtering,
  URL extraction, dismissal, and safe open behavior.
* `frontend/src/components/MeetingDetectionPrompt.tsx` presents candidates and
  requires user action before opening links or starting recording.
* `frontend/src/components/PreferenceSettings.tsx` exposes detection mode and
  quiet hours, plus a local approved-event form while full calendar sync is not
  connected.
