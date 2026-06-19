# Calendar Integration

Meetily calendar integration connects upcoming events to recording setup,
meeting detection, summaries, exports, Apple Notes, and local MCP context. It is
local-first and opt-in: calendar data is ignored until the user enables a
provider and selects calendars in Settings.

## Provider Strategy

The rollout is staged so the app gets useful calendar-backed meeting context
without committing to cloud OAuth before the desktop UX and storage boundaries
are stable.

| Phase | Provider | Purpose | Notes |
| --- | --- | --- | --- |
| 1 | Apple Calendar on macOS | Read selected local calendars and create optional linked events for recorded meetings. | Use EventKit through a small native macOS bridge. EventKit is Apple's public API for calendar access. Sandboxed macOS builds need `com.apple.security.personal-information.calendars` and calendar usage strings. |
| 2 | ICS subscription/import | Add read-only events from user-provided calendar feeds or files. | No OAuth, no writes, and useful for teams that publish meeting calendars. Store feed URL only when the user opts in. |
| 3 | Google Calendar | Add direct cloud sync when desktop OAuth callback handling is approved. | Keep provider-specific tokens out of the normalized event table and store them in provider settings with revocation. |

Apple Calendar is first because the app is a Tauri desktop product with strong
macOS usage, and it also supports the related Apple Notes workflow. The shared
data model remains provider-neutral so Google Calendar and ICS can be added
without changing meeting detection, recording metadata, or artifact links.

## Permission Model

Calendar sync is disabled by default. Users must opt in from Settings, select a
provider, and select which calendars are allowed.

Apple Calendar should use EventKit rather than AppleScript for event reads and
writes. EventKit gives a narrower public API surface for events and calendars;
AppleScript should be reserved for Apple Notes or last-resort automation paths.
On current macOS versions, the app must distinguish full event access from
write-only event access. Read sync requires full access; creating Meetily-owned
events can use a write-only path if the implementation supports it.

Permission states:

| State | Behavior |
| --- | --- |
| Not configured | No calendar commands read or write provider data. |
| Permission needed | Settings explains the system prompt and offers a connect action. |
| Connected | Sync reads only selected calendars into the local event cache. |
| Revoked | Sync stops immediately, stale prompts are hidden, and provider tokens/cursors are invalidated. |
| Error | The last sync error is visible and retryable; app startup must not fail. |

Calendar writes, such as creating an Apple Calendar event for a recorded call,
are a separate opt-in from calendar reads. Users can enable read-only upcoming
meeting detection without allowing Meetily to write events.

## Normalized Event Model

Calendar provider data should be normalized before any UI, recording, export, or
MCP surface consumes it.

`calendar_provider_accounts`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID. |
| `provider` | text | `apple`, `ics`, or `google`. |
| `account_label` | text | User-visible account/source label. |
| `status` | text | `not_configured`, `permission_needed`, `connected`, `revoked`, `error`. |
| `last_sync_at` | datetime nullable | Last attempted sync. |
| `last_error` | text nullable | User-safe error summary only. |
| `created_at` / `updated_at` | datetime | Local audit timestamps. |

Provider-specific secrets, OAuth tokens, refresh tokens, and raw auth responses
must not be stored in normalized event rows. If Google OAuth is added later,
store tokens in a dedicated provider-secret store with explicit revocation. On
macOS, use Keychain for provider secrets rather than plaintext app-data files.

`calendar_sources`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID. |
| `provider_account_id` | text | Parent provider account. |
| `provider_calendar_id` | text | Calendar/source identifier from provider. |
| `name` | text | User-visible calendar name. |
| `color` | text nullable | Optional UI color. |
| `selected` | boolean | Only selected calendars are synced into prompts. |
| `read_only` | boolean | Whether writes are allowed for this source. |
| `last_sync_at` | datetime nullable | Source-level sync timestamp. |

