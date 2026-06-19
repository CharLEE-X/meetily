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
};
