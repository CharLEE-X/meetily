import type { MeetingJoinCandidate, MeetingProvider } from './meetingDetectionService';

export interface BrowserMeetingSignal {
  browser: string;
  provider?: MeetingProvider;
  title: string | null;
  url: string | null;
  isActive: boolean;
  permissionStatus?: SignalPermissionStatus;
  checkedAt?: string;
  freshnessMs?: number;
}

export interface MicActivitySignal {
  isActive: boolean;
  peakLevel: number;
  rmsLevel: number;
}

export type SignalPermissionStatus = 'available' | 'limited' | 'denied' | 'unknown';

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface NativeMeetingActivitySignals {
  activeAppName: string | null;
  activeWindowTitle: string | null;
  activeWindowBounds?: WindowBounds | null;
  runningApps: string[];
  browserTabs: BrowserMeetingSignal[];
  checkedAt: string;
  missingPermissions?: string[];
  permissionStatus?: Record<string, SignalPermissionStatus>;
  signalFreshnessMs?: number;
  degradedMode?: boolean;
  error?: string | null;
}

export interface MeetingActivitySignals extends NativeMeetingActivitySignals {
  micActivity?: MicActivitySignal | null;
}

export interface MeetingActivityScore {
  isLikelyMeeting: boolean;
  confidence: number;
  provider: MeetingProvider;
  title: string;
  meetingUrl: string | null;
  reasons: string[];
}

export interface AmbientMeetingDetectionOptions {
  minimumConfidence: number;
  windowMinutes: number;
}

const DEFAULT_OPTIONS: AmbientMeetingDetectionOptions = {
  minimumConfidence: 65,
  windowMinutes: 90,
};

const MEETING_PROCESS_HINTS = [
  { provider: 'teams' as const, terms: ['microsoft teams', 'teams'] },
  { provider: 'zoom' as const, terms: ['zoom.us', 'zoom'] },
  { provider: 'google-meet' as const, terms: ['google chrome', 'chrome', 'arc', 'safari', 'microsoft edge', 'msedge', 'firefox'] },
];

const MEETING_TEXT_HINTS = [
  { provider: 'google-meet' as const, terms: ['meet.google.com', 'google meet'] },
  { provider: 'teams' as const, terms: ['teams.microsoft.com', 'teams.live.com', 'microsoft teams', 'teams meeting'] },
  { provider: 'zoom' as const, terms: ['zoom.us', 'zoom.com', 'zoom meeting'] },
  { provider: 'slack' as const, terms: ['slack.com/huddle', 'slack://huddle', 'slack huddle', 'slack call'] },
];

