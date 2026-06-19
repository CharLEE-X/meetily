# Apple Notes Export

Meetily Apple Notes export writes meeting summaries to the local macOS Notes app.
It is opt-in, local-first, and designed around explicit destination preview
before meeting content leaves Meetily's app-managed storage.

## Integration Decision

The first supported implementation uses AppleScript through the local
`osascript` bridge.

| Option | Decision | Reason |
| --- | --- | --- |
| AppleScript | Use for the first macOS slice. | Notes exposes accounts, folders, note ids, note names, and HTML bodies in its scripting dictionary. This is enough for folder discovery, note creation, and app-owned note updates without cloud OAuth. |
| Shortcuts | Do not depend on it for core export. | Shortcuts adds user setup and brittle shortcut distribution. It can remain a future user-custom automation target. |
| Native Notes API | Not available for third-party macOS apps. | Apple does not provide a public Notes framework for creating notes directly. |
| File export only | Keep as separate export feature. | File export does not satisfy the user's Notes organization workflow or calendar backlink goal. |

AppleScript is intentionally narrow. Meetily should only read folders and notes
needed to find the app-managed destination or update a previously exported note.
It must not scan unrelated note contents or silently export meeting content.

## Consent And Permission Flow

Apple Notes export is off by default.

Users enable it from Settings, then confirm the destination before the first
write. Manual export remains available even when automatic export is disabled.
Automatic export is a separate opt-in and runs only after a summary is complete.

Expected states:

| State | Behavior |
| --- | --- |
| Not configured | No Notes commands run. |
| Permission needed | Settings explains the macOS Automation prompt and offers a connect/test action. |
| Connected | Manual export and enabled automatic export can write to the selected destination. |
| Revoked | Future exports stop and existing local export metadata remains visible. |
| Error | Last user-safe error is visible with retry and disconnect actions. |

If macOS rejects Automation access, Meetily stores a sanitized error and shows
remediation that points users to System Settings > Privacy & Security >
Automation. The app must keep recording, transcription, and summaries working
when Notes export is unavailable.

## Destination Model

Default destination:

* Account: `On My Mac` when present; otherwise the first writable Notes account
  returned by Notes after the preview clearly labels the account.
* Root folder: `Meetily`.
* Optional grouping: per-client/project folders when the user chooses a grouping
  field. Try the requested nested folder first; if AppleScript cannot create or
  find that nested folder for the selected account, use a flat folder name such
  as `Meetily - Client Name` and record the fallback in local status.

Destination preview includes the account label, folder label, note title, and
the content sections that will be written. The preview is shown from meeting
details before export and must be confirmed whenever the resolved account,
folder, grouping mode, or note title pattern differs from the last confirmed
destination. Settings also shows whether the destination has been confirmed so
users can understand whether automation can run reliably.

If the selected account is iCloud-backed, preview copy must say that Apple Notes
may sync the exported meeting content through the user's Apple account. This is
still a user-selected local macOS destination, but it is not guaranteed to remain
device-local once Notes syncs it.

## Note Shape

The exported note should be readable without needing Meetily open:

* meeting title;
* meeting date/time and source meeting id;
* summary;
* key decisions;
* action items;
* transcript reference or local Meetily meeting reference;
* Apple Calendar event reference when the meeting is linked to one;
* export timestamp and "Created by Meetily" footer.

Full transcripts are not written by default. The first slice includes a
transcript reference or local path/link when available, because transcripts can
be long and often contain more sensitive raw content than summaries.

Notes bodies are written as simple HTML and escaped from meeting content.
Short metadata values can be passed to AppleScript as process arguments, but the
HTML body should be passed through a restrictive temporary file or stdin to avoid
macOS argument-length limits and to prevent content interpolation into scripts.

## Duplicate Handling

Meetily stores one export record per meeting and destination.

`apple_notes_exports`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID. |
| `meeting_id` | text | Meetily meeting id. |
| `provider` | text | `apple_notes`. |
| `account_id` / `account_name` | text nullable | Notes account metadata when available. |
| `folder_id` / `folder_name` | text nullable | Destination folder metadata. |
| `provider_note_id` | text nullable | Apple Notes note id returned by Notes. |
| `note_title` | text | Last exported title. |
| `content_hash` | text | Hash of normalized exported content. |
| `status` | text | `pending`, `exported`, `updated`, `failed`, `revoked`, or `missing`. |
| `last_error` | text nullable | User-safe error summary only. |
| `exported_at` / `created_at` / `updated_at` | datetime | Local audit timestamps. |

Repeated export updates the stored note id when it still exists. If the note was
deleted or moved outside Meetily, a best-effort `exists note id ...` probe marks
the row `missing`; the next confirmed export creates a new note and updates the
local record. Notes note ids are treated as durable enough for app-owned updates
but not permanent across iCloud re-sync, account re-add, or container rebuilds.
Meetily must not create a duplicate note merely because the content changed.

## Calendar Linking

Apple Notes and Apple Calendar share local meeting artifact metadata:

* `meeting_calendar_links.notes_export_id` points to the Notes export record
  when a linked calendar event exists.
* Calendar event creation can include the Notes destination label or note id
  after export.
* Notes export attaches its local export id to any existing meeting calendar
  link.

Neither integration implies consent for the other. Notes export must not expose
calendar metadata unless Calendar access and Notes export are both enabled.

## Failure And Privacy Rules

* No automatic export before the user enables Notes export and automatic export
  separately.
* Failed exports never mark a meeting as exported.
* Disconnect stops future writes but does not delete external Notes content.
* Meeting deletion removes local export metadata and asks before modifying or
  deleting an external note.
* Logs and errors must not include full summary or transcript content.
* Non-macOS builds show Notes export as unsupported instead of failed.

## Implementation References

Implemented module boundaries:

* Rust: `frontend/src-tauri/src/apple_notes.rs`.
* Database migration:
  `frontend/src-tauri/migrations/20260107000000_add_apple_notes_exports.sql`.
* Frontend service: `frontend/src/services/appleNotesService.ts`.
* Settings UI: `frontend/src/components/AppleNotesSettings.tsx`.
* Meeting detail UI: `frontend/src/components/MeetingDetails/AppleNotesExportPanel.tsx`.
* Calendar link updates: `frontend/src-tauri/src/calendar.rs` and
  `frontend/src-tauri/migrations/20260102000000_add_calendar_integration_tables.sql`.

Tauri commands:

* `list_apple_notes_providers`
* `get_apple_notes_settings`
* `connect_apple_notes_provider`
* `disconnect_apple_notes_provider`
* `update_apple_notes_settings`
* `preview_apple_notes_export`
* `export_meeting_to_apple_notes`
* `get_meeting_apple_notes_export`
* `list_recent_apple_notes_exports`

## QA Matrix

| Scenario | Expected result |
| --- | --- |
| Open Settings before connecting Notes | Apple Notes shows not connected, automation health explains setup is incomplete, and no Notes commands run. |
| Connect Apple Notes | The account enters permission-needed/ready state and the first write can request macOS Automation permission. |
| Preview a meeting export | Meeting details shows the destination folder, note title, included sections, and destination confirmation requirement. |
| Confirm and export | A note is created in the configured folder, an `apple_notes_exports` record is saved, and repeat export updates the same note id. |
| Export after Calendar event creation | The existing `meeting_calendar_links` row is backfilled with `notes_export_id`. |
| Create Calendar event after Notes export | The Calendar event notes include Apple Notes export metadata. |
| Automation disabled | Manual export still works after preview; automatic export does not run. |
| Disconnect Notes | Future writes stop, external Notes content is not deleted, and local history remains visible. |
