# Transcription Accuracy Baseline

This is the CHA-1675 baseline for improving Meetily Community transcription
accuracy while keeping local-first behavior. It maps the current pipeline,
defines repeatable benchmark fixtures, and records the known quality and
stability gaps that CHA-1676 and CHA-1677 should address.

## Current Pipeline Map

### Live recording path

1. `frontend/src-tauri/src/audio/recording_commands.rs` resolves preferred or
   default microphone and system-audio devices, validates the selected
   transcription model before recording starts, then starts `RecordingManager`.
2. `frontend/src-tauri/src/audio/pipeline.rs` receives microphone/system chunks,
   resamples to the pipeline target when needed, mixes streams, applies VAD, and
   emits speech chunks for transcription. Shutdown sends flush chunks so
   remaining VAD-buffered speech is processed.
3. `frontend/src-tauri/src/audio/transcription/worker.rs` starts one worker in
   serial mode. This intentionally favors chronological transcript emission over
   throughput. It resamples each speech chunk to 16 kHz if needed, calls the
   selected engine, applies provider-specific confidence filtering, and emits
   `transcript-update`, `transcription-progress`, and chunk-loss diagnostics.
4. Frontend transcript state sorts updates by recording-relative timestamp and
   sequence id before rendering and persistence.

### Retranscription/import path

1. `frontend/src-tauri/src/audio/retranscription.rs` finds the saved audio file,
   decodes it, converts it to 16 kHz mono, then runs VAD with a longer redemption
   time than live recording.
2. Very long speech segments are split at low-energy boundaries before model
   inference to reduce word loss at arbitrary cuts.
3. The selected provider transcribes each segment, then the module replaces the
   meeting transcripts in a database transaction and rewrites transcript files.

### Provider and model paths

- `frontend/src-tauri/src/audio/transcription/engine.rs` chooses Parakeet or
  Whisper from persisted transcript config. Unsupported cloud providers are
  rejected for local recording.
- `frontend/src-tauri/src/whisper_engine/commands.rs` discovers Whisper files in
  the app models directory, emits loading started/completed/failed events, and
  validates readiness from the configured model.
- `frontend/src-tauri/src/parakeet_engine/commands.rs` discovers Parakeet model
  directories, prefers the int8 model when auto-loading, emits Parakeet-specific
  loading events, and validates readiness.
- `frontend/src-tauri/src/config.rs` currently defines `large-v3-turbo` as the
  default Whisper model and `parakeet-tdt-0.6b-v3-int8` as the default Parakeet
  model.
- `frontend/src/components/TranscriptSettings.tsx` exposes provider selection
  and delegates local model download/selection to the Whisper and Parakeet model
  manager components.

## Baseline Model Behavior

| Provider | Current default | Strength | Known gap |
| --- | --- | --- | --- |
| Parakeet | `parakeet-tdt-0.6b-v3-int8` | Fast local streaming and small memory footprint. | No confidence score; manual language selection is not supported. |
| Local Whisper | `large-v3-turbo` | Higher accuracy profile with manual language support. | Larger download, slower load/inference, and higher memory use. |

The live worker accepts all Parakeet non-empty output because Parakeet does not
return confidence. Whisper output is filtered at `0.3` confidence. Provider
confidence semantics are therefore not comparable yet.

## Benchmark Fixture Manifest

The repeatable fixture manifest lives at
`docs/transcription-benchmark-fixtures.json`. It defines the minimum benchmark
set needed before and after transcription changes:

- `short-clean-en`: provider/model smoke test and readiness path.
- `long-meeting-en`: one-hour stability, memory, ordering, and timestamp drift.
- `noisy-laptop-en`: preprocessing sensitivity for fan noise, keyboard noise,
  clipping, and VAD fragmentation.
- `multi-speaker-overlap-en`: overlapping speech and multi-speaker turn-taking.
- `non-english-es`: Whisper language override and Parakeet auto-language
  limitation.

Fixture audio and reference transcripts are intentionally referenced by path
instead of committed here. Audio fixtures can be large and may contain voice data;
they should live in a controlled local QA fixture directory or artifact store.

## Metrics To Record

Each benchmark run should record:

- provider, model, profile, platform, CPU/GPU backend, and app version;
- fixture id and duration;
- word error rate and character error rate against the reference transcript;
- realtime factor for inference;
- peak RSS memory in MB;
- emitted segment count and empty segment count;
- average confidence when the provider exposes confidence;
- timestamp drift at start, middle, and end of the fixture;
- failure mode, user-facing error, and whether recovery was possible.

## Current Bottlenecks And Quality Gaps

- The live worker is serial. This preserves order and avoids chunk reordering
  bugs, but it can lag during long or high-throughput sessions.
- Live and retranscription VAD redemption times differ. This is intentional, but
  it means live and batch results can segment the same audio differently.
- Confidence is only meaningful for Whisper today. Parakeet uses an implicit
  default confidence in downstream payloads, so quality dashboards must treat it
  as unavailable rather than comparable.
- Manual language selection applies to Whisper. Parakeet warns and ignores the
  language preference.
- Model readiness states exist, but the user sees provider-specific events and
  labels rather than one consistent quality/speed profile model.
- Large Whisper models can improve accuracy but create download, load, and memory
  risk. The next issue must expose the tradeoff clearly and keep the current
  default path working.
- Long-meeting stability depends on VAD flushing, chunk count verification, and
  transaction-safe retranscription writes. These are the key regression points
  for later changes.

## Follow-Up Targets

- CHA-1676 should add explicit fast, balanced, and high-accuracy profile metadata
  over the existing local models and preserve current defaults.
- CHA-1676 should normalize model readiness/download/missing labels across
  Parakeet and Whisper without enabling cloud transcription by default.
- CHA-1677 should add selectable preprocessing presets and use this fixture suite
  to compare noise, clipping, VAD fragmentation, and timestamp drift.
- CHA-1677 should keep timestamp continuity checks green for long meetings and
  imported/retranscribed audio.
