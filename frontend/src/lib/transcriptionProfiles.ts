export type TranscriptProvider = 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai';

export interface TranscriptModelConfigLike {
  provider: TranscriptProvider;
  model: string;
  apiKey?: string | null;
}

export type TranscriptionQualityProfileId = 'fast' | 'balanced' | 'high-accuracy';

export interface TranscriptionQualityProfile {
  id: TranscriptionQualityProfileId;
  name: string;
  provider: TranscriptProvider;
  model: string;
  badge: string;
  summary: string;
  bestFor: string;
  tradeoff: string;
  sizeLabel: string;
}

export const TRANSCRIPTION_QUALITY_PROFILES: TranscriptionQualityProfile[] = [
  {
    id: 'fast',
    name: 'Fast live meetings',
    provider: 'parakeet',
    model: 'parakeet-tdt-0.6b-v3-int8',
    badge: 'Default',
    summary: 'Optimized for real-time local capture with strong accuracy and low latency.',
    bestFor: 'Daily calls, standups, and long meetings where lag matters.',
    tradeoff: 'Parakeet does not expose Whisper-style confidence scores or manual language overrides.',
    sizeLabel: '~670 MB',
  },
  {
    id: 'balanced',
    name: 'Balanced accuracy',
    provider: 'localWhisper',
    model: 'large-v3-turbo-q5_0',
    badge: 'Recommended accuracy',
    summary: 'Whisper large-v3-turbo in a smaller quantized package for better quality without the largest download.',
    bestFor: 'Important meetings where accuracy matters but runtime should remain practical.',
    tradeoff: 'Slower than Parakeet and still requires a local model download.',
    sizeLabel: '~547 MB',
  },
  {
    id: 'high-accuracy',
    name: 'Highest accuracy',
    provider: 'localWhisper',
    model: 'large-v3',
    badge: 'Best quality',
    summary: 'Full large-v3 Whisper model for the strongest local recognition quality.',
    bestFor: 'Critical recordings, difficult audio, and post-meeting retranscription.',
    tradeoff: 'Largest download and slowest processing path.',
    sizeLabel: '~2.9 GB',
  },
];

export function getTranscriptionQualityProfile(
  config: Pick<TranscriptModelConfigLike, 'provider' | 'model'>,
): TranscriptionQualityProfile | undefined {
  return TRANSCRIPTION_QUALITY_PROFILES.find(
    (profile) => profile.provider === config.provider && profile.model === config.model,
  );
}

export function buildTranscriptConfigFromProfile(
  profile: TranscriptionQualityProfile,
  currentConfig: TranscriptModelConfigLike,
): TranscriptModelConfigLike {
  return {
    ...currentConfig,
    provider: profile.provider,
    model: profile.model,
    apiKey: null,
  };
}
