# Meeting Chat Release Notes

This release adds source-cited chat for completed meetings. Users can ask
follow-up questions from the meeting details screen and inspect the transcript,
summary, action-item, note, or screenshot context used for each answer.

## Included

* Summary/Chat switch in meeting details.
* Persisted chat history per meeting.
* Suggested first questions for common follow-ups.
* Local retrieval index over transcripts, summaries, action items, key points,
  notes, and enabled screenshot metadata.
* Structured citations with source type, timestamp or artifact label, excerpt,
  and local file path when available.
* Loading, cancel, retry, empty transcript, and missing model states.
* Bounded context selection for long meetings so chat does not send full meeting
  transcripts by default.

## Provider And Privacy Behavior

Meeting chat uses Meetily's local SQLite data and local retrieval index first.
The index remains on the device and is rebuilt from local meeting artifacts
before each answer.

Answer generation uses the AI model provider selected in Meetily settings:

* Local providers, such as Ollama or the built-in local model path, keep the chat
  prompt on the user's machine.
* Cloud providers, such as Claude, OpenAI, Groq, OpenRouter, or a custom
  OpenAI-compatible endpoint, receive only the bounded selected context needed
  to answer the question.

Meeting context is treated as untrusted source material. The chat prompt tells
the model to ignore instructions embedded in transcripts, notes, summaries, and
screenshot text. Context excerpts are wrapped in sanitized source blocks before
being sent to the selected model.

## QA Matrix

| Scenario | Expected result |
| --- | --- |
| Short meeting with transcript | Chat answers with transcript citations and persisted history. |
| Long meeting | Retrieval selects bounded chunks instead of sending the full transcript. |
| Empty meeting | Chat shows a transcript-required state and cannot submit a question. |
| Meeting with summary/actions | Answers can cite summary and action-item sources. |
| Meeting with screenshot metadata | Screenshot citations show labels and local file paths when available. |
| Missing or unsaved model | Chat shows a model-required state before submit. |
| Cancel in-flight answer | The pending bubble changes to canceled and the input is released. |
| Failed provider request | The failed answer exposes a retry action tied to the original question. |
| Tab switch during answer | Chat remains mounted and keeps the in-flight/pending state. |
| Prompt boundary injection text | Context block delimiters are sanitized before model routing. |

## Verification Notes

Checks used for this implementation:

* `cargo test --manifest-path frontend/src-tauri/Cargo.toml --lib meeting_chat`
* `pnpm --dir frontend run build`
* `git diff --check` for whitespace validation

Known repository-level Rust warnings remain in unrelated macOS screenshot and
audio test code paths. They do not block the meeting chat surface.
