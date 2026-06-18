import { invoke } from '@tauri-apps/api/core';

export type ExportFormat = 'markdown' | 'pdf' | 'docx';

export interface ExportSections {
  metadata: boolean;
  summary: boolean;
  actionItems: boolean;
  transcript: boolean;
}

export interface ExportSettings {
  defaultFormat: ExportFormat;
  sections: ExportSections;
  autoExportEnabled: boolean;
  autoExportFormat: ExportFormat;
  destinationDir?: string | null;
  fileNameTemplate: string;
}

export interface ExportMeetingOptions {
  format: ExportFormat;
  sections: ExportSections;
  destinationDir?: string | null;
  fileName?: string | null;
  autoExport?: boolean;
}

export interface ExportResult {
  meetingId: string;
  format: ExportFormat;
  filePath: string;
  byteSize: number;
  createdAt: string;
  sections: ExportSections;
  autoExport: boolean;
}

export interface ExportHistoryEntry {
  meetingId: string;
  format: ExportFormat;
  filePath: string;
  byteSize: number;
  createdAt: string;
  autoExport: boolean;
}

export const defaultExportSettings: ExportSettings = {
  defaultFormat: 'markdown',
  sections: {
    metadata: true,
    summary: true,
    actionItems: true,
    transcript: true,
  },
  autoExportEnabled: false,
  autoExportFormat: 'markdown',
  destinationDir: null,
  fileNameTemplate: '{title}-{date}',
};

export async function getExportSettings(): Promise<ExportSettings> {
  return invoke<ExportSettings>('export_get_settings');
}

export async function updateExportSettings(settings: ExportSettings): Promise<ExportSettings> {
  return invoke<ExportSettings>('export_update_settings', { settings });
}

export async function exportMeeting(
  meetingId: string,
  options: ExportMeetingOptions,
): Promise<ExportResult> {
  return invoke<ExportResult>('export_meeting', { meetingId, options });
}

export async function getExportHistory(meetingId?: string): Promise<ExportHistoryEntry[]> {
  return invoke<ExportHistoryEntry[]>('export_get_history', {
    meetingId: meetingId ?? null,
  });
}