const MEETING_URL_PATTERN = /https?:\/\/(?:[^\s<>"')]+)/i;

function normalize(value: string | null | undefined): string {
  return (value ?? '').toLowerCase();
}

function clampScore(score: number): number {
  return Math.max(0, Math.min(100, Math.round(score)));
}

export function detectMeetingProviderFromText(text: string | null | undefined): MeetingProvider {
  const normalized = normalize(text);
  if (normalized.includes('slack') && (normalized.includes('huddle') || normalized.includes('call'))) {
    return 'slack';
  }
  for (const hint of MEETING_TEXT_HINTS) {
    if (hint.terms.some((term) => normalized.includes(term))) {
      return hint.provider;
    }
  }
  return 'unknown';
}

function detectProviderFromProcesses(appNames: string[]): MeetingProvider {
  const normalizedApps = appNames.map(normalize);
  for (const hint of MEETING_PROCESS_HINTS) {
    if (normalizedApps.some((name) => hint.terms.some((term) => name.includes(term)))) {
      return hint.provider;
    }
  }
  return 'unknown';
}

function detectProviderFromBrowserSignals(tabs: BrowserMeetingSignal[]): MeetingProvider {
  for (const tab of tabs) {
    if (tab.provider && tab.provider !== 'unknown') return tab.provider;
    const provider = detectMeetingProviderFromText(`${tab.title ?? ''} ${tab.url ?? ''}`);
    if (provider !== 'unknown') return provider;
  }
  return 'unknown';
}

function extractMeetingUrlFromSignals(signals: MeetingActivitySignals): string | null {
  for (const tab of signals.browserTabs) {
    const url = tab.url?.match(MEETING_URL_PATTERN)?.[0] ?? null;
    if (url && detectMeetingProviderFromText(url) !== 'unknown') return url.replace(/[.,;]+$/, '');
  }

  const windowUrl = signals.activeWindowTitle?.match(MEETING_URL_PATTERN)?.[0] ?? null;
  return windowUrl && detectMeetingProviderFromText(windowUrl) !== 'unknown'
    ? windowUrl.replace(/[.,;]+$/, '')
    : null;
}

function titleFromSignals(signals: MeetingActivitySignals, provider: MeetingProvider): string {
  const activeTab = signals.browserTabs.find((tab) => tab.isActive && tab.title);
  if (activeTab?.title) return activeTab.title;
  if (signals.activeWindowTitle) return signals.activeWindowTitle;
  switch (provider) {
    case 'google-meet':
      return 'Google Meet call';
    case 'teams':
      return 'Microsoft Teams call';
    case 'zoom':
      return 'Zoom call';
    case 'slack':
      return 'Slack huddle';
    default:
      return 'Detected meeting';
  }
}

export function scoreMeetingActivitySignals(
  signals: MeetingActivitySignals,
  options: Partial<AmbientMeetingDetectionOptions> = {}
): MeetingActivityScore {
  const settings = { ...DEFAULT_OPTIONS, ...options };
  const textHaystack = [
    signals.activeAppName,
    signals.activeWindowTitle,
    ...signals.browserTabs.flatMap((tab) => [tab.title, tab.url]),
  ].filter(Boolean).join('\n');

  const meetingUrl = extractMeetingUrlFromSignals(signals);
  const textProvider = detectMeetingProviderFromText(textHaystack);
  const browserProvider = detectProviderFromBrowserSignals(signals.browserTabs);
  const processProvider = detectProviderFromProcesses([signals.activeAppName ?? '', ...signals.runningApps]);
  const hasDegradedPermissions = signals.degradedMode || (signals.missingPermissions?.length ?? 0) > 0;
  let provider = textProvider !== 'unknown' ? textProvider : browserProvider !== 'unknown' ? browserProvider : processProvider;
  const reasons: string[] = [];
  let confidence = 0;

  const activeWindowProvider = detectMeetingProviderFromText(signals.activeWindowTitle);
  if (activeWindowProvider !== 'unknown') {
    confidence += 45;
    reasons.push('Active meeting window');
  }

  const activeBrowserMeeting = signals.browserTabs.some((tab) => tab.isActive && (
    (tab.provider && tab.provider !== 'unknown') || detectMeetingProviderFromText(`${tab.title ?? ''} ${tab.url ?? ''}`) !== 'unknown'
  ));
  if (activeBrowserMeeting) {
    confidence += meetingUrl ? 60 : 40;
    reasons.push('Active browser meeting tab');
  }

  const activeAppProvider = detectProviderFromProcesses([signals.activeAppName ?? '']);
  if (activeAppProvider !== 'unknown') {
    confidence += 25;
    reasons.push('Meeting app in focus');
  }

  const runningProvider = detectProviderFromProcesses(signals.runningApps);
  if (runningProvider !== 'unknown') {
    confidence += 15;
    reasons.push('Meeting app running');
  }

  if (signals.micActivity?.isActive) {
    confidence += 25;
    reasons.push('Microphone activity');
  }

  const hasStrongSignal = activeWindowProvider !== 'unknown' || activeBrowserMeeting || Boolean(meetingUrl);
  if (hasDegradedPermissions) {
    reasons.push(`Limited by missing permission${signals.missingPermissions?.length === 1 ? '' : 's'}`);
    if (!hasStrongSignal) {
      confidence = Math.min(confidence, 39);
      if (provider === processProvider) provider = 'unknown';
    }
  }

  if (meetingUrl && !activeBrowserMeeting) {
    confidence += 35;
    reasons.push('Meeting link visible');
  }

  const finalConfidence = clampScore(confidence);
  return {
    isLikelyMeeting: provider !== 'unknown' && finalConfidence >= settings.minimumConfidence,
    confidence: finalConfidence,
    provider,
    title: titleFromSignals(signals, provider),
    meetingUrl,
    reasons,
  };
}

export function buildAmbientMeetingCandidate(
  signals: MeetingActivitySignals,
  now = new Date(),
  options: Partial<AmbientMeetingDetectionOptions> = {}
): MeetingJoinCandidate | null {
  const settings = { ...DEFAULT_OPTIONS, ...options };
  const score = scoreMeetingActivitySignals(signals, settings);
  if (!score.isLikelyMeeting) return null;

  const startAt = now.toISOString();
  const endAt = new Date(now.getTime() + settings.windowMinutes * 60 * 1000).toISOString();
  const idSeed = [
    score.provider,
    score.meetingUrl ?? score.title,
    signals.activeAppName ?? 'unknown-app',
  ].join(':');

  return {
    id: `ambient:${idSeed}`,
    eventId: idSeed,
    calendarId: 'ambient',
    calendarName: null,
    title: score.title,
    startAt,
    endAt,
    attendees: [],
    meetingUrl: score.meetingUrl,
    provider: score.provider,
    source: 'ambient',
    minutesUntilStart: 0,
    isActive: true,
    confidence: score.confidence,
    reasons: score.reasons,
  };
}
