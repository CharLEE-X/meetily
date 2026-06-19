import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';

export interface AppleNotesProviderInfo {
  provider: string;
  label: string;
  available: boolean;
  supportsWrite: boolean;
  notes?: string | null;
}

export interface AppleNotesProviderAccount {
  id: string;
  provider: string;
  accountLabel: string;
  status: string;
  rootFolderName: string;
  groupingMode: string;
  autoExportEnabled: boolean;
  confirmedDestinationHash?: string | null;
  lastExportAt?: string | null;
  lastError?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AppleNotesExportRecord {
  id: string;
  meetingId: string;
  provider: string;
  accountId?: string | null;
  accountName?: string | null;
  folderId?: string | null;
  folderName?: string | null;
  providerNoteId?: string | null;
  noteTitle: string;
  contentHash: string;
  status: string;
  lastError?: string | null;
  exportedAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AppleNotesSettingsState {
  providers: AppleNotesProviderInfo[];
  accounts: AppleNotesProviderAccount[];
  recentExports: AppleNotesExportRecord[];
}

export interface AppleNotesExportPreview {
  meetingId: string;
  noteTitle: string;
  accountLabel: string;
  folderName: string;
  contentHash: string;
  destinationHash: string;
  summaryAvailable: boolean;
  transcriptReference?: string | null;
  sections: string[];
  requiresDestinationConfirmation: boolean;
  iCloudSyncDisclosure?: string | null;
}

export interface AppleNotesExportRequest {
  meetingId: string;
  confirmDestinationHash?: string | null;
}

export interface AppleNotesSettingsUpdateRequest {
  provider?: string;
  rootFolderName?: string;
  autoExportEnabled?: boolean;
}

const requireDesktop = () => {
  if (!isTauriRuntime()) {
    throw new Error('Apple Notes export is available in the desktop app.');
  }
};

export const appleNotesService = {
  async listProviders(): Promise<AppleNotesProviderInfo[]> {
    requireDesktop();
    return invoke<AppleNotesProviderInfo[]>('list_apple_notes_providers');
  },

  async getSettings(): Promise<AppleNotesSettingsState> {
    requireDesktop();
    return invoke<AppleNotesSettingsState>('get_apple_notes_settings');
  },

  async connectProvider(provider = 'apple_notes'): Promise<AppleNotesProviderAccount> {
    requireDesktop();
    return invoke<AppleNotesProviderAccount>('connect_apple_notes_provider', { provider });
  },

  async disconnectProvider(provider = 'apple_notes'): Promise<AppleNotesProviderAccount> {
    requireDesktop();
    return invoke<AppleNotesProviderAccount>('disconnect_apple_notes_provider', { provider });
  },

  async updateSettings(request: AppleNotesSettingsUpdateRequest): Promise<AppleNotesProviderAccount> {
    requireDesktop();
    return invoke<AppleNotesProviderAccount>('update_apple_notes_settings', { request });
  },

  async previewExport(meetingId: string): Promise<AppleNotesExportPreview> {
    requireDesktop();
    return invoke<AppleNotesExportPreview>('preview_apple_notes_export', { meetingId });
  },

  async exportMeeting(request: AppleNotesExportRequest): Promise<AppleNotesExportRecord> {
    requireDesktop();
    return invoke<AppleNotesExportRecord>('export_meeting_to_apple_notes', { request });
  },

  async getMeetingExport(meetingId: string): Promise<AppleNotesExportRecord | null> {
    requireDesktop();
    return invoke<AppleNotesExportRecord | null>('get_meeting_apple_notes_export', { meetingId });
  },

  async listRecentExports(limit = 10): Promise<AppleNotesExportRecord[]> {
    requireDesktop();
    return invoke<AppleNotesExportRecord[]>('list_recent_apple_notes_exports', { limit });
  },
};
