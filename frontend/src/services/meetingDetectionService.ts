import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';
import {
  MeetingActivitySignals,
  MicActivitySignal,
  NativeMeetingActivitySignals,
  buildAmbientMeetingCandidate,
} from './meetingDetectionSignals';
import { CalendarEvent, CalendarSyncRequest, CalendarSyncResult, calendarService } from './calendarService';

export type MeetingDetectionMode = 'disabled' | 'prompt' | 'autoOpen';
export type MeetingProvider = 'google-meet' | 'zoom' | 'teams' | 'slack' | 'unknown';

export interface MeetingDetectionSettings {
  mode: MeetingDetectionMode;
  lookaheadMinutes: number;
  staleAfterMinutes: number;
  ambientDetectionEnabled: boolean;
  ambientMicSignalEnabled: boolean;
  ambientMinimumConfidence: number;
  quietHoursEnabled: boolean;
  quietHoursStart: string;
  quietHoursEnd: string;
  updatedAt: string | null;
}

export interface ApprovedCalendarEvent {
  id: string;
  calendarId: string;
  calendarName?: string | null;
  source: 'calendar' | 'local-app';
  provider?: string | null;
  title: string;
  description?: string | null;
  location?: string | null;
  startAt: string;
  endAt: string;
  attendees?: string[];
  meetingUrl?: string | null;
  updatedAt?: string | null;
}

export interface MeetingJoinCandidate {
  id: string;
  eventId: string;
  calendarId: string;
  calendarName: string | null;
  title: string;
  startAt: string;
  endAt: string;
  attendees: string[];
  meetingUrl: string | null;
  provider: MeetingProvider;
  source: ApprovedCalendarEvent['source'] | 'ambient';
  minutesUntilStart: number;
  isActive: boolean;
  confidence?: number;
  reasons?: string[];
}

const SETTINGS_KEY = 'meetily.meetingDetectionSettings';
const EVENTS_KEY = 'meetily.approvedMeetingEvents';
const SELECTED_RECORDING_EVENT_KEY = 'meetily.selectedCalendarRecordingEvent';
const DISMISSED_KEY = 'meetily.dismissedMeetingCandidates';
const AUTO_OPENED_KEY = 'meetily.autoOpenedMeetingCandidates';
const TRACKING_RETENTION_MS = 7 * 24 * 60 * 60 * 1000;

export const MEETING_DETECTION_SETTINGS_EVENT = 'meeting-detection-settings-changed';

export const DEFAULT_MEETING_DETECTION_SETTINGS: MeetingDetectionSettings = {
  mode: 'disabled',
  lookaheadMinutes: 15,
  staleAfterMinutes: 10,
  ambientDetectionEnabled: true,
  ambientMicSignalEnabled: true,
  ambientMinimumConfidence: 65,
  quietHoursEnabled: false,
  quietHoursStart: '18:00',
  quietHoursEnd: '08:00',
  updatedAt: null,
};

function hasLocalStorage(): boolean {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function readJson<T>(key: string, fallback: T): T {
  if (!hasLocalStorage()) return fallback;
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) return fallback;
    return JSON.parse(raw) as T;
  } catch (error) {
    console.warn(`Failed to read ${key}:`, error);
    return fallback;
  }
}

function writeJson<T>(key: string, value: T) {
  if (!hasLocalStorage()) return;
  window.localStorage.setItem(key, JSON.stringify(value));
}

function clampNumber(value: unknown, fallback: number, min: number, max: number): number {
  const parsed = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.min(max, Math.max(min, Math.round(parsed)));
}

function sanitizeTime(value: unknown, fallback: string): string {
  return typeof value === 'string' && /^([01]\d|2[0-3]):[0-5]\d$/.test(value) ? value : fallback;
}

export function getMeetingDetectionSettings(): MeetingDetectionSettings {
  const stored = readJson<Partial<MeetingDetectionSettings>>(SETTINGS_KEY, DEFAULT_MEETING_DETECTION_SETTINGS);
  const mode = stored.mode && ['disabled', 'prompt', 'autoOpen'].includes(stored.mode)
    ? stored.mode
    : DEFAULT_MEETING_DETECTION_SETTINGS.mode;

  return {
    mode,
    lookaheadMinutes: clampNumber(stored.lookaheadMinutes, DEFAULT_MEETING_DETECTION_SETTINGS.lookaheadMinutes, 1, 120),
    staleAfterMinutes: clampNumber(stored.staleAfterMinutes, DEFAULT_MEETING_DETECTION_SETTINGS.staleAfterMinutes, 1, 120),
    ambientDetectionEnabled: stored.ambientDetectionEnabled ?? DEFAULT_MEETING_DETECTION_SETTINGS.ambientDetectionEnabled,
    ambientMicSignalEnabled: stored.ambientMicSignalEnabled ?? DEFAULT_MEETING_DETECTION_SETTINGS.ambientMicSignalEnabled,
    ambientMinimumConfidence: clampNumber(stored.ambientMinimumConfidence, DEFAULT_MEETING_DETECTION_SETTINGS.ambientMinimumConfidence, 50, 95),
    quietHoursEnabled: Boolean(stored.quietHoursEnabled),
    quietHoursStart: sanitizeTime(stored.quietHoursStart, DEFAULT_MEETING_DETECTION_SETTINGS.quietHoursStart),
    quietHoursEnd: sanitizeTime(stored.quietHoursEnd, DEFAULT_MEETING_DETECTION_SETTINGS.quietHoursEnd),
    updatedAt: stored.updatedAt ?? null,
  };
}

