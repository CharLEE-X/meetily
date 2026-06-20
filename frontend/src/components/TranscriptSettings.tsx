import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Label } from './ui/label';
import { CheckCircle2, Eye, EyeOff, Lock, Unlock } from 'lucide-react';
import { ModelManager } from './WhisperModelManager';
import { ParakeetModelManager } from './ParakeetModelManager';
import { WhisperAPI } from '@/lib/whisper';
import { ParakeetAPI } from '@/lib/parakeet';
import {
    TRANSCRIPTION_QUALITY_PROFILES,
    buildTranscriptConfigFromProfile,
    getTranscriptionQualityProfile,
} from '@/lib/transcriptionProfiles';
import type { TranscriptionQualityProfile } from '@/lib/transcriptionProfiles';
import {
    TRANSCRIPTION_PREPROCESSING_PRESETS,
    TRANSCRIPTION_PREPROCESSING_STORAGE_KEY,
    getTranscriptionPreprocessingPreset,
} from '@/lib/transcriptionPreprocessing';
import type { TranscriptionPreprocessingPresetId } from '@/lib/transcriptionPreprocessing';


export interface TranscriptModelProps {
    provider: 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai';
    model: string;
    apiKey?: string | null;
}

export interface TranscriptSettingsProps {
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    onModelSelect?: () => void;
}

type ProfileReadiness = 'checking' | 'ready' | 'missing' | 'downloading' | 'error' | 'unknown';

