# Active Speaker Timeline Release Notes

This release adds opt-in active speaker timeline support for call-window
snapshot workflows.

## Added

* Suggested and confirmed speaker labels in the transcript.
* Confidence indicators for screenshot-assisted labels.
* Visual speaker suggestions derived from local call-window display names and
  provider active-speaker UI markers.
* User correction controls for accept, rename, assign, merge, clear generated
  labels, and undo recent corrections.
* QA fixtures and docs covering single-speaker, speaker-switch, overlapping,
  missing, ambiguous, and manual-correction cases.

## Privacy

Snapshot-assisted speaker labeling is local and opt-in. It uses bounded meeting
UI evidence only: visible display names, provider speaker markers, recording
time, transcript timing, provider id, and confidence. The implementation has no
face-recognition or face-matching dependency path, and candidate OCR text that
looks like a face-recognition or identity-inference claim is rejected as speaker
evidence.

Generated labels are suggestions until the user confirms or edits them.
Confirmed/manual labels remain available if generated labels are cleared.

## QA Notes

Run `cargo test speaker --lib` to exercise active-speaker alignment, confidence,
review-only behavior, ambiguous cues, screenshot cue extraction, and privacy
boundary tests. Manual provider QA should still verify Google Meet, Zoom,
Teams, Slack, FaceTime, and Webex capture behavior before external release.
