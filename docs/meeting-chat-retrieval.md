# Meeting Chat Retrieval

Meetily meeting chat uses a local, rebuildable lexical index before routing a question to the selected meeting-summary model provider. The index keeps meeting data local unless the user has selected an external provider for answer generation.

## Indexed Artifacts

The `meeting_chat_index` table stores bounded text chunks for:

- `transcript`: transcript segments with timestamps and audio offsets.
- `summary`: generated meeting summaries and per-transcript summaries.
- `action_item`: extracted action item text.
- `key_point`: extracted key point text.
- `note`: user-authored meeting notes.
- `screenshot`: screenshot labels plus local metadata, with file paths for UI inspection.

Each row has a `source_type`, `source_id`, `source_label`, optional timestamp/audio offsets, optional file path, and chunk text. Chat citations refer back to these fields.

## Citation Format

Prompt citations use stable prefixes by source type:

- `[T1]`: transcript evidence.
- `[S1]`: meeting or segment summary.
- `[A1]`: action item.
- `[K1]`: key point.
- `[N1]`: meeting note.
- `[I1]`: screenshot artifact.

The API returns structured citation metadata alongside the answer so the UI can show timestamps, source labels, excerpts, and screenshot paths without parsing model text.

## Retrieval

The first implementation uses local lexical scoring because it is deterministic, dependency-free, and privacy-safe. Query terms are matched against indexed chunk text, with small boosts for source types that match question intent, such as action-item or screenshot questions.

Long meetings are handled by chunking index rows and selecting only the top bounded context rows. The chat prompt never sends the full transcript by default.

## Rebuild Behavior

The index is safe to rebuild:

1. Delete all `meeting_chat_index` rows for the meeting inside a transaction.
2. Re-read current transcripts, summaries, notes, and screenshot metadata.
3. Insert fresh chunks with updated citation metadata.

`meeting_chat_rebuild_index` exposes explicit rebuilds for the UI. `meeting_chat_ask` also rebuilds before answering so regenerated summaries, edited notes, screenshots, and late transcript rows are included without a separate invalidation step.

## Prompt Safety

Meeting content is treated as untrusted source material. System instructions tell the model to ignore instructions embedded in transcripts, notes, summaries, and screenshot text, and to answer only from supplied context.
