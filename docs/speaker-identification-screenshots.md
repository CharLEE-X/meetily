# Speaker Identification and Screenshots

This document is the implementation contract for speaker identification, local diarization, and optional periodic screenshots. It applies to CHA-1667 and its child issues.

## Consent Model

Speaker identification and screenshots are separate opt-ins.

| Capability | Default | Required opt-in | Revocation behavior |
| --- | --- | --- | --- |
| Audio diarization | Off | User enables speaker identification for a meeting or starts a manual reprocess action | Stop future label generation and clear detected labels by default |
| Manual speaker labels | Available after transcript exists | User edits or confirms a label | User can clear labels without deleting transcript text |
| Periodic screenshots | Off | Global availability plus per-meeting confirmation before first capture | Stop future captures immediately and offer to delete captured screenshots |
| Screenshot-assisted labels | Off | Per-meeting confirmation naming screenshots as a label source | Stop screenshot-derived labels and clear detected labels by default |

The first screenshot confirmation must show:

* Capture interval.
* Storage location.
* Retention behavior.
* Deletion path.
* Whether screenshots may be used for speaker labels, summaries, exports, MCP, or chat indexes.
* A warning that screenshots can include unrelated windows, private documents, notifications, participant video, or credentials visible on screen.

Visual speaker identification is manual-only for this implementation track. Screenshots can be used as user-visible context for manual label correction, but the app must not infer a person's real identity from a face, name badge, participant tile, or other visual signal until a separate consent and model policy is approved.

## Data Model

Additive migrations should create these tables before UI or capture code writes data:

### `speaker_labels`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT primary key | UUID |
| `meeting_id` | TEXT indexed | References `meetings.id` |
| `display_name` | TEXT | User-facing label, for example `Speaker 1` or `Adrian` |
| `source` | TEXT | `diarization`, `manual`, `screenshot_context`, `imported`, or `legacy` |
| `status` | TEXT | `detected`, `confirmed`, `hidden`, or `deleted` |
| `confidence` | REAL nullable | Detection confidence when available |
| `created_at`, `updated_at` | DATETIME | UTC |
| `deleted_at` | DATETIME nullable | Soft-delete marker for audit and undo windows |
| `metadata_json` | TEXT nullable | Content-free metadata, no transcript text or image payloads |

### `transcript_speaker_segments`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT primary key | UUID |
| `meeting_id` | TEXT indexed | References `meetings.id` |
| `transcript_id` | TEXT indexed | References transcript segment when available |
| `speaker_label_id` | TEXT indexed | References `speaker_labels.id` |
| `start_time`, `end_time` | REAL nullable | Recording-relative seconds |
| `source` | TEXT | `diarization`, `manual`, `legacy`, or `screenshot_context` |
| `confidence` | REAL nullable | Segment assignment confidence |
| `created_at`, `updated_at` | DATETIME | UTC |
| `correction_id` | TEXT nullable | Last correction event that changed this row |

### `speaker_corrections`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT primary key | UUID |
| `meeting_id` | TEXT indexed | References `meetings.id` |
| `action` | TEXT | `rename`, `merge`, `split`, `assign`, `confirm`, `clear`, or `delete` |
| `before_json`, `after_json` | TEXT | Label IDs and timing ranges only; no transcript bodies |
| `created_at` | DATETIME | UTC |

### `meeting_screenshots`

| Column | Type | Notes |
| --- | --- | --- |
| `id` | TEXT primary key | UUID |
| `meeting_id` | TEXT indexed | References `meetings.id` |
| `captured_at` | DATETIME | UTC capture time |
| `recording_time` | REAL nullable | Recording-relative seconds |
| `file_path` | TEXT | App-managed path relative to app data when possible |
| `thumbnail_path` | TEXT nullable | App-managed thumbnail path |
| `display_label` | TEXT nullable | Display/window label when available |
| `status` | TEXT | `captured`, `skipped`, `permission_denied`, `deleted`, or `failed` |
| `redaction_status` | TEXT | `not_available`, `not_applied`, `applied`, or `failed` |
| `source` | TEXT | `periodic`, `manual`, or `imported` |
| `created_at`, `updated_at` | DATETIME | UTC |
| `deleted_at` | DATETIME nullable | Soft-delete marker |
| `metadata_json` | TEXT nullable | No OCR text, transcript text, or image payloads |

The legacy `transcripts.speaker` column is a compatibility input only. New code should write `speaker_labels` and `transcript_speaker_segments`, then optionally mirror a simple display label back to `transcripts.speaker` for older readers.

## Storage Layout

Use Tauri app-data paths:

```text
artifacts/
  meetings/<meeting-id>/
    screenshots/
      <screenshot-id>.png
      thumbnails/
        <screenshot-id>.jpg
```

Screenshots must not be stored in user-selected export folders by default. Exporting screenshots later requires destination preview and explicit selection of screenshot inclusion.

## Retention and Deletion

Default retention is "keep until deleted with meeting". Future retention settings may add automatic cleanup, but must show the configured window before deleting user-visible screenshots.

Deletion rules:

* Deleting a screenshot marks its row deleted and removes app-managed image and thumbnail files.
* Clearing speaker labels deletes detected labels and segment mappings while preserving transcript text.
* Deleting a meeting removes screenshots, labels, corrections, segment mappings, and any app-managed files under that meeting artifact folder.
* Missing screenshot files must show a nonfatal missing-file state and allow metadata cleanup.

## Runtime Indicators

When screenshots are active, the recording surface must show:

* Screenshot capture is on.
* Next capture countdown or scheduled time.
* Pause/resume screenshots.
* Stop screenshots.
* A link or affordance to review/delete captures.

Pausing recording pauses screenshots. Resuming recording may resume screenshots only if the screenshot state is still enabled and visible.

## Service Boundaries

Proposed commands:

* `run_speaker_labeling`
* `get_speaker_labels`
* `update_speaker_label`
* `clear_speaker_labels`
* `get_screenshot_preferences`
* `set_screenshot_preferences`
* `start_meeting_screenshot_capture`
* `stop_meeting_screenshot_capture`
* `capture_meeting_screenshot_now`
* `list_meeting_screenshots`
* `delete_meeting_screenshot`
* `attach_meeting_screenshots`

All commands that create labels or screenshots must re-check consent and OS permissions in Rust, even when the UI has already shown consent.

## Export, MCP, and Chat Boundaries

Speaker labels and screenshots are not automatically available to exports, MCP clients, cloud providers, or meeting chat indexes.

They may cross a boundary only when:

* The destination feature is enabled.
* The user selects speaker labels or screenshots as included content.
* The preview names the selected content types.
* The audit/history record notes inclusion without storing image payloads or transcript bodies.

Screenshots must never be sent to cloud providers, MCP clients, exports, or chat indexes as OCR text unless a separate redaction and consent policy is approved.
