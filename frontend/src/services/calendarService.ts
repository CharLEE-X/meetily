import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';

export interface CalendarProviderInfo {
  provider: string;
  label: string;
  available: boolean;
  supportsRead: boolean;
  supportsWrite: boolean;
  notes?: string | null;
}

export interface CalendarProviderAccount {
  id: string;
  provider: string;
  accountLabel: string;
  status: string;
  lastSyncAt?: string | null;
  lastError?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CalendarSource {
  id: string;
  providerAccountId: string;
  providerCalendarId: string;
  name: string;
  color?: string | null;
  selected: boolean;
  readOnly: boolean;
  lastSyncAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CalendarEvent {
  id: string;
  provider: string;
  providerEventId: string;
  calendarSourceId: string;
  title: string;
  startsAt: string;
  endsAt: string;
  timezone?: string | null;
  location?: string | null;
  meetingUrl?: string | null;
  meetingProvider?: string | null;
  attendeeCount?: number | null;
  attendeeNames?: string[] | null;
  organizerName?: string | null;
  descriptionExcerpt?: string | null;
  contentHash: string;
  syncStatus: string;
  updatedAt: string;
}

export interface CalendarSettingsState {
  providers: CalendarProviderInfo[];
  accounts: CalendarProviderAccount[];
  sources: CalendarSource[];
}

export interface CalendarSyncRequest {
  provider?: string;
  lookbackDays?: number;
  lookaheadDays?: number;
}

export interface CalendarSyncResult {
  provider: string;
  status: string;
  syncedEventCount: number;
  startedAt: string;
  completedAt: string;
  error?: string | null;
}

const requireDesktop = () => {
  if (!isTauriRuntime()) {
    throw new Error('Calendar integration is available in the desktop app.');
  }
};

export const calendarService = {
  async listProviders(): Promise<CalendarProviderInfo[]> {
    requireDesktop();
    return invoke<CalendarProviderInfo[]>('list_calendar_providers');
  },

  async getSettings(): Promise<CalendarSettingsState> {
    requireDesktop();
    return invoke<CalendarSettingsState>('get_calendar_settings');
  },

  async connectProvider(provider: string): Promise<CalendarProviderAccount> {
    requireDesktop();
    return invoke<CalendarProviderAccount>('connect_calendar_provider', { provider });
  },

  async disconnectProvider(provider: string): Promise<CalendarProviderAccount> {
    requireDesktop();
    return invoke<CalendarProviderAccount>('disconnect_calendar_provider', { provider });
  },

  async syncEvents(request?: CalendarSyncRequest): Promise<CalendarSyncResult> {
    requireDesktop();
    return invoke<CalendarSyncResult>('sync_calendar_events', { request });
  },

  async listUpcomingEvents(limit = 25): Promise<CalendarEvent[]> {
    requireDesktop();
    return invoke<CalendarEvent[]>('list_upcoming_calendar_events', { limit });
  },

  async linkMeetingEvent(
    meetingId: string,
    calendarEventId: string,
    linkSource = 'selected_before_recording',
    confidence?: number,
  ): Promise<void> {
    requireDesktop();
    return invoke('link_meeting_calendar_event', {
      meetingId,
      calendarEventId,
      linkSource,
      confidence,
    });
  },
};
