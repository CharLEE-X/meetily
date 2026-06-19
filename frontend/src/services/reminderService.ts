import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';

export interface ReminderProviderInfo {
  provider: string;
  label: string;
  available: boolean;
  supportsListDiscovery: boolean;
  supportsCreate: boolean;
  notes?: string | null;
}

export interface ReminderProviderAccount {
  id: string;
  provider: string;
  accountLabel: string;
  status: string;
  defaultListId?: string | null;
  lastSyncAt?: string | null;
  lastError?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ReminderList {
  id: string;
  providerAccountId: string;
  providerListId: string;
  name: string;
  color?: string | null;
  selected: boolean;
  isDefault: boolean;
  lastSeenAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ReminderSettingsState {
  providers: ReminderProviderInfo[];
  accounts: ReminderProviderAccount[];
  lists: ReminderList[];
}

export interface ReminderListSyncRequest {
  provider?: string;
}

export interface ReminderListSyncResult {
  provider: string;
  status: string;
  syncedListCount: number;
  startedAt: string;
  completedAt: string;
  error?: string | null;
}

export interface ReminderDefaultListRequest {
  provider?: string;
  listId: string;
}

export interface ReminderSourceEvidence {
  label: string;
  snippet: string;
}

export interface ReminderDraft {
  id: string;
  meetingId: string;
  summaryId?: string | null;
  title: string;
  notes?: string | null;
  dueAt?: string | null;
  priority?: number | null;
  listId?: string | null;
  category: string;
  confidence: number;
  sourceEvidence: ReminderSourceEvidence[];
  dedupeKey: string;
  status: string;
  createdAt: string;
  updatedAt: string;
}

export interface ReminderDraftRequest {
  meetingId: string;
  includeLowConfidence?: boolean;
}

export interface ReminderDraftGenerationResult {
  meetingId: string;
  drafts: ReminderDraft[];
  hiddenLowConfidenceCount: number;
  generatedAt: string;
}

export interface ReminderDraftUpdateRequest {
  draftId: string;
  title: string;
  notes?: string | null;
  dueAt?: string | null;
  priority?: number | null;
  listId?: string | null;
}

export interface CreateReminderRequest {
  meetingId: string;
  draftIds: string[];
}

export interface CreatedReminderLink {
  id: string;
  meetingId: string;
  draftId?: string | null;
  dedupeKey: string;
  provider: string;
  providerReminderId: string;
  listId?: string | null;
  title: string;
  status: string;
  createdAt: string;
  updatedAt: string;
  lastError?: string | null;
}

export interface ReminderCreationFailure {
  draftId: string;
  title: string;
  error: string;
}

export interface CreateReminderResult {
  meetingId: string;
  created: CreatedReminderLink[];
  skipped: CreatedReminderLink[];
  failed: ReminderCreationFailure[];
}

const requireDesktop = () => {
  if (!isTauriRuntime()) {
    throw new Error('Apple Reminders integration is available in the desktop app.');
  }
};

export const reminderService = {
  async listProviders(): Promise<ReminderProviderInfo[]> {
    requireDesktop();
    return invoke<ReminderProviderInfo[]>('list_reminder_providers');
  },

  async getSettings(): Promise<ReminderSettingsState> {
    requireDesktop();
    return invoke<ReminderSettingsState>('get_reminder_settings');
  },

  async connectProvider(provider: string): Promise<ReminderProviderAccount> {
    requireDesktop();
    return invoke<ReminderProviderAccount>('connect_reminder_provider', { provider });
  },

  async disconnectProvider(provider: string): Promise<ReminderProviderAccount> {
    requireDesktop();
    return invoke<ReminderProviderAccount>('disconnect_reminder_provider', { provider });
  },

  async syncLists(request?: ReminderListSyncRequest): Promise<ReminderListSyncResult> {
    requireDesktop();
    return invoke<ReminderListSyncResult>('sync_reminder_lists', { request });
  },

  async updateDefaultList(request: ReminderDefaultListRequest): Promise<ReminderProviderAccount> {
    requireDesktop();
    return invoke<ReminderProviderAccount>('update_default_reminder_list', { request });
  },

  async generateDrafts(request: ReminderDraftRequest): Promise<ReminderDraftGenerationResult> {
    requireDesktop();
    return invoke<ReminderDraftGenerationResult>('generate_reminder_drafts', { request });
  },

  async listDrafts(meetingId: string, includeLowConfidence = false): Promise<ReminderDraft[]> {
    requireDesktop();
    return invoke<ReminderDraft[]>('list_reminder_drafts', { meetingId, includeLowConfidence });
  },

  async updateDraft(request: ReminderDraftUpdateRequest): Promise<ReminderDraft> {
    requireDesktop();
    return invoke<ReminderDraft>('update_reminder_draft', { request });
  },

  async dismissDraft(draftId: string): Promise<ReminderDraft> {
    requireDesktop();
    return invoke<ReminderDraft>('dismiss_reminder_draft', { draftId });
  },

  async createSelected(request: CreateReminderRequest): Promise<CreateReminderResult> {
    requireDesktop();
    return invoke<CreateReminderResult>('create_selected_reminders', { request });
  },

  async listCreated(meetingId: string): Promise<CreatedReminderLink[]> {
    requireDesktop();
    return invoke<CreatedReminderLink[]>('list_created_reminders', { meetingId });
  },
};
