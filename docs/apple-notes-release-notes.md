# Apple Notes Export Release Notes

This release adds local Apple Notes summary export for macOS users. The feature
is opt-in, uses the local macOS Notes app through Automation, and keeps export
status in Meetily's local database.

## Added

* Apple Notes tab in Settings with connection state, destination folder, and
  default-off auto-export setting.
* Automation health checks for macOS support, permission, destination
  confirmation, auto-export, and recent export activity.
* Meeting detail export panel with destination preview and confirmation before
  writing meeting content.
* App-managed Notes export records with account, folder, note id, content hash,
  status, and last error.
* Repeat export updates the same Apple Notes note when the stored note id is
  still available.
* Apple Calendar linking: Notes exports attach to existing meeting calendar
  links, and Calendar event creation can include Notes export metadata.

## Provider Limitations

* Apple Notes export is supported only in the macOS desktop app.
* The first implementation uses AppleScript through `osascript` because Apple
  does not expose a public Notes framework for third-party write access.
* Meetily targets the `On My Mac` Notes account when available, then falls back
  to the first Notes account returned by the app.
* Full transcripts are not written by default. Exported notes include summary
  content and a local transcript reference when available.
* Auto-export is guarded by destination confirmation; manual export remains the
  reliable path for the first write to each destination.

## Privacy and Consent

Apple Notes export is off until the user connects Apple Notes and confirms the
destination from meeting details. Auto-export is a separate setting and remains
off by default.

If the selected Notes account syncs through iCloud, exported summaries may leave
the local device through the user's Apple account. Meetily shows destination and
sync disclosure before writing content. Disconnecting Apple Notes stops future
writes and preserves visible local history; it does not delete external notes.

## QA Matrix

| Scenario | Expected result |
| --- | --- |
| Open Settings before connecting Notes | Apple Notes shows not connected and automation health explains setup is incomplete. |
| Connect Apple Notes | Settings prepares the local account state and explains that the first write may request macOS Automation permission. |
| Save destination settings | Root folder and auto-export preference persist and update the automation health card. |
| Preview a meeting export | Meeting details shows note title, folder, sections, destination hash, and any iCloud disclosure. |
| Export with destination confirmed | Notes creates or updates one app-managed note and Meetily stores an `apple_notes_exports` row. |
| Export the same meeting again | Meetily updates the stored note id instead of creating a duplicate. |
| Apple Notes permission denied | The export fails with a user-safe permission error; recording, transcription, and summary stay usable. |
| Calendar event exists first | Notes export backfills `meeting_calendar_links.notes_export_id`. |
| Notes export exists first | Later Calendar event creation includes Notes metadata in the event notes. |
| Disconnect Apple Notes | Future writes stop, external Notes content is untouched, and local export history remains visible. |

## Verification Notes

Checks used for this implementation:

* `cargo test --manifest-path frontend/src-tauri/Cargo.toml apple_notes::tests --lib`
* `cargo test --manifest-path frontend/src-tauri/Cargo.toml calendar::tests --lib`
* `pnpm --dir frontend run build`

The full Rust library suite currently has unrelated audio device-detection test
failures on this machine. Those failures are outside the Apple Notes and
Calendar integration surface.