`calendar_events`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local deterministic ID, e.g. provider plus event id hash. |
| `provider` | text | `apple`, `ics`, or `google`. |
| `provider_event_id` | text | Provider event identifier. |
| `calendar_source_id` | text | Selected source. |
| `title` | text | Event title. |
| `starts_at` / `ends_at` | datetime | UTC timestamps. |
| `timezone` | text nullable | Provider timezone when available. |
| `location` | text nullable | Truncated location field. |
| `meeting_url` | text nullable | First supported meeting URL extracted from location/notes. |
| `meeting_provider` | text nullable | `google_meet`, `zoom`, `teams`, or `unknown`. |
| `attendee_count` | integer nullable | Count only by default. |
| `attendee_names` | json nullable | Optional names when separately enabled; emails are excluded by default. |
| `organizer_name` | text nullable | Optional organizer display name. |
| `description_excerpt` | text nullable | Short sanitized excerpt for context, never full notes by default. |
| `content_hash` | text | Hash of the normalized fields listed below. |
| `sync_status` | text | `active`, `cancelled`, `stale`, `error`. |
| `updated_at` | datetime | Local update timestamp. |

`meeting_calendar_links`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID. |
| `meeting_id` | text | Meetily meeting id. |
| `calendar_event_id` | text | Normalized calendar event id. |
| `link_source` | text | `selected_before_recording`, `auto_matched`, or `created_by_meetily`. |
| `confidence` | real nullable | Auto-match confidence. |
| `apple_event_identifier` | text nullable | Apple Calendar identifier for created/updated events. |
| `notes_export_id` | text nullable | Apple Notes export linkage when available. |
| `created_at` / `updated_at` | datetime | Local timestamps. |

## Meeting URL Extraction

The calendar service extracts meeting URLs from event location and the sanitized
description excerpt. Supported URLs match the meeting detection rules:

* Google Meet: `meet.google.com/*`
* Zoom: `zoom.us/*`, `*.zoom.us/*`
* Microsoft Teams: `teams.microsoft.com/*`

Extraction must prefer explicit URLs over provider-specific prose, deduplicate
links, and store only the selected canonical URL. Before persistence, strip
common sensitive URL parameters and dial-in fragments such as passcodes, PINs,
telephone numbers, and one-time tokens when they are not required to open the
meeting. Raw event descriptions should not be persisted unless a later feature
adds explicit user consent and a storage reason.

## Change Detection and Event Edge Cases

`calendar_events.content_hash` is computed from the normalized fields that affect
prompts and recording metadata:

* provider event id;
* calendar source id;
* title;
* start and end timestamp;
* timezone;
* location;
* canonical meeting URL;
* meeting provider;
* organizer display name;
* attendee count and enabled attendee display names;
* sanitized description excerpt;
* cancelled status.

Recurring events must be expanded into occurrence rows for the active sync
window. Each occurrence gets a stable local ID derived from provider event id and
occurrence start timestamp. Provider cancellation/deletion marks matching rows
as `cancelled` first so active prompts disappear; rows can be purged after the
retention window below.

All-day events are excluded from meeting detection by default unless they contain
a supported meeting URL and the user explicitly selects them. Floating-time
events are normalized using the provider/calendar timezone when available; if no
timezone is available, use the local system timezone and mark the row as
timezone-inferred for UI disclosure.

## Sync Behavior

Default sync window:

* Look back 1 day so late recordings can still match a recent event.
* Look ahead 14 days for upcoming prompts.
* Refresh on app start, when Settings opens, when the user clicks Sync now, and
  periodically every 15 minutes while the app is running.
* Purge event rows older than 30 days unless they are linked to a Meetily
  meeting; linked rows retain only the fields needed for the link and audit
  status after they leave the active sync window.

Offline behavior:

* Previously synced events remain available until they age out of the sync
  window or the provider is disconnected.
* Sync errors do not block recording, transcription, summaries, or app startup.
* Meeting detection should hide stale event prompts after the configured stale
  window even if sync fails.

