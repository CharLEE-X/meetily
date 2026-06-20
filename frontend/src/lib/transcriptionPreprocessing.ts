export type TranscriptionPreprocessingPresetId = 'balanced' | 'noisy-room' | 'raw-capture';

export interface TranscriptionPreprocessingPreset {
  id: TranscriptionPreprocessingPresetId;
  name: string;
  badge: string;
  summary: string;
  pipeline: string[];
  bestFor: string;
}

export const TRANSCRIPTION_PREPROCESSING_STORAGE_KEY = 'transcriptionPreprocessingPreset';

export const TRANSCRIPTION_PREPROCESSING_PRESETS: TranscriptionPreprocessingPreset[] = [
  {
    id: 'balanced',
    name: 'Balanced clarity',
    badge: 'Default',
    summary: 'Uses the current production path: resampling, high-pass filtering, loudness normalization, and VAD segmentation.',
    pipeline: ['48 kHz capture normalization', '80 Hz high-pass filter', 'EBU R128 loudness normalization', 'Silero VAD with pause bridging'],
    bestFor: 'Most meetings, mixed mic/system audio, and long calls.',
  },
  {
    id: 'noisy-room',
    name: 'Noisy room',
    badge: 'Setup guidance',
    summary: 'Keeps the native preprocessing path stable while guiding laptop mics, fan noise, and keyboard-heavy calls toward better model and language choices.',
    pipeline: ['Prefer headset or close mic', 'Use a specific Whisper language when possible', 'Keep VAD pause bridging', 'Review benchmark noisy-laptop fixture'],
    bestFor: 'Open offices, laptop microphones, and calls with background noise.',
  },
  {
    id: 'raw-capture',
    name: 'Raw capture',
    badge: 'Diagnostic',
    summary: 'Keeps the default native capture behavior but frames the session as a diagnostic run for comparing transcripts against the baseline fixture suite.',
    pipeline: ['Preserve default capture path', 'Avoid changing language mid-run', 'Compare WER/CER and timestamp drift', 'Use retranscription for controlled model comparisons'],
    bestFor: 'Debugging audio devices, timestamp continuity, and before/after benchmark runs.',
  },
];

export function isTranscriptionPreprocessingPresetId(value: string | null): value is TranscriptionPreprocessingPresetId {
  return value === 'balanced' || value === 'noisy-room' || value === 'raw-capture';
}

export function getTranscriptionPreprocessingPreset(
  presetId: string | null,
): TranscriptionPreprocessingPreset {
  const resolvedId = isTranscriptionPreprocessingPresetId(presetId) ? presetId : 'balanced';
  return TRANSCRIPTION_PREPROCESSING_PRESETS.find((preset) => preset.id === resolvedId)
    ?? TRANSCRIPTION_PREPROCESSING_PRESETS[0];
}
