import assert from 'node:assert/strict';
import test from 'node:test';

import type * as TranscriptionProfilesModule from './transcriptionProfiles';

const {
  TRANSCRIPTION_QUALITY_PROFILES,
  buildTranscriptConfigFromProfile,
  getTranscriptionQualityProfile,
} = await import(new URL('./transcriptionProfiles.ts', import.meta.url).href) as typeof TranscriptionProfilesModule;

test('defines fast, balanced, and high-accuracy transcription profiles', () => {
  assert.deepEqual(
    TRANSCRIPTION_QUALITY_PROFILES.map((profile) => profile.id),
    ['fast', 'balanced', 'high-accuracy'],
  );
});

test('keeps the existing Parakeet int8 default mapped to the fast profile', () => {
  const profile = getTranscriptionQualityProfile({
    provider: 'parakeet',
    model: 'parakeet-tdt-0.6b-v3-int8',
  });

  assert.equal(profile?.id, 'fast');
});

test('builds a saved transcript config from a profile without carrying API keys', () => {
  const balancedProfile = TRANSCRIPTION_QUALITY_PROFILES.find((profile) => profile.id === 'balanced');
  assert.ok(balancedProfile);

  const config = buildTranscriptConfigFromProfile(balancedProfile, {
    provider: 'deepgram',
    model: 'nova-2-phonecall',
    apiKey: 'secret',
  });

  assert.equal(config.provider, 'localWhisper');
  assert.equal(config.model, 'large-v3-turbo-q5_0');
  assert.equal(config.apiKey, null);
});