export function saveMeetingDetectionSettings(settings: MeetingDetectionSettings): MeetingDetectionSettings {
  const sanitized = {
    ...settings,
    updatedAt: new Date().toISOString(),
  };
  writeJson(SETTINGS_KEY, getSanitizedSettings(sanitized));
  return getMeetingDetectionSettings();
}

function getSanitizedSettings(settings: MeetingDetectionSettings): MeetingDetectionSettings {
  return {
    ...getMeetingDetectionSettings(),
    ...settings,
    mode: ['disabled', 'prompt', 'autoOpen'].includes(settings.mode) ? settings.mode : 'disabled',
    lookaheadMinutes: clampNumber(settings.lookaheadMinutes, DEFAULT_MEETING_DETECTION_SETTINGS.lookaheadMinutes, 1, 120),
    staleAfterMinutes: clampNumber(settings.staleAfterMinutes, DEFAULT_MEETING_DETECTION_SETTINGS.staleAfterMinutes, 1, 120),
    ambientDetectionEnabled: Boolean(settings.ambientDetectionEnabled),
    ambientMicSignalEnabled: Boolean(settings.ambientMicSignalEnabled),
    ambientMinimumConfidence: clampNumber(settings.ambientMinimumConfidence, DEFAULT_MEETING_DETECTION_SETTINGS.ambientMinimumConfidence, 50, 95),
    quietHoursStart: sanitizeTime(settings.quietHoursStart, DEFAULT_MEETING_DETECTION_SETTINGS.quietHoursStart),
    quietHoursEnd: sanitizeTime(settings.quietHoursEnd, DEFAULT_MEETING_DETECTION_SETTINGS.quietHoursEnd),
  };
}

export function getApprovedCalendarEvents(): ApprovedCalendarEvent[] {
  return readJson<ApprovedCalendarEvent[]>(EVENTS_KEY, [])
    .filter((event) => Boolean(event.id && event.calendarId && event.title && event.startAt && event.endAt));
}

export function saveApprovedCalendarEvents(events: ApprovedCalendarEvent[]) {
  writeJson(EVENTS_KEY, events);
}

export function saveSyncedCalendarEvents(events: ApprovedCalendarEvent[]) {
  const nonCalendarEvents = getApprovedCalendarEvents().filter((event) => event.source !== 'calendar');
  writeJson(EVENTS_KEY, [...events, ...nonCalendarEvents].slice(0, 50));
}

export function addApprovedCalendarEvent(event: ApprovedCalendarEvent): ApprovedCalendarEvent[] {
  const events = getApprovedCalendarEvents();
  const nextEvents = [event, ...events.filter((item) => !(item.id === event.id && item.calendarId === event.calendarId))];
  saveApprovedCalendarEvents(nextEvents.slice(0, 50));
  return getApprovedCalendarEvents();
}

export function calendarEventToApprovedEvent(event: CalendarEvent): ApprovedCalendarEvent {
  return {
    id: event.id,
    calendarId: event.calendarSourceId,
    calendarName: event.provider,
    source: 'calendar',
    provider: event.meetingProvider,
    title: event.title,
    description: event.descriptionExcerpt,
    location: event.location,
    startAt: event.startsAt,
    endAt: event.endsAt,
    attendees: event.attendeeNames ?? [],
    meetingUrl: event.meetingUrl,
    updatedAt: event.updatedAt,
  };
}

export async function syncApprovedCalendarEventsFromProvider(
  request: CalendarSyncRequest = { provider: 'apple' },
  limit = 25,
): Promise<{ result: CalendarSyncResult; events: ApprovedCalendarEvent[] }> {
  const result = await calendarService.syncEvents(request);
  const upcomingEvents = await calendarService.listUpcomingEvents(limit);
  const approvedEvents = upcomingEvents.map(calendarEventToApprovedEvent);
  saveSyncedCalendarEvents(approvedEvents);
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(MEETING_DETECTION_SETTINGS_EVENT));
  }
  return { result, events: approvedEvents };
}

