import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';

export interface AppSettings {
  launchAtLogin: boolean;
  startMinimized: boolean;
  startupSupported: boolean;
  loginItemInstalled: boolean;
  loginItemPath?: string | null;
}

export interface AppSettingsUpdate {
  launchAtLogin: boolean;
  startMinimized: boolean;
}

const webPreviewSettings: AppSettings = {
  launchAtLogin: false,
  startMinimized: false,
  startupSupported: false,
  loginItemInstalled: false,
  loginItemPath: null,
};

export const appSettingsService = {
  async getSettings(): Promise<AppSettings> {
    if (!isTauriRuntime()) return webPreviewSettings;
    return invoke<AppSettings>('get_app_settings');
  },

  async updateSettings(settings: AppSettingsUpdate): Promise<AppSettings> {
    if (!isTauriRuntime()) {
      throw new Error('Startup settings are available in the desktop app.');
    }

    return invoke<AppSettings>('update_app_settings', { settings });
  },
};
