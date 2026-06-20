# Speaker Identification and Screenshots

This document is the implementation contract for speaker identification, local
diarization, optional periodic screenshots, and call-window snapshot capture. It
applies to CHA-1667, CHA-1813, and their child issues.

## Consent Model

Speaker identification and screenshots are separate opt-ins.

| Capability | Default | Required opt-in | Revocation behavior |
| --- | --- | --- | --- |
| Audio diarization | Off | User enables speaker identification for a meeting or starts a manual reprocess action | Stop future label generation and clear detected labels by default |
| Manual speaker labels | Available after transcript exists | User edits or confirms a label | User can clear labels without deleting transcript text |
| Periodic screenshots | Off | Global availability plus per-meeting confirmation before first capture | Stop future captures immediately and offer to delete captured screenshots |
| Call-window snapshots | Off | Global availability plus per-meeting confirmation naming call-window scope | Stop future captures immediately and offer to delete captured snapshots |
| Screenshot-assisted labels | Off | Per-meeting confirmation naming screenshots as a label source | Stop screenshot-derived labels and clear detected labels by default |

The first screenshot confirmation must show:

* Capture interval.
* Storage location.
* Retention behavior.
* Deletion path.
* Whether screenshots may be used for speaker labels, summaries, exports, MCP, or chat indexes.
* A warning that screenshots can include unrelated windows, private documents, notifications, participant video, or credentials visible on screen.
* For call-window snapshots, a separate statement that Meetily captures only the
  detected call window when fresh bounds are available, and skips capture rather
  than silently falling back to full-screen capture.

Visual speaker identification is manual-only for this implementation track. Screenshots can be used as user-visible context for manual label correction, but the app must not infer a person's real identity from a face, name badge, participant tile, or other visual signal until a separate consent and model policy is approved.

## Call-Window Snapshot Contract

Call-window snapshots are a narrower successor to full-screen periodic
screenshots. They are designed to capture the visible meeting window only, so
speaker cues and meeting context can be reviewed without turning Meetily into a
screen recorder.

Capture scope:

* Capture only the detected call window when window bounds are known, fresh, and
  linked to a supported meeting provider signal.
* If bounds are missing, stale, off-screen, implausibly small, or tied to an
  unsupported/non-call window, skip capture and record a skipped metadata row.
* Full-screen fallback is prohibited unless a future release adds an explicit
  user confirmation that names the fallback, its risk, and the one-time scope.
* Capture is still frame-based. Continuous video recording is out of scope
  because it would increase privacy risk, storage volume, review burden, and
  participant consent complexity.

Call-window capture v1 manual checklist:

* Google Meet in a supported browser: focus the Meet tab, start recording with
  screenshots enabled, confirm the snapshot is cropped to the browser meeting
  window and metadata includes provider, title, bounds, recording time, and
  `periodic` source trigger.
* Zoom desktop app: focus the active meeting window, start recording with
  screenshots enabled, confirm the snapshot is cropped to Zoom and no fullscreen
  image is created when the meeting window is not detectable.
* Microsoft Teams desktop or browser: focus the call window, start recording
  with screenshots enabled, confirm missing Accessibility or Screen Recording
  permission shows a user-facing screenshots-unavailable status while audio
  recording continues.

Required metadata for each captured or skipped snapshot:

| Field | Purpose |
| --- | --- |
| `provider` | Detected provider, such as `google-meet`, `zoom`, `teams`, `slack`, or `unknown`. |
| `window_title` | User-visible call/window title when available; no transcript or OCR text. |
| `window_bounds` | x/y/width/height used for call-window-only capture. |
| `recording_time` | Recording-relative seconds for timeline placement. |
| `relevance_confidence` | 0-100 score from the relevance filter. |
| `source_trigger` | `interval`, `speech-event`, `manual`, `startup`, or `retry`. |
| `redaction_state` | `not_available`, `not_applied`, `applied`, `failed`, or `needs_review`. |
| `skip_reason` | For skipped rows, why no image was stored. |

Downstream use:

* Meeting timeline review may show thumbnails and metadata after screenshot
  consent.
* Speaker labels may use snapshots only after the separate
  screenshot-assisted-label consent is enabled for that meeting.
* Meeting chat, exports, MCP tools, cloud providers, and post-meeting agent
  handoffs must not include snapshots, OCR, or derived visual facts unless that
  destination has its own explicit content-inclusion consent and preview.
* Snapshot metadata may be used for local QA and filtering without exposing image
  payloads outside app-managed storage.

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

For call-window snapshots, `metadata_json` must include the metadata fields from
the Call-Window Snapshot Contract. The image file path remains outside
`metadata_json` and must be app-managed.

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
* Skipped snapshots store metadata only and never create placeholder image files.
* Retention cleanup must remove both captured call-window image files and their
  thumbnail files before or at the same time as metadata rows are marked deleted.

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
