import assert from 'node:assert/strict';
import test from 'node:test';

import type * as TranscriptionPreprocessingModule from './transcriptionPreprocessing';

const {
  TRANSCRIPTION_PREPROCESSING_PRESETS,
  getTranscriptionPreprocessingPreset,
  isTranscriptionPreprocessingPresetId,
} = await import(new URL('./transcriptionPreprocessing.ts', import.meta.url).href) as typeof TranscriptionPreprocessingModule;

test('keeps balanced clarity as the default preprocessing preset', () => {
  assert.equal(getTranscriptionPreprocessingPreset(null).id, 'balanced');
  assert.equal(TRANSCRIPTION_PREPROCESSING_PRESETS[0].id, 'balanced');
});

test('validates known preprocessing preset ids', () => {
  assert.equal(isTranscriptionPreprocessingPresetId('noisy-room'), true);
  assert.equal(isTranscriptionPreprocessingPresetId('raw-capture'), true);
  assert.equal(isTranscriptionPreprocessingPresetId('unsupported'), false);
});

test('falls back to balanced for unknown persisted values', () => {
  assert.equal(getTranscriptionPreprocessingPreset('unsupported').id, 'balanced');
});
