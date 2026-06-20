# Active Speaker Timeline

Active speaker timeline support uses local meeting-window evidence to help label
who spoke in each transcript segment. The feature is designed for review and
correction, not identity proof.

## Label States

| State | Meaning | Downstream behavior |
| --- | --- | --- |
| Suggested | Meetily generated a label from local audio timing, legacy source labels, or call-window visible-name cues. | Shown with confidence and evidence context. Users should review before relying on it. |
| Confirmed | The user accepted or edited the label. | Treated as the trusted label for the meeting and preserved when generated labels are cleared. |
| Manual | The user assigned a speaker directly to a transcript segment. | Stored as a confirmed correction and can be undone through the recent correction log. |
| Low confidence | Evidence was weak, ambiguous, or provider confidence was low. | Stays review-only and does not overwrite stronger audio or confirmed labels. |
| Unknown | No usable visual, audio, or manual label exists. | Transcript remains unlabeled until the user assigns a speaker or reruns detection. |

## Evidence Use

Meetily may use only bounded facts visible in the call window:

* Visible participant display-name text.
* Provider active-speaker markers such as caption speaker labels, active tile
  rings, participant-list active state, or equivalent UI text.
* Snapshot recording time and transcript segment timing.
* Provider id and confidence score.

Meetily must not use face recognition, face matching, profile-photo matching,
appearance-based identity inference, or cloud identity matching. Snapshot images
stay local unless a future destination has its own explicit consent and preview.

## Confidence and Corrections

High-confidence visual cues can be suggested or auto-applied depending on the
speaker-labeling setting. Low-confidence cues remain review-only. Near-tied cues
for different names are treated as ambiguous and fall back to audio or timing
labels.

Users can:

* Accept a suggested label.
* Rename a label.
* Assign a visual suggestion to a transcript segment.
* Merge duplicate labels.
* Clear generated labels while keeping confirmed/manual labels.
* Undo the most recent speaker correction.

Correction events store label IDs, timing ranges, source, confidence, and state
snapshots. They do not store transcript body text, raw OCR text, face-derived
features, or screenshot image payloads.

## QA Fixtures

Representative fixtures live in
`docs/active-speaker-timeline-qa-fixtures.json` and cover:

* Single speaker.
* Speaker switch.
* Overlapping cues.
* Missing visual cues.
* Ambiguous visible names.
* Manual correction.

The executable unit tests in `frontend/src-tauri/src/speaker.rs` and
`frontend/src-tauri/src/screenshots.rs` cover the core alignment, ambiguity,
review-only, OCR filtering, and structural privacy boundaries. The
`manual-correction` fixture documents the expected correction workflow and is
covered by the command-level correction implementation and undo review.
