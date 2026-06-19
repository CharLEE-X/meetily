# Apple Reminders Follow-Up Integration

Meetily Apple Reminders follow-ups turn meeting actions into editable reminder
drafts and create only the reminders the user explicitly approves. The feature
is local-first, default-off, and review-before-write. It is optimized for
day-to-day engineering follow-ups where the reminder needs enough source context
to still make sense hours or days after the call.

## Product Principles

* No Apple Reminders access is attempted until the user connects the integration
  in Settings.
* No reminder is created silently. Every external write requires a visible draft
  preview and an explicit create action.
* Meetily stores only provider/list metadata, draft metadata, dedupe keys, and
  app-created reminder link records needed for status, history, and safe retry.
* Meetily must not read or display unrelated user reminders.
* Disconnecting Apple Reminders stops future reads and writes, but does not
  delete external reminders.
* Meeting deletion must disclose linked external reminders and ask before any
  external reminder is modified or deleted.

## Provider Strategy

Apple Reminders is the first task provider because Meetily is a macOS-first
desktop app and the Reminders app is local to the user's machine. The provider
model should stay narrow and provider-neutral enough that future task providers
can reuse draft generation and review UI without inheriting Apple-specific
automation details.

| Phase | Provider path | Purpose | Notes |
| --- | --- | --- | --- |
| 1 | Local macOS Reminders bridge | Connect/disconnect, list discovery, create selected reminders, and read status only for app-created reminders. | This can use AppleScript/JXA as the first Tauri bridge when EventKit Reminders bindings are not yet in place. It requires Apple Events automation permission for Reminders and must surface permission errors in Settings. |
| 2 | EventKit hardening | Replace or supplement the bridge with Apple's public EventKit Reminders API where practical. | EventKit offers a narrower supported API for reminders, calendars/lists, completion status, and identifiers. Sandboxed builds need the relevant reminders entitlement and usage strings. |
| 3 | Provider abstraction | Add other task providers only after the draft/review/write/audit model is stable. | Jira, Linear, Todoist, Asana, Google Tasks, and automatic issue creation are out of scope for the first Apple Reminders release. Codex or Claude automation should handle tool-rich workflows rather than duplicating those capabilities inside Meetily. |

The first implementation should prefer the smallest reliable local bridge. If
AppleScript/JXA is used, scripts must be generated from structured data, escape
all user-controlled strings, and return only provider identifiers/status for
lists and app-created reminders. The bridge must never enumerate reminder
contents outside the selected lists and app-created identifiers required for
status/history.

## Permission And State Model

Apple Reminders is controlled from Settings.

| State | Behavior |
| --- | --- |
| `not_configured` | No Reminders commands read or write provider data. Review UI can explain that the integration is available but disabled. |
| `permission_needed` | User clicked Connect or Sync/Discover lists, and the OS prompt or automation permission is required. App startup remains unaffected. |
| `connected` | Meetily can list destination lists and create user-selected reminders. Draft generation remains local and can run before creation, but creation controls require this state. |
| `revoked` | Reads/writes stop immediately. Existing local link records remain visible as unavailable/history metadata. |
| `error` | Last user-safe error is visible and retryable. Failures do not block recording, transcription, summaries, or app startup. |

Consent scope is global provider opt-in plus per-create explicit approval.
Connection alone allows list discovery and app-created reminder status checks; it
does not authorize background writes.

## Local Data Model

The schema should be provider-neutral where it affects draft review and history.
Apple-specific identifiers belong in provider fields, not in meeting or summary
tables.

`reminder_provider_accounts`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID. |
| `provider` | text | `apple_reminders` for the first release. |
| `account_label` | text | User-visible provider label. |
| `status` | text | `not_configured`, `permission_needed`, `connected`, `revoked`, or `error`. |
| `default_list_id` | text nullable | Local `reminder_lists.id` selected for new follow-ups. |
| `last_sync_at` | datetime nullable | Last list/status discovery attempt. |
| `last_error` | text nullable | User-safe error summary only. |
| `created_at` / `updated_at` | datetime | Local audit timestamps. |