export function TranscriptSettings({ transcriptModelConfig, setTranscriptModelConfig, onModelSelect }: TranscriptSettingsProps) {
    const [apiKey, setApiKey] = useState<string | null>(transcriptModelConfig.apiKey || null);
    const [showApiKey, setShowApiKey] = useState<boolean>(false);
    const [isApiKeyLocked, setIsApiKeyLocked] = useState<boolean>(true);
    const [isLockButtonVibrating, setIsLockButtonVibrating] = useState<boolean>(false);
    const [uiProvider, setUiProvider] = useState<TranscriptModelProps['provider']>(transcriptModelConfig.provider);
    const [profileReadiness, setProfileReadiness] = useState<Record<string, ProfileReadiness>>({});
    const [preprocessingPresetId, setPreprocessingPresetId] = useState<TranscriptionPreprocessingPresetId>(() => {
        if (typeof window === 'undefined') return 'balanced';
        return getTranscriptionPreprocessingPreset(window.localStorage.getItem(TRANSCRIPTION_PREPROCESSING_STORAGE_KEY)).id;
    });
    const activeProfile = getTranscriptionQualityProfile(transcriptModelConfig);
    const activePreprocessingPreset = getTranscriptionPreprocessingPreset(preprocessingPresetId);

    // Sync uiProvider when backend config changes (e.g., after model selection or initial load)
    useEffect(() => {
        setUiProvider(transcriptModelConfig.provider);
    }, [transcriptModelConfig.provider]);

    useEffect(() => {
        if (transcriptModelConfig.provider === 'localWhisper' || transcriptModelConfig.provider === 'parakeet') {
            setApiKey(null);
        }
    }, [transcriptModelConfig.provider]);

    useEffect(() => {
        let cancelled = false;

        const loadReadiness = async () => {
            const nextReadiness: Record<string, ProfileReadiness> = {};
            TRANSCRIPTION_QUALITY_PROFILES.forEach((profile) => {
                nextReadiness[profile.id] = 'checking';
            });
            setProfileReadiness((currentReadiness) => (
                Object.keys(currentReadiness).length > 0 ? currentReadiness : nextReadiness
            ));

            try {
                const [whisperModels, parakeetModels] = await Promise.all([
                    WhisperAPI.init().then(() => WhisperAPI.getAvailableModels()).catch(() => []),
                    ParakeetAPI.init().then(() => ParakeetAPI.getAvailableModels()).catch(() => []),
                ]);

                if (cancelled) return;

                const statusByProfile = TRANSCRIPTION_QUALITY_PROFILES.reduce<Record<string, ProfileReadiness>>((acc, profile) => {
                    const model = profile.provider === 'localWhisper'
                        ? whisperModels.find((candidate) => candidate.name === profile.model)
                        : parakeetModels.find((candidate) => candidate.name === profile.model);

                    if (!model) {
                        acc[profile.id] = 'unknown';
                    } else if (model.status === 'Available') {
                        acc[profile.id] = 'ready';
                    } else if (model.status === 'Missing') {
                        acc[profile.id] = 'missing';
                    } else if (model.status !== null && typeof model.status === 'object' && 'Downloading' in model.status) {
                        acc[profile.id] = 'downloading';
                    } else {
                        acc[profile.id] = 'error';
                    }
                    return acc;
                }, {});

                setProfileReadiness(statusByProfile);
            } catch (error) {
                console.error('Failed to load transcription profile readiness:', error);
            }
        };

        loadReadiness();
        const intervalId = window.setInterval(loadReadiness, 5000);

        return () => {
            cancelled = true;
            window.clearInterval(intervalId);
        };
    }, []);

    const fetchApiKey = async (provider: string) => {
        try {

            const data = await invoke('api_get_transcript_api_key', { provider }) as string;

            setApiKey(data || '');
        } catch (err) {
            console.error('Error fetching API key:', err);
            setApiKey(null);
        }
    };
    const modelOptions = {
        localWhisper: [], // Model selection handled by ModelManager component
        parakeet: [], // Model selection handled by ParakeetModelManager component
        deepgram: ['nova-2-phonecall'],
        elevenLabs: ['eleven_multilingual_v2'],
        groq: ['llama-3.3-70b-versatile'],
        openai: ['gpt-4o'],
    };
    const requiresApiKey = transcriptModelConfig.provider === 'deepgram' || transcriptModelConfig.provider === 'elevenLabs' || transcriptModelConfig.provider === 'openai' || transcriptModelConfig.provider === 'groq';

    const handleInputClick = () => {
        if (isApiKeyLocked) {
            setIsLockButtonVibrating(true);
            setTimeout(() => setIsLockButtonVibrating(false), 500);
        }
    };

    const handleWhisperModelSelect = (modelName: string) => {
        // Always update config when model is selected, regardless of current provider
        // This ensures the model is set when user switches back
        setTranscriptModelConfig({
            ...transcriptModelConfig,
            provider: 'localWhisper', // Ensure provider is set correctly
            model: modelName
        });
        // Close modal after selection
        if (onModelSelect) {
            onModelSelect();
        }
    };

    const handleParakeetModelSelect = (modelName: string) => {
        // Always update config when model is selected, regardless of current provider
        // This ensures the model is set when user switches back
        setTranscriptModelConfig({
            ...transcriptModelConfig,
            provider: 'parakeet', // Ensure provider is set correctly
            model: modelName
        });
        // Close modal after selection
        if (onModelSelect) {
            onModelSelect();
        }
    };

    const handleProfileSelect = async (profile: TranscriptionQualityProfile) => {
        const nextConfig = buildTranscriptConfigFromProfile(profile, transcriptModelConfig);
        setUiProvider(profile.provider);
        setTranscriptModelConfig(nextConfig as TranscriptModelProps);

        try {
            await invoke('api_save_transcript_config', {
                provider: nextConfig.provider,
                model: nextConfig.model,
                apiKey: null,
            });
        } catch (error) {
            console.error('Failed to save transcription quality profile:', error);
        }
    };

    const getProfileReadinessLabel = (readiness: ProfileReadiness | undefined) => {
        switch (readiness) {
            case 'ready':
                return { label: 'Ready', className: 'bg-green-50 text-green-700 ring-green-200' };
            case 'missing':
                return { label: 'Download required', className: 'bg-amber-50 text-amber-700 ring-amber-200' };
            case 'downloading':
                return { label: 'Downloading', className: 'bg-blue-50 text-blue-700 ring-blue-200' };
            case 'error':
                return { label: 'Needs attention', className: 'bg-red-50 text-red-700 ring-red-200' };
            case 'checking':
                return { label: 'Checking', className: 'bg-gray-50 text-gray-600 ring-gray-200' };
            default:
                return { label: 'Check model below', className: 'bg-gray-50 text-gray-600 ring-gray-200' };
        }
    };

    const handlePreprocessingPresetSelect = (presetId: TranscriptionPreprocessingPresetId) => {
        setPreprocessingPresetId(presetId);
        if (typeof window !== 'undefined') {
            window.localStorage.setItem(TRANSCRIPTION_PREPROCESSING_STORAGE_KEY, presetId);
        }
    };

    return (
        <div className="space-y-6">
            <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
                <h3 className="text-lg font-semibold text-gray-900">Transcription settings</h3>
                <p className="mt-2 max-w-3xl text-sm leading-6 text-gray-600">
                    Choose the speech-to-text engine that creates live and saved transcripts from meeting audio. Local engines keep transcription on this Mac and require downloaded model files; cloud engines, when enabled, require an API key and may send audio or transcript context to that provider.
                </p>
                <div className="mt-4 grid gap-3 text-sm text-gray-600 md:grid-cols-2">
                    <div className="rounded-md bg-gray-50 p-3 ring-1 ring-gray-200">
                        <span className="font-medium text-gray-900">Parakeet:</span> optimized for fast local streaming and day-to-day meeting capture.
                    </div>
                    <div className="rounded-md bg-gray-50 p-3 ring-1 ring-gray-200">
                        <span className="font-medium text-gray-900">Local Whisper:</span> a reliable fallback when you prefer Whisper model compatibility or accuracy.
                    </div>
                </div>
            </div>
            <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
                <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                    <div>
                        <h3 className="text-lg font-semibold text-gray-900">Quality profile</h3>
                        <p className="mt-2 max-w-3xl text-sm leading-6 text-gray-600">
                            Profiles choose a tested local provider and model combination for new recordings. They are shortcuts over the model managers below, so missing models still use the same download and readiness flow.
                        </p>
                    </div>
                    {activeProfile ? (
                        <div className="inline-flex items-center gap-2 rounded-full bg-green-50 px-3 py-1 text-sm font-medium text-green-700 ring-1 ring-green-200">
                            <CheckCircle2 className="h-4 w-4" />
                            {activeProfile.name}
                        </div>
                    ) : (
                        <div className="rounded-full bg-gray-100 px-3 py-1 text-sm font-medium text-gray-600 ring-1 ring-gray-200">
                            Custom model
                        </div>
                    )}
                </div>
                <div className="mt-5 grid gap-3 lg:grid-cols-3">
                    {TRANSCRIPTION_QUALITY_PROFILES.map((profile) => {
                        const isSelected = activeProfile?.id === profile.id;
                        const readiness = getProfileReadinessLabel(profileReadiness[profile.id]);
                        return (
                            <button
                                key={profile.id}
                                type="button"
                                onClick={() => handleProfileSelect(profile)}
                                className={`rounded-lg border p-4 text-left transition hover:border-blue-300 hover:bg-blue-50/40 ${isSelected
                                    ? 'border-blue-500 bg-blue-50 ring-1 ring-blue-200'
                                    : 'border-gray-200 bg-white'
                                    }`}
                            >
                                <div className="flex items-start justify-between gap-3">
                                    <div>
                                        <div className="text-sm font-semibold text-gray-900">{profile.name}</div>
                                        <div className="mt-1 text-xs font-medium uppercase tracking-wide text-blue-700">{profile.badge}</div>
                                    </div>
                                    <div className="flex flex-col items-end gap-2">
                                        {isSelected && <CheckCircle2 className="h-5 w-5 shrink-0 text-blue-600" />}
                                        <span className={`rounded-full px-2 py-0.5 text-xs font-medium ring-1 ${readiness.className}`}>
                                            {readiness.label}
                                        </span>
                                    </div>
                                </div>
                                <p className="mt-3 text-sm leading-6 text-gray-600">{profile.summary}</p>
                                {profileReadiness[profile.id] === 'missing' && (
                                    <p className="mt-2 rounded-md bg-amber-50 px-2 py-1 text-xs text-amber-800 ring-1 ring-amber-100">
                                        Select this profile, then use the model manager below to download it before recording.
                                    </p>
                                )}
                                <dl className="mt-4 space-y-2 text-xs leading-5 text-gray-600">
                                    <div>
                                        <dt className="font-medium text-gray-900">Best for</dt>
                                        <dd>{profile.bestFor}</dd>
                                    </div>
                                    <div>
                                        <dt className="font-medium text-gray-900">Tradeoff</dt>
                                        <dd>{profile.tradeoff}</dd>
                                    </div>
                                    <div className="flex items-center justify-between rounded-md bg-gray-50 px-2 py-1 ring-1 ring-gray-100">
                                        <dt className="font-medium text-gray-900">Model</dt>
                                        <dd className="text-right">{profile.model} ({profile.sizeLabel})</dd>
                                    </div>
                                </dl>
                            </button>
                        );
                    })}
                </div>
            </div>
            <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
                <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                    <div>
                        <h3 className="text-lg font-semibold text-gray-900">Preprocessing preset</h3>
                        <p className="mt-2 max-w-3xl text-sm leading-6 text-gray-600">
                            These presets describe how to run the current native capture path for better accuracy checks. Balanced keeps the production defaults: resampling, high-pass filtering, loudness normalization, and VAD pause bridging.
                        </p>
                    </div>
                    <div className="rounded-full bg-gray-100 px-3 py-1 text-sm font-medium text-gray-700 ring-1 ring-gray-200">
                        {activePreprocessingPreset.name}
                    </div>
                </div>
                <div className="mt-5 grid gap-3 lg:grid-cols-3">
                    {TRANSCRIPTION_PREPROCESSING_PRESETS.map((preset) => {
                        const isSelected = activePreprocessingPreset.id === preset.id;
                        return (
                            <button
                                key={preset.id}
                                type="button"
                                onClick={() => handlePreprocessingPresetSelect(preset.id)}
                                className={`rounded-lg border p-4 text-left transition hover:border-blue-300 hover:bg-blue-50/40 ${isSelected
                                    ? 'border-blue-500 bg-blue-50 ring-1 ring-blue-200'
                                    : 'border-gray-200 bg-white'
                                    }`}
                            >
                                <div className="flex items-start justify-between gap-3">
                                    <div>
                                        <div className="text-sm font-semibold text-gray-900">{preset.name}</div>
                                        <div className="mt-1 text-xs font-medium uppercase tracking-wide text-blue-700">{preset.badge}</div>
                                    </div>
                                    {isSelected && <CheckCircle2 className="h-5 w-5 shrink-0 text-blue-600" />}
                                </div>
                                <p className="mt-3 text-sm leading-6 text-gray-600">{preset.summary}</p>
                                <div className="mt-4 text-xs leading-5 text-gray-600">
                                    <div className="font-medium text-gray-900">Best for</div>
                                    <p>{preset.bestFor}</p>
                                    <div className="mt-3 font-medium text-gray-900">Pipeline notes</div>
                                    <ul className="mt-1 list-disc space-y-1 pl-4">
                                        {preset.pipeline.map((step) => (
                                            <li key={step}>{step}</li>
                                        ))}
                                    </ul>
                                </div>
                            </button>
                        );
                    })}
                </div>
                {transcriptModelConfig.provider === 'parakeet' && (
                    <p className="mt-4 rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800 ring-1 ring-amber-100">
                        Parakeet uses automatic language detection. Choose a Whisper profile when you need to pin a specific language or use auto-translate to English.
                    </p>
                )}
            </div>
            <div>
                {/* <div className="flex justify-between items-center mb-4">
                    <h3 className="text-lg font-semibold text-gray-900">Transcript Settings</h3>
                </div> */}
                <div className="space-y-4 rounded-lg border border-gray-200 bg-white p-6 pb-6 shadow-sm">
                    <div>
                        <Label className="block text-sm font-medium text-gray-700 mb-1">
                            Transcript Model
                        </Label>
                        <p className="mb-3 max-w-3xl text-sm leading-6 text-gray-600">
                            This selection controls new recording sessions. Model managers below download, select, and verify the local files needed before transcription can run reliably.
                        </p>
                        <div className="flex space-x-2 mx-1">
                            <Select
                                value={uiProvider}
                                onValueChange={(value) => {
                                    const provider = value as TranscriptModelProps['provider'];
                                    setUiProvider(provider);
                                    if (provider !== 'localWhisper' && provider !== 'parakeet') {
                                        fetchApiKey(provider);
                                    }
                                }}
                            >
                                <SelectTrigger className='focus:ring-1 focus:ring-blue-500 focus:border-blue-500'>
                                    <SelectValue placeholder="Select provider" />
                                </SelectTrigger>
                                <SelectContent>
                                    <SelectItem value="parakeet">⚡ Parakeet (Recommended - Real-time / Accurate)</SelectItem>
                                    <SelectItem value="localWhisper">🏠 Local Whisper (High Accuracy)</SelectItem>
                                    {/* <SelectItem value="deepgram">☁️ Deepgram (Backup)</SelectItem>
                                    <SelectItem value="elevenLabs">☁️ ElevenLabs</SelectItem>
                                    <SelectItem value="groq">☁️ Groq</SelectItem>
                                    <SelectItem value="openai">☁️ OpenAI</SelectItem> */}
                                </SelectContent>
                            </Select>

                            {uiProvider !== 'localWhisper' && uiProvider !== 'parakeet' && (
                                <Select
                                    value={transcriptModelConfig.model}
                                    onValueChange={(value) => {
                                        const model = value as TranscriptModelProps['model'];
                                        setTranscriptModelConfig({ ...transcriptModelConfig, provider: uiProvider, model });
                                    }}
                                >
                                    <SelectTrigger className='focus:ring-1 focus:ring-blue-500 focus:border-blue-500'>
                                        <SelectValue placeholder="Select model" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        {modelOptions[uiProvider].map((model) => (
                                            <SelectItem key={model} value={model}>{model}</SelectItem>
                                        ))}
                                    </SelectContent>
                                </Select>
                            )}

                        </div>
                    </div>

                    {uiProvider === 'localWhisper' && (
                        <div className="mt-6">
                            <ModelManager
                                selectedModel={transcriptModelConfig.provider === 'localWhisper' ? transcriptModelConfig.model : undefined}
                                onModelSelect={handleWhisperModelSelect}
                                autoSave={true}
                            />
                        </div>
                    )}

                    {uiProvider === 'parakeet' && (
                        <div className="mt-6">
                            <ParakeetModelManager
                                selectedModel={transcriptModelConfig.provider === 'parakeet' ? transcriptModelConfig.model : undefined}
                                onModelSelect={handleParakeetModelSelect}
                                autoSave={true}
                            />
                        </div>
                    )}


                    {requiresApiKey && (
                        <div>
                            <Label className="block text-sm font-medium text-gray-700 mb-1">
                                API Key
                            </Label>
                            <p className="mb-3 text-sm leading-6 text-gray-600">
                                API keys are stored locally and are only used for the selected cloud transcription provider. Local Whisper and Parakeet do not need an API key.
                            </p>
                            <div className="relative mx-1">
                                <Input
                                    type={showApiKey ? "text" : "password"}
                                    className={`pr-24 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 ${isApiKeyLocked ? 'bg-gray-100 cursor-not-allowed' : ''
                                        }`}
                                    value={apiKey || ''}
                                    onChange={(e) => setApiKey(e.target.value)}
                                    disabled={isApiKeyLocked}
                                    onClick={handleInputClick}
                                    placeholder="Enter your API key"
                                />
                                {isApiKeyLocked && (
                                    <div
                                        onClick={handleInputClick}
                                        className="absolute inset-0 flex items-center justify-center bg-gray-100 bg-opacity-50 rounded-md cursor-not-allowed"
                                    />
                                )}
                                <div className="absolute inset-y-0 right-0 pr-1 flex items-center">
                                    <Button
                                        type="button"
                                        variant="ghost"
                                        size="icon"
                                        onClick={() => setIsApiKeyLocked(!isApiKeyLocked)}
                                        className={`transition-colors duration-200 ${isLockButtonVibrating ? 'animate-vibrate text-red-500' : ''
                                            }`}
                                        title={isApiKeyLocked ? "Unlock to edit" : "Lock to prevent editing"}
                                    >
                                        {isApiKeyLocked ? <Lock className="h-4 w-4" /> : <Unlock className="h-4 w-4" />}
                                    </Button>
                                    <Button
                                        type="button"
                                        variant="ghost"
                                        size="icon"
                                        onClick={() => setShowApiKey(!showApiKey)}
                                    >
                                        {showApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                    </Button>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div >
    )
}

