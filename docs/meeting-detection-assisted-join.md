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
* Ambient detection is a local heuristic, not proof that another app owns the microphone.
* Window-aware detection reads local app, window, tab, microphone, and system-audio activity signals only after the user enables it.
* Call-window visual confirmation is separate from screenshot capture. Detection may report that a likely call window exists without saving any image.
* Detection can be disabled quickly from Settings or from a prompt.

## Detection v2 Provider Matrix

Window-aware detection v2 treats provider support as evidence quality, not as
permission to join or record. Every provider path must still end in a user
action before recording starts.

| Provider | Strong signals | Weaker signals | Permission-sensitive signals | Notes |
| --- | --- | --- | --- | --- |
| Google Meet in Chrome, Arc, Safari, Edge, or Firefox | Active tab URL on `meet.google.com`; active window title containing Google Meet; calendar event with Meet URL | Browser process running; visible Meet URL in window title | Browser tab inspection may require Automation permission; active window title may require Accessibility | Browser URL is the strongest non-visual signal where tab inspection is available. Firefox may degrade to window-title and process hints. |
| Zoom desktop app | Active Zoom meeting window title; Zoom app in focus with microphone/system audio activity; calendar event with Zoom URL | Zoom process running | Active window title and bounds require Accessibility; call-window visual confirmation may require Screen Recording | Process-only Zoom is weak because the app can stay open outside a call. |
| Zoom in browser | Active tab URL on `zoom.us` or `zoom.com`; active browser meeting tab title | Browser process running; visible Zoom link | Browser tab inspection may require Automation permission | Treat similarly to Google Meet but provider-specific URL parsing must handle join links. |
| Microsoft Teams desktop app | Active Teams meeting/call window; Teams app in focus with microphone/system audio activity; calendar event with Teams URL | Teams process running | Active window title and bounds require Accessibility; visual confirmation may require Screen Recording | Teams often runs in background, so process-only must not trigger prompts. |
| Microsoft Teams in browser | Active tab URL on `teams.microsoft.com` or `teams.live.com`; active tab title with Teams meeting/call | Browser process running | Browser tab inspection may require Automation permission | Use URL plus active-tab state as strong evidence. |
| Slack huddles/calls | Slack call/huddle window in focus; Slack window title with huddle/call terms plus mic/system audio activity | Slack process running | Active window title and bounds require Accessibility | Slack process-only is especially weak because Slack is commonly open all day. |
| Unknown or unsupported meeting app | Active window title with generic meeting/call language plus microphone/system audio activity | Process name only | Active window title and mic/system audio access | Prompt must explain low confidence and avoid provider-specific actions. |

## Signal Taxonomy

The scorer should keep signal types explicit so prompts can explain why a
meeting was detected and why a prompt did or did not appear.

| Signal | Strength | Privacy sensitivity | Prompt use |
| --- | --- | --- | --- |
| Calendar event with recognized meeting URL and current/near start time | Strong | Low, if calendar integration is already enabled | Can produce upcoming or active meeting prompts. |
| Active browser tab with recognized meeting URL | Strong | Medium, requires browser Automation on macOS | Can produce active-call prompts and safe Open meeting actions. |
| Active window title for recognized meeting provider | Strong | Medium, may require Accessibility | Can produce active-call prompts when paired with provider/app context. |
| Active app is a known meeting provider | Medium | Low to medium | Helps confirm provider, but should not be enough alone. |
| Running app/process is a known meeting provider | Weak | Low | Never enough alone; only a supporting signal. |
| Microphone activity from Meetily audio monitor | Medium | Medium, requires microphone permission | Supports an active-call prompt, but is not proof of a meeting. |
| System audio activity from selected/system device | Medium | Medium, depends on capture backend and OS permissions | Supports active-call scoring when paired with provider/window/calendar evidence. |
| Meeting link visible in title/text but not active tab | Medium | Low to medium | Useful for opening/join prep, not proof that the call is active. |
| Call-window visual confirmation without saved screenshot | Strong | High, may require Screen Recording | Confirms a visible call UI and can unlock call-window capture later. |
| Saved call-window snapshot | Strong | High, requires separate screenshot opt-in | Not required for detection v2 prompt eligibility. |

Process-only signals must never trigger recording prompts by themselves. A
process can keep running after a meeting, before a meeting, or all day in the
case of Teams and Slack.

## Confidence Threshold Contract

Detection v2 should return a structured score with:

* `confidence` from 0 to 100;
* `provider`, including `unknown` when provider is unresolved;
* `recommendedAction`: `none`, `open-meeting`, `start-recording`, or
  `review-setup`;
* `reasons`, ordered from strongest to weakest;
* `missingPermissions`, when signals could not be checked;
* `degradedMode`, when scoring ran without one or more permission-sensitive
  signals.

Recommended thresholds:

