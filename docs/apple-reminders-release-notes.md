# Apple Reminders Follow-Ups Release Notes

This release adds local Apple Reminders follow-up creation for macOS users. It is
default-off, review-before-write, and designed for programmer workflows such as
PR review, Linear follow-up, deploy checks, docs updates, implementation tasks,
experiment revisits, and clarification follow-ups.

## What Is Included

* Connect Apple Reminders from Settings.
* Discover local Reminders lists and choose a global default list.
* Configure developer workflow presets for category enablement, due-date
  defaults, priority defaults, and category-specific lists.
* Generate local reminder drafts from meeting summaries and action items.
* Edit, select, dismiss, and create only approved reminder drafts.
* Store local links for app-created reminders to support duplicate prevention,
  retry, meeting context, status, and history.
* Show created reminders in meeting details and recent follow-up history.
* Refresh status for app-created reminders as `open`, `completed`, `missing`, or
  `unavailable`.

## Privacy And Consent

Apple Reminders follow-ups are opt-in. Meetily does not attempt to access
Reminders until the user connects the provider in Settings.

Meetily writes to Apple Reminders only after the user reviews drafts and clicks
the explicit create action. Connection and list discovery do not authorize
background reminder creation.

Meetily stores local provider/list metadata, editable draft metadata, dedupe
keys, app-created reminder identifiers, source meeting ids, status timestamps,
and user-safe errors. It does not store unrelated Apple Reminders content.

Status refresh is limited to reminder identifiers previously created by
Meetily. Reminder titles, list names, and due dates shown in Meetily history are
loaded from Meetily's local database, not from scanning Apple Reminders.

Disconnecting Apple Reminders stops future writes and status refresh. Existing
external Apple Reminders are not deleted or modified.

## Provider Limitations

* Apple Reminders is supported only on macOS in this release.
* The first bridge uses local macOS automation to talk to Reminders. macOS may
  prompt for permission, and permission can be revoked in System Settings.
* Status refresh is best-effort. If permission is revoked or Reminders cannot be
  reached, local history shows `unavailable`.
* If an app-created reminder is deleted outside Meetily, history marks it
  `missing` when status can be refreshed.
* Meetily does not sync all reminders from selected lists.
* Meetily does not read or search unrelated Apple Reminders content.

## Out Of Scope

* Background creation without review.
* Auto-creating reminders immediately after a meeting.
* Deleting or editing external Apple Reminders from Meetily.
* Syncing every reminder in a list.
* Sending reminder drafts or reminder status to MCP clients, exports, Apple
  Notes, calendar events, cloud LLMs, Codex, or Claude without a separate
  destination-specific opt-in.
* Creating Linear, Jira, GitHub, or other issue-tracker tasks directly. Those
  workflows should stay in Codex or Claude when they need repository, Linear,
  Jira, or GitHub tool access.

## QA Checklist

Before shipping a build with this feature, verify:

* connect and disconnect states;
* permission denied and permission restored;
* list discovery and default-list selection;
* workflow preset changes;
* draft generation from a meeting summary;
* draft edit and dismiss;
* create selected reminders;
* duplicate prevention on retry;
* partial creation failure handling;
* status refresh for open/completed/missing reminders;
* unavailable state after disconnect or revoked permission;
* meeting deletion removes local reminder drafts/links without silently deleting
  external Apple Reminders.