export function selectCalendarEventForRecording(event: ApprovedCalendarEvent) {
  if (typeof window === 'undefined') return;
  window.sessionStorage.setItem(SELECTED_RECORDING_EVENT_KEY, JSON.stringify(event));
}

export function getSelectedCalendarEventForRecording(): ApprovedCalendarEvent | null {
  if (typeof window === 'undefined') return null;
  try {
    const raw = window.sessionStorage.getItem(SELECTED_RECORDING_EVENT_KEY);
    if (!raw) return null;
    const event = JSON.parse(raw) as ApprovedCalendarEvent;
    if (!event.id || !event.title || !event.startAt || !event.endAt) return null;
    return event;
  } catch (error) {
    console.warn('Failed to read selected calendar event:', error);
    return null;
  }
}

export function clearSelectedCalendarEventForRecording() {
  if (typeof window === 'undefined') return;
  window.sessionStorage.removeItem(SELECTED_RECORDING_EVENT_KEY);
}

function timeToMinutes(value: string): number {
  const [hours, minutes] = value.split(':').map(Number);
  return hours * 60 + minutes;
}

export function isWithinQuietHours(date: Date, settings = getMeetingDetectionSettings()): boolean {
  if (!settings.quietHoursEnabled) return false;
  const nowMinutes = date.getHours() * 60 + date.getMinutes();
  const start = timeToMinutes(settings.quietHoursStart);
  const end = timeToMinutes(settings.quietHoursEnd);
  if (start === end) return true;
  if (start < end) return nowMinutes >= start && nowMinutes < end;
  return nowMinutes >= start || nowMinutes < end;
}

