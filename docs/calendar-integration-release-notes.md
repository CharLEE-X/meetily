# Calendar Integration Release Notes

## Added

* Calendar tab in Settings for Apple Calendar on macOS.
* Local calendar provider tables for accounts, sources, events, and meeting links.
* Apple Calendar sync into a local upcoming-event cache.
* Meeting URL extraction for Google Meet, Zoom, and Microsoft Teams links.
* Upcoming meetings list with sync status, last sync, and permission/error state.
* Event selection for the next manual recording, including title and metadata
  prefill through the existing meeting detection metadata path.
* Explicit Apple Calendar event creation settings with target calendar and
  default-off write opt-in.
* Automation health checks for connection, cached events, event creation, last
  sync, and errors.

## Provider Limitations

* Apple Calendar is the only enabled provider in this release.
* The first Apple Calendar slice uses the local macOS Calendar automation bridge
  for event metadata and Meetily-owned event creation. A later hardening pass
  should move event reads/writes to EventKit before broad distribution.
* ICS and Google Calendar are visible in the provider model but remain planned.
* Calendar sync is user-triggered from Settings. App-start and periodic
  background sync remain follow-up work.
* Calendar source selection is represented in the database model, but the first
  UI slice syncs the Apple Calendar account as a single local source.

## Privacy and Consent

Calendar integration is off until the user connects a provider in Settings.
Synced event data stays local in the app database and the meeting detection
local candidate store. The app stores only minimal event metadata used for
prompts and recording setup: title, start/end time, local source identifiers,
event location, supported meeting URL, provider label, and sanitized short
description context.

Meetily does not expose calendar metadata to MCP tools or cloud AI providers in
this release. Apple Notes and Apple Calendar linking requires separate opt-in for
both destinations before metadata is shared between them.

Disconnecting Apple Calendar revokes cached prompts and marks cached calendar
events as revoked. It does not delete or modify external Apple Calendar events.

## QA Matrix

| Scenario | Expected result |
| --- | --- |
| Open Settings before connecting a provider | Calendar tab shows Apple Calendar as not connected and no upcoming events. |
| Connect Apple Calendar | Provider row is created locally and Settings shows permission-needed/ready state without blocking app startup. |
| Sync with macOS Calendar permission granted | Upcoming event cache updates, supported meeting URLs are extracted, and events appear in Settings. |
| Sync with permission denied or unavailable | Error appears in Settings; recording, transcription, summaries, and app startup still work. |
| No upcoming events | Upcoming list shows an empty state and no meeting prompt is generated. |
| Overlapping events | Multiple upcoming events remain visible and sorted by start time; duplicate meeting prompts are still deduplicated by existing meeting detection rules. |
| Select an upcoming event | Event card shows selected state and the next manual recording uses the event title and metadata. |
| Unselect an event | Selected state clears and the next manual recording returns to generated title behavior unless a prompt candidate is used. |
| Enable event creation | Settings saves the target calendar and write opt-in separately from read sync. |
| Create a Calendar event from meeting details | A completed meeting with a summary creates a Meetily-owned event and stores the Apple event identifier locally. |
| Create the same event again | The existing event is updated instead of duplicated. |
| Create an event after Apple Notes export | Event notes include Apple Notes export metadata. |
| Disconnect provider | Calendar prompts are cleared, cached events are revoked locally, and non-calendar/local approved meeting candidates are preserved. |
| Offline or sync failure after prior sync | Existing non-stale local cache remains available until disconnect or stale-window filtering hides prompts. |
| MCP client reads meetings | Calendar metadata is not exposed through MCP in this release; future MCP calendar access requires a separate scope and Settings toggle. |

## Manual Verification Notes

The repository checks completed for this implementation:

* `cargo test --manifest-path frontend/src-tauri/Cargo.toml calendar::tests --lib`
* `pnpm build`
* `pnpm tauri build --no-bundle`

`pnpm run lint` currently opens the first-run Next.js ESLint setup prompt in
this repository. QA reviewers should use the checks above as the
non-interactive gates until an ESLint config lands.
