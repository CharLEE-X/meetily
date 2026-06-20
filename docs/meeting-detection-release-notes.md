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
* Window-aware meeting detection v2 for local app/window, browser tab, microphone,
  system audio, and permission-limited signals.
* Confidence scoring with user-facing reasons, missing permission guidance, and
  recommended actions.
* Slack huddle/call recognition from Slack-qualified window or URL metadata.

## Privacy and Consent

Meeting detection is disabled by default. Prompt-only is the recommended mode.
Auto-open only launches the meeting URL after opt-in; it does not click provider
join buttons, start recording, or enable microphone/camera. Recording remains a
separate user action.

The first implementation reads provider-neutral event metadata supplied by an
approved calendar integration layer or the local approved-event form. It stores only local
detection settings, approved event metadata, dismissed candidate IDs, and
auto-open history.

Window-aware detection reads local app, window, tab, microphone, and system audio
activity signals only after the user enables the feature. It does not capture or
store transcript text, summaries, screenshots, provider tokens, or meeting chat
content for meeting detection.

## Supported Providers

Calendar URL extraction:

* Google Meet: `meet.google.com`
* Zoom: `zoom.us`, `zoom.com`
* Microsoft Teams: `teams.microsoft.com`

Window and browser signal recognition:

* Google Meet in Chrome, Arc, Safari, Edge, or Firefox.
* Zoom desktop app and browser meetings on `zoom.us` or `zoom.com`.
* Microsoft Teams desktop app and browser meetings on `teams.microsoft.com` or
  `teams.live.com`.
* Slack huddles/calls from Slack-qualified huddle/call window metadata or
  `slack.com/huddle` browser text.

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
| Google Meet in Chrome, Arc, or Safari | Active meeting tab/window produces a provider-labelled prompt with reasons. Browser Automation limits are reported when tab URL/title cannot be read. |
| Zoom desktop app | Active Zoom meeting window plus audio activity can recommend Start recording; Zoom process-only does not trigger a recording prompt. |
| Zoom browser meeting | Active `zoom.us` or `zoom.com` browser meeting tab can recommend Open meeting or Start recording depending on audio/context. |
| Microsoft Teams desktop app | Active Teams meeting/call window plus audio activity can recommend Start recording; Teams process-only remains below prompt threshold. |
| Microsoft Teams browser meeting | Active `teams.microsoft.com` or `teams.live.com` browser meeting tab can recommend Open meeting and shows browser permission limits when unavailable. |
| Slack huddle/call | Slack-qualified huddle/call window plus audio activity can recommend Start recording; bare Slack process or generic `huddle` text is ignored. |
| False positive: meeting app open all day | Running Teams, Zoom, browser, or Slack without strong window/tab/calendar evidence does not show a recording prompt. |
| Dismissed prompt | Dismissed candidate remains hidden until the event/source changes. |
| Permission-limited detection | Prompt shows missing permission guidance and Review setup; weak signals stay conservative. |

## Troubleshooting

* Disable detection from Settings > Meeting detection by selecting Disabled, or
  use Disable detection from a prompt.
* If browser meetings are not detected, review browser Automation permission and
  make sure the meeting tab is active.
* If active call windows are not detected, review macOS Accessibility permission.
* If confidence is lower than expected, verify microphone/system audio setup and
  use Prompt only while adjusting permissions.