Auto-match behavior:

* `selected_before_recording` is used when the user chooses an event.
* `auto_matched` requires matching event time window plus either meeting URL
  equality or high-confidence title similarity.
* Auto-match confidence below `0.85` must produce a prompt instead of silently
  linking the meeting.

## Recording Metadata Flow

Calendar events may populate recording setup only after explicit user selection
or an approved prompt.

Flow:

1. Calendar service syncs selected events into `calendar_events`.
2. Meeting detection reads provider-neutral event candidates.
3. User selects a candidate or starts recording from a prompt.
4. Recording setup uses event title, start/end, meeting URL, and optional context
   to prefill meeting name and metadata.
5. `meeting_calendar_links` records the relationship for summaries, exports,
   Apple Notes, Apple Calendar back-links, and MCP context.

No calendar candidate can auto-start recording. Auto-open may open a meeting URL
only when the existing meeting detection consent mode allows it.

## Apple Notes and Calendar Linking

Apple Notes export and Apple Calendar event creation should share
`meeting_calendar_links` instead of writing separate siloed metadata.

When Apple Calendar event creation is enabled:

* A completed recording can create or update a calendar event with title,
  start/end time, Meetily meeting reference, transcript availability, and summary
  status.
* If an Apple Notes summary exists, the calendar event notes include the Notes
  reference or destination label.
* If the calendar event exists first, later Apple Notes export backfills the
  linked artifact section with calendar metadata.
* Meetily only updates events it created or previously linked through
  `meeting_calendar_links.apple_event_identifier`; it must not modify unrelated
  user calendar events based on title/time similarity alone.

The app should never write to Apple Calendar merely because read-only calendar
sync is enabled.

## Privacy and Storage Rules

Calendar data is sensitive and follows
[Privacy, Consent, and Access Controls](privacy-consent-access-controls.md).

Rules:

* Store the minimum event metadata needed for detection and recording context.
* Do not store full event descriptions by default.
* Do not store attendee emails by default.
* Attendee display names require a separate consent toggle from calendar
  connection because names plus title and time can identify participants.
* Do not expose calendar metadata through MCP, exports, Apple Notes, or cloud AI
  providers unless that destination has separate explicit consent.
* MCP calendar exposure requires its own future scope and Settings toggle; the
  initial calendar Tauri commands are for the app UI only.
* Provider disconnect invalidates sync cursors/tokens and hides stale prompts.
* Provider disconnect deletes unlinked `calendar_events`, clears selected
  calendar sources, and retains only minimal `meeting_calendar_links` rows needed
  to show that a meeting was previously linked. User-created external calendar
  events are not deleted without explicit confirmation.
* Meeting deletion removes local `meeting_calendar_links`; it does not silently
  delete external calendar events without a separate confirmation.

## Implementation References

Planned module boundaries, to be created by implementation issues:

* Rust: `frontend/src-tauri/src/calendar/` for provider bridges, sync, storage,
  and Tauri commands.
* Frontend service: `frontend/src/services/calendarService.ts`.
* Settings UI: `frontend/src/components/CalendarSettings.tsx` and
  `frontend/src/app/settings/page.tsx`.
* Meeting detection consumers:
  `frontend/src/services/meetingDetectionService.ts` and
  `frontend/src/components/MeetingDetectionPrompt.tsx`.
* Recording metadata consumers:
  `frontend/src/hooks/useRecordingStart.ts` and
  `frontend/src/services/recordingService.ts`.

Initial Tauri commands:

* `list_calendar_providers`
* `get_calendar_settings`
* `update_calendar_settings`
* `connect_calendar_provider`
* `disconnect_calendar_provider`
* `list_calendar_sources`
* `sync_calendar_events`
* `list_upcoming_calendar_events`
* `link_meeting_calendar_event`
* `create_or_update_meeting_calendar_event`
