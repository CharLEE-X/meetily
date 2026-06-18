import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';

export type MeetingDetectionMode = 'disabled' | 'prompt' | 'autoOpen';
export type MeetingProvider = 'google-meet' | 'zoom' | 'teams' | 'unknown';

export interface MeetingDetectionSettings {
  mode: MeetingDetectionMode;
  lookaheadMinutes: number;
  staleAfterMinutes: number;
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
  meetingUrl: string;
  provider: MeetingProvider;
  source: ApprovedCalendarEvent['source'];
  minutesUntilStart: number;
  isActive: boolean;
}

const SETTINGS_KEY = 'meetily.meetingDetectionSettings';
const EVENTS_KEY = 'meetily.approvedMeetingEvents';
const DISMISSED_KEY = 'meetily.dismissedMeetingCandidates';
const AUTO_OPENED_KEY = 'meetily.autoOpenedMeetingCandidates';
const TRACKING_RETENTION_MS = 7 * 24 * 60 * 60 * 1000;

export const MEETING_DETECTION_SETTINGS_EVENT = 'meeting-detection-settings-changed';

export const DEFAULT_MEETING_DETECTION_SETTINGS: MeetingDetectionSettings = {
  mode: 'disabled',
  lookaheadMinutes: 15,
  staleAfterMinutes: 10,
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

export function addApprovedCalendarEvent(event: ApprovedCalendarEvent): ApprovedCalendarEvent[] {
  const events = getApprovedCalendarEvents();
  const nextEvents = [event, ...events.filter((item) => !(item.id === event.id && item.calendarId === event.calendarId))];
  saveApprovedCalendarEvents(nextEvents.slice(0, 50));
  return getApprovedCalendarEvents();
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
  return 'unknown';
}

function candidateId(event: ApprovedCalendarEvent, meetingUrl: string): string {
  return `${event.calendarId}:${event.id}:${meetingUrl}`;
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
    .map((event) => {
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
  if (!isTauriRuntime()) {
    window.open(candidate.meetingUrl, '_blank', 'noopener,noreferrer');
    return;
  }
  await invoke('open_external_url', { url: candidate.meetingUrl });
}