export function extractMeetingUrl(event: ApprovedCalendarEvent): string | null {
  const haystack = [event.meetingUrl, event.location, event.description].filter(Boolean).join('\n');
  const match = haystack.match(/https?:\/\/(?:[^\s<>"')]+)/i);
  if (!match) return null;
  const cleanUrl = match[0].replace(/[.,;]+$/, '');
  return detectMeetingProvider(cleanUrl) === 'unknown' ? null : cleanUrl;
}

export function detectMeetingProvider(url: string): MeetingProvider {
  const normalized = url.toLowerCase();
  if (normalized.includes('meet.google.com')) return 'google-meet';
  if (normalized.includes('zoom.us') || normalized.includes('zoom.com')) return 'zoom';
  if (normalized.includes('teams.microsoft.com') || normalized.includes('teams.live.com')) return 'teams';
  if (normalized.includes('slack.com/huddle') || normalized.includes('slack://huddle')) return 'slack';
  return 'unknown';
}

function candidateId(event: ApprovedCalendarEvent, meetingUrl: string): string {
  return `${event.calendarId}:${event.id}:${meetingUrl}`;
}

export function buildMeetingCandidateFromEvent(event: ApprovedCalendarEvent, now = new Date()): MeetingJoinCandidate {
  const meetingUrl = extractMeetingUrl(event);
  const startMs = Date.parse(event.startAt);
  const endMs = Date.parse(event.endAt);
  const safeStartMs = Number.isFinite(startMs) ? startMs : now.getTime();
  const safeEndMs = Number.isFinite(endMs) ? endMs : safeStartMs;

  return {
    id: meetingUrl ? candidateId(event, meetingUrl) : `${event.calendarId}:${event.id}:selected`,
    eventId: event.id,
    calendarId: event.calendarId,
    calendarName: event.calendarName ?? null,
    title: event.title,
    startAt: event.startAt,
    endAt: event.endAt,
    attendees: event.attendees ?? [],
    meetingUrl,
    provider: meetingUrl ? detectMeetingProvider(meetingUrl) : 'unknown',
    source: event.source,
    minutesUntilStart: Math.round((safeStartMs - now.getTime()) / 60000),
    isActive: safeStartMs <= now.getTime() && now.getTime() <= safeEndMs,
  };
}

function pruneTrackingMap(key: string): Record<string, string> {
  const now = Date.now();
  const current = readJson<Record<string, string>>(key, {});
  const pruned = Object.fromEntries(
    Object.entries(current).filter(([, timestamp]) => {
      const parsed = Date.parse(timestamp);
      return Number.isFinite(parsed) && now - parsed <= TRACKING_RETENTION_MS;
    })
  );
  if (Object.keys(pruned).length !== Object.keys(current).length) {
    writeJson(key, pruned);
  }
  return pruned;
}

function readDismissedCandidates(): Record<string, string> {
  return pruneTrackingMap(DISMISSED_KEY);
}

function readAutoOpenedCandidates(): Record<string, string> {
  return pruneTrackingMap(AUTO_OPENED_KEY);
}

function isDismissed(candidateIdValue: string, event: ApprovedCalendarEvent): boolean {
  const dismissedAt = readDismissedCandidates()[candidateIdValue];
  if (!dismissedAt) return false;
  const eventUpdatedAt = event.updatedAt ? Date.parse(event.updatedAt) : null;
  return !eventUpdatedAt || Date.parse(dismissedAt) >= eventUpdatedAt;
}

export function dismissMeetingCandidate(candidate: Pick<MeetingJoinCandidate, 'id'>) {
  writeJson(DISMISSED_KEY, {
    ...readDismissedCandidates(),
    [candidate.id]: new Date().toISOString(),
  });
}

export function wasAutoOpened(candidate: Pick<MeetingJoinCandidate, 'id'>): boolean {
  return Boolean(readAutoOpenedCandidates()[candidate.id]);
}

export function markMeetingCandidateAutoOpened(candidate: Pick<MeetingJoinCandidate, 'id'>) {
  writeJson(AUTO_OPENED_KEY, {
    ...readAutoOpenedCandidates(),
    [candidate.id]: new Date().toISOString(),
  });
}

export function isMeetingCandidateDismissed(candidate: Pick<MeetingJoinCandidate, 'id'>): boolean {
  return Boolean(readDismissedCandidates()[candidate.id]);
}

export function getUpcomingMeetingCandidates(
  events = getApprovedCalendarEvents(),
  settings = getMeetingDetectionSettings(),
  now = new Date()
): MeetingJoinCandidate[] {
  if (settings.mode === 'disabled' || isWithinQuietHours(now, settings)) return [];

  const seen = new Set<string>();
  const nowMs = now.getTime();
  const lookaheadMs = settings.lookaheadMinutes * 60 * 1000;
  const staleMs = settings.staleAfterMinutes * 60 * 1000;

  return events
    .map((event): MeetingJoinCandidate | null => {
      const meetingUrl = extractMeetingUrl(event);
      if (!meetingUrl) return null;
      const startMs = Date.parse(event.startAt);
      const endMs = Date.parse(event.endAt);
      if (!Number.isFinite(startMs) || !Number.isFinite(endMs)) return null;
      if (startMs - nowMs > lookaheadMs) return null;
      if (nowMs - endMs > staleMs) return null;
      if (endMs <= startMs) return null;

      const id = candidateId(event, meetingUrl);
      if (seen.has(id) || isDismissed(id, event)) return null;
      seen.add(id);

      return {
        id,
        eventId: event.id,
        calendarId: event.calendarId,
        calendarName: event.calendarName ?? null,
        title: event.title,
        startAt: event.startAt,
        endAt: event.endAt,
        attendees: event.attendees ?? [],
        meetingUrl,
        provider: detectMeetingProvider(meetingUrl),
        source: event.source,
        minutesUntilStart: Math.round((startMs - nowMs) / 60000),
        isActive: startMs <= nowMs && nowMs <= endMs,
      } satisfies MeetingJoinCandidate;
    })
    .filter((candidate): candidate is MeetingJoinCandidate => Boolean(candidate))
    .sort((a, b) => Date.parse(a.startAt) - Date.parse(b.startAt));
}

export async function openMeetingCandidate(candidate: MeetingJoinCandidate) {
  if (!candidate.meetingUrl) {
    throw new Error('This detected meeting does not expose a join link.');
  }
  if (!isTauriRuntime()) {
    window.open(candidate.meetingUrl, '_blank', 'noopener,noreferrer');
    return;
  }
  await invoke('open_external_url', { url: candidate.meetingUrl });
}

export async function getNativeMeetingActivitySignals(): Promise<NativeMeetingActivitySignals | null> {
  if (!isTauriRuntime()) return null;
  try {
    return await invoke<NativeMeetingActivitySignals>('get_meeting_activity_signals');
  } catch (error) {
    console.warn('Failed to collect meeting activity signals:', error);
    return null;
  }
}

export async function getAmbientMeetingCandidate(
  settings = getMeetingDetectionSettings(),
  micActivity?: MicActivitySignal | null,
  now = new Date()
): Promise<MeetingJoinCandidate | null> {
  if (settings.mode === 'disabled' || !settings.ambientDetectionEnabled || isWithinQuietHours(now, settings)) return null;

  const nativeSignals = await getNativeMeetingActivitySignals();
  if (!nativeSignals) return null;

  const candidate = buildAmbientMeetingCandidate({
    ...nativeSignals,
    micActivity: settings.ambientMicSignalEnabled ? micActivity : null,
  } satisfies MeetingActivitySignals, now, {
    minimumConfidence: settings.ambientMinimumConfidence,
    windowMinutes: Math.max(30, Math.min(180, settings.staleAfterMinutes + 90)),
  });

  if (!candidate || isMeetingCandidateDismissed(candidate)) return null;
  return candidate;
}