`reminder_lists`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID or deterministic provider/list hash. |
| `provider_account_id` | text | Parent provider account. |
| `provider_list_id` | text | Reminders list/calendar identifier from provider. |
| `name` | text | User-visible list name. |
| `color` | text nullable | Optional UI color when available. |
| `selected` | boolean | Whether this list can be used as a destination. |
| `is_default` | boolean | Whether this is the global default list. |
| `last_seen_at` | datetime nullable | Last successful discovery timestamp. |

`reminder_drafts`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID. |
| `meeting_id` | text | Source Meetily meeting id. |
| `summary_id` | text nullable | Summary/template output used to produce the draft. |
| `title` | text | Editable reminder title. |
| `notes` | text nullable | Editable notes preview before creation. |
| `due_at` | datetime nullable | Suggested due date/time in UTC, blank when uncertain. |
| `priority` | integer nullable | Provider-neutral priority, e.g. 1 high, 5 medium, 9 low. |
| `list_id` | text nullable | Suggested destination list. |
| `category` | text | Suggested programmer follow-up category. |
| `confidence` | real | 0.0-1.0 extraction confidence. |
| `source_evidence` | json | Short cited source snippets, timestamps, action item IDs, or summary section labels. Snippets should be capped to about 280 characters each. |
| `dedupe_key` | text | Stable hash from meeting id, normalized title, category, source evidence, and due bucket. |
| `status` | text | `suggested`, `selected`, `dismissed`, `created`, `duplicate`, or `failed`. |
| `created_at` / `updated_at` | datetime | Local timestamps. |

`created_reminder_links`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | text | Local UUID. |
| `meeting_id` | text | Source Meetily meeting id. |
| `draft_id` | text nullable | Draft used for creation, if retained. |
| `provider` | text | `apple_reminders`. |
| `provider_reminder_id` | text | Apple Reminders identifier returned by the provider bridge. |
| `provider_list_id` | text | Destination list identifier. |
| `title` | text | Title at creation time for display and history. |
| `dedupe_key` | text | Same dedupe key used by the draft. |
| `status` | text | `created`, `open`, `completed`, `missing`, `unavailable`, or `error`. |
| `created_at` | datetime | Creation timestamp. |
| `last_status_at` | datetime nullable | Last status refresh timestamp. |
| `last_error` | text nullable | User-safe status/read error. |

Local records must not store unrelated reminder contents. Status refresh may
query only provider identifiers previously created by Meetily.

## Reminder Draft Shape

The frontend service and Tauri commands should use the same draft shape:

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Local draft id. |
| `meetingId` | yes | Source meeting id. |
| `title` | yes | Short, action-oriented, editable. |
| `notes` | no | Context that will be written to the reminder if the user approves. |
| `dueAt` | no | Suggested due date/time; omit when uncertain. |
| `priority` | no | Provider-neutral priority. |
| `listId` | no | Default or category-specific destination list. |
| `sourceEvidence` | yes | Short snippets/timestamps/action item references for user review. |
| `category` | yes | Programmer follow-up category. |
| `confidence` | yes | Extraction confidence; low-confidence drafts are hidden or clearly marked. |
| `dedupeKey` | yes | Stable duplicate prevention key. |
| `status` | yes | Draft lifecycle status. |

Notes written to Apple Reminders should be concise and structured:

```text
From Meetily meeting: <meeting title>
When: <meeting date/time>
Why: <one-line source evidence>
Source: meetily://meeting/<meeting-id> or local meeting reference label

<optional user-edited notes>
```

If the `meetily://` deep-link scheme is not registered yet, use a source
reference label that still lets the user locate the meeting inside Meetily. Do
not include full transcripts or long summaries in reminder notes by default.

## Dedupe And Retry Rules

Duplicate prevention happens before creation and during retry.

The initial dedupe key should normalize:

* meeting id;
* draft category;
* lowercased title with punctuation collapsed;
* source evidence identifiers or transcript timestamps;
* due date bucket, such as same day or same hour for relative reminders.

If a draft's dedupe key matches an existing `created_reminder_links` row for the
same meeting and provider, the UI marks it as already created and does not
preselect it. Users can still edit enough fields to generate a different dedupe
key when they intentionally want another reminder.

Creation is per-reminder. Partial failure must retain successful links and show
failed drafts with retryable user-safe errors.

## Programmer Follow-Up Categories

The draft engine should bias toward useful engineering reminders and avoid
generic meeting noise.