| Confidence | Meaning | Prompt behavior |
| --- | --- | --- |
| 0-39 | Weak or background-only evidence | No prompt. May show passive settings diagnostics only. |
| 40-64 | Possible meeting | No recording prompt. May show setup/review prompt if the user is already in detection settings. |
| 65-79 | Likely meeting | Prompt-only candidate can appear. Recommended action is usually Open meeting or Review setup. |
| 80-100 | Active call | Prompt may recommend Start recording, but recording still requires explicit user action. |

An active-call prompt should normally require at least one strong provider signal
plus either audio activity or calendar/current-time context. A strong browser URL
or active meeting window alone can show a meeting prompt, but it should not claim
that audio is active.

## macOS Permissions and Degraded Modes

| Capability | macOS permission likely needed | Degraded behavior when unavailable |
| --- | --- | --- |
| Active app/process list | Usually none for process names; app focus details may vary | Fall back to running app hints and calendar/browser data. |
| Active window title and bounds | Accessibility | Do not infer call-window bounds. Report `missingPermissions: [accessibility]`. |
| Browser active tab URL/title | Automation for the specific browser | Use window title and process hints only. Report browser automation limitation. |
| Microphone activity | Microphone | Score without mic activity and explain that audio activity could not be checked. |
| System audio activity | Audio capture backend permissions/configuration | Score without system audio activity and avoid active-audio claims. |
| Call-window visual confirmation | Screen Recording, and often Accessibility for bounds | Do not save images. Report that visual confirmation is unavailable. |

Degraded-mode prompts must be conservative. If only weak signals are available
because permissions are missing, Meetily should show setup guidance rather than
an active recording prompt.

## Event Matching Rules

The first implementation accepts provider-neutral calendar event metadata from
[Calendar Integration](calendar-integration.md) and a local approved-event form
in Settings:

* event ID, calendar ID, optional calendar name;
* title, start and end time;
* attendees, if supplied by an approved calendar source;
* description, location, or explicit meeting URL.

The native calendar integration extracts Google Meet, Zoom, and Microsoft Teams
URLs. Candidates are shown when the event starts within the configured lookahead
window or is already active. Events older than the configured stale window are
hidden. Duplicate event/link combinations are suppressed, and dismissed
candidates remain hidden until the source event changes.

## Ambient Signal Detection

When enabled, Meetily can also detect likely live meetings without calendar
metadata by combining local-only signals:

* known meeting apps or browsers running, including Teams, Zoom, Google Chrome,
  Arc, Safari, Edge, Firefox, and Slack;
* the active app and active window title, where the OS allows access;
* active browser tab title and URL for Google Meet, Zoom, Microsoft Teams, or
  Slack huddle links;
* optional microphone input activity from Meetily's existing audio level monitor;
* optional system audio activity when available from the recording backend;
* optional call-window visual confirmation when a separate visual-detection
  capability is enabled.

These signals are scored together and only produce a prompt when confidence
passes the configured threshold. Process-only signals are intentionally weak and
do not trigger prompts by themselves. If an ambient candidate has no meeting URL,
Meetily offers Start recording but disables Open meeting.

On macOS, active window and browser tab inspection may require Accessibility or
Automation permissions. If those permissions are unavailable, Meetily falls back
to weaker process and microphone signals and may not show a prompt.

## Stored Data

Meetily stores local detection preferences, dismissed candidate IDs, auto-open
history, and approved event metadata supplied by the calendar integration layer.
Base meeting detection does not store transcript text, summaries, screenshots,
provider tokens, or meeting chat content.

If the user separately opts in to call-window snapshot capture, screenshots are
stored under the screenshot/call-window capture feature, not as required
detection state. Detection may reference snapshot-derived metadata such as
recording time, provider, relevance confidence, or visual-confirmation status,
but image payloads remain governed by the screenshot retention, review, and
deletion rules.

## Implementation References

* `frontend/src/services/meetingDetectionService.ts` owns matching, filtering,
  URL extraction, dismissal, ambient candidate creation, and safe open behavior.
* `frontend/src/services/meetingDetectionSignals.ts` owns local ambient signal
  scoring.
* `frontend/src-tauri/src/meeting_detection.rs` collects read-only app, window,
  and browser signals for the desktop shell.
* `frontend/src/components/MeetingDetectionPrompt.tsx` presents candidates and
  requires user action before opening links or starting recording.
* `frontend/src/components/PreferenceSettings.tsx` exposes detection mode and
  quiet hours, ambient signal controls, plus a local approved-event form while
  full calendar sync is not connected.

## Prompt UI QA Matrix

Use these cases when validating the recording prompt manually:

| Scenario | Expected prompt |
| --- | --- |
| High-confidence active call | Shows provider, title/window title, confidence, signal reasons, and `Recommended: Start recording`. Recording remains a button click. |
| Calendar-only upcoming call | Shows calendar source, provider, meeting title, confidence, and `Recommended: Open meeting`. Auto-open may open only the meeting URL when enabled. |
| Low-confidence or process-only signals | No recording prompt should appear. If surfaced in setup diagnostics, action should be `Review setup`, not `Start recording`. |
| Permission-limited state | Shows missing permission guidance, signal reasons, and a `Review setup` action. Prompt must stay conservative until stronger signals are available. |