| Category | Examples | Default due date behavior | Default priority |
| --- | --- | --- | --- |
| `pr_review` | Review a teammate's PR, re-review after changes, check CI before merge. | Tomorrow morning when no explicit due date exists; "after updates" remains undated unless a time is present. | Medium |
| `linear_follow_up` | Update a Linear issue, add acceptance criteria, move status, reply to a blocker. | End of next workday unless the meeting says today/tomorrow. | Medium |
| `deploy_alert_check` | Check production after deploy, verify alerting, revisit logs after a few hours. | Parse "in a few hours", "after deploy", or default to 2 hours when deploy/check language is explicit. | High |
| `docs_update` | Write docs, update README, publish release notes, document decisions. | Within 2 workdays unless an explicit due date exists. | Low/Medium |
| `implementation_task` | Build a small code change, fix a bug, add a test, investigate a repo issue. | Leave blank when scope is unclear; otherwise next workday. | Medium |
| `experiment_revisit` | Revisit an architecture choice, compare metrics, check experiment result. | One week by default unless the meeting names a shorter window. | Low/Medium |
| `clarification_follow_up` | Ask a teammate/customer a question, send a summary, confirm ownership. | Tomorrow when no explicit due date exists. | Medium |

Category presets must be configurable later. Conservative defaults are required:
it is better to produce fewer high-signal drafts than many noisy reminders.

## Generation Rules

Draft generation consumes local meeting data only:

* structured action items from the summary;
* summary sections such as decisions, risks, blockers, or follow-ups;
* transcript snippets around action-item timestamps when available;
* user-provided additional context for the summary when present.

The engine should prefer explicit owner/action language. Drafts should be
discarded or marked low confidence when the action is vague, already completed,
owned by someone else without user involvement, or not actionable.

Due-date inference must be conservative. Relative phrases are interpreted from
the meeting end time or summary generation time, not the current wall clock at
draft review time. Ambiguous phrases such as "later" or "soon" should not create
a due date unless a category preset gives a safe default and the UI explains the
choice.

## Review UI Contract

The first write-capable UI must support:

* scan all suggestions;
* edit title, notes, due date, priority, and list;
* select/unselect drafts;
* dismiss a draft for the current meeting;
* create only selected drafts;
* show disconnected, permission denied, empty, duplicate, partial failure, and
  success states.

The primary action must communicate the number of reminders that will be
created. No keyboard shortcut, auto-export, or post-summary automation may create
reminders without the same explicit review state.

## Privacy, Revoke, And Deletion Behavior

Apple Reminders follows the shared
[Privacy, Consent, and Access Controls](privacy-consent-access-controls.md)
policy.

Specific rules:

* Connecting Apple Reminders authorizes list discovery and app-created reminder
  status checks only.
* Creating reminders requires a per-action preview that includes destination
  list, title, due date, priority, and notes.
* Local audit metadata includes provider, list id/name, draft id, dedupe key,
  created timestamp, result status, and source meeting id. It must not store raw
  provider scripts, unrelated reminder contents, or full transcript text.
* Disconnecting or OS permission revocation disables new writes, hides create
  controls, and marks status refresh unavailable.
* Meeting deletion removes local drafts and link records by default. If external
  reminders exist, the delete flow must show them and ask before attempting to
  delete or modify Apple Reminders items.
* Clearing local history never silently deletes Apple Reminders items.

## Out Of Scope For First Release

* Background reminder creation without review.
* Reading or searching all Apple Reminders content.
* Full two-way sync of every reminder in selected lists.
* Creating Linear/Jira/GitHub issues from reminders.
* Cloud task providers.
* Sending reminder drafts or Apple Reminders status to MCP clients, exports,
  Apple Notes, calendar events, or cloud AI providers without a separate
  destination-specific opt-in.
* Deleting external Apple Reminders during meeting deletion without explicit
  confirmation.

## Implementation Slices

The implementation issues should use this order:

1. Provider connection, permission state, list discovery, and default list.
2. Local draft generation and unit tests.
3. Review UI for edit/select/dismiss/create-intent.
4. Write path for selected reminders, dedupe, partial failure, and link records.
5. Developer presets for category defaults.
6. Created-reminder status/history for app-created reminders.
7. Documentation, QA matrix, and release notes.
