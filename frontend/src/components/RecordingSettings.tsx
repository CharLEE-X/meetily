import React, { useState, useEffect } from 'react';
import { Switch } from '@/components/ui/switch';
import { Camera, FolderOpen } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { DeviceSelection, SelectedDevices } from '@/components/DeviceSelection';
import Analytics from '@/lib/analytics';
import { toast } from 'sonner';
import {
  getScreenshotPreferences,
  setScreenshotPreferences,
  ScreenshotPreferences,
} from '@/services/screenshotService';

export interface RecordingPreferences {
  save_folder: string;
  auto_save: boolean;
  file_format: string;
  preferred_mic_device: string | null;
  preferred_system_device: string | null;
}

interface RecordingSettingsProps {
  onSave?: (preferences: RecordingPreferences) => void;
}

export function RecordingSettings({ onSave }: RecordingSettingsProps) {
  const [preferences, setPreferences] = useState<RecordingPreferences>({
    save_folder: '',
    auto_save: true,
    file_format: 'mp4',
    preferred_mic_device: null,
    preferred_system_device: null
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [showRecordingNotification, setShowRecordingNotification] = useState(true);
  const [screenshotPreferences, setLocalScreenshotPreferences] = useState<ScreenshotPreferences>({
    enabled: false,
    intervalSeconds: 60,
    captureTarget: 'callWindow',
    captureMode: 'interval',
    retentionDays: 30,
  });
  const [savingScreenshotPreferences, setSavingScreenshotPreferences] = useState(false);

  // Load recording preferences on component mount
  useEffect(() => {
    const loadPreferences = async () => {
      try {
        const prefs = await invoke<RecordingPreferences>('get_recording_preferences');
        setPreferences(prefs);
      } catch (error) {
        console.error('Failed to load recording preferences:', error);
        // If loading fails, get default folder path
        try {
          const defaultPath = await invoke<string>('get_default_recordings_folder_path');
          setPreferences(prev => ({ ...prev, save_folder: defaultPath }));
        } catch (defaultError) {
          console.error('Failed to get default folder path:', defaultError);
        }
      } finally {
        setLoading(false);
      }
    };

    loadPreferences();
  }, []);

  // Load screenshot capture preference separately. This is privacy-sensitive and defaults off.
  useEffect(() => {
    const loadScreenshotPreferences = async () => {
      try {
        const prefs = await getScreenshotPreferences();
        setLocalScreenshotPreferences(prefs);
      } catch (error) {
        console.error('Failed to load screenshot preferences:', error);
      }
    };
    loadScreenshotPreferences();
  }, []);

  // Load recording notification preference
  useEffect(() => {
    const loadNotificationPref = async () => {
      try {
        const { Store } = await import('@tauri-apps/plugin-store');
        const store = await Store.load('preferences.json');
        const show = await store.get<boolean>('show_recording_notification') ?? true;
        setShowRecordingNotification(show);
      } catch (error) {
        console.error('Failed to load notification preference:', error);
      }
    };
    loadNotificationPref();
  }, []);

  const handleAutoSaveToggle = async (enabled: boolean) => {
    const newPreferences = { ...preferences, auto_save: enabled };
    setPreferences(newPreferences);
    await savePreferences(newPreferences);

    // Track auto-save setting change
    await Analytics.track('auto_save_recording_toggled', {
      enabled: enabled.toString()
    });
  };

  const handleDeviceChange = async (devices: SelectedDevices) => {
    const newPreferences = {
      ...preferences,
      preferred_mic_device: devices.micDevice,
      preferred_system_device: devices.systemDevice
    };
    setPreferences(newPreferences);
    await savePreferences(newPreferences);

    // Track default device preference changes
    // Note: Individual device selection analytics are tracked in DeviceSelection component
    await Analytics.track('default_devices_changed', {
      has_preferred_microphone: (!!devices.micDevice).toString(),
      has_preferred_system_audio: (!!devices.systemDevice).toString()
    });
  };

  const handleOpenFolder = async () => {
    try {
      await invoke('open_recordings_folder');
    } catch (error) {
      console.error('Failed to open recordings folder:', error);
    }
  };

  const handleNotificationToggle = async (enabled: boolean) => {
    try {
      setShowRecordingNotification(enabled);
      const { Store } = await import('@tauri-apps/plugin-store');
      const store = await Store.load('preferences.json');
      await store.set('show_recording_notification', enabled);
      await store.save();
      toast.success('Preference saved');
      await Analytics.track('recording_notification_preference_changed', {
        enabled: enabled.toString()
      });
    } catch (error) {
      console.error('Failed to save notification preference:', error);
      toast.error('Failed to save preference');
    }
  };

  const saveScreenshotPreferences = async (prefs: ScreenshotPreferences) => {
    setSavingScreenshotPreferences(true);
    try {
      const saved = await setScreenshotPreferences(prefs);
      setLocalScreenshotPreferences(saved);
      if (typeof window !== 'undefined') {
        if (saved.enabled) {
          sessionStorage.setItem('screenshot_capture_mode', saved.captureMode);
        } else {
          sessionStorage.removeItem('screenshot_capture_mode');
        }
      }
      toast.success('Screenshot preferences saved');
    } catch (error) {
      console.error('Failed to save screenshot preferences:', error);
      toast.error('Failed to save screenshot preferences');
    } finally {
      setSavingScreenshotPreferences(false);
    }
  };

  const handleScreenshotToggle = async (enabled: boolean) => {
    const next = { ...screenshotPreferences, enabled };
    setLocalScreenshotPreferences(next);
    await saveScreenshotPreferences(next);
  };

  const handleScreenshotIntervalChange = async (value: string) => {
    const intervalSeconds = Math.max(30, Math.min(900, Number(value) || 60));
    const next = { ...screenshotPreferences, intervalSeconds };
    setLocalScreenshotPreferences(next);
    await saveScreenshotPreferences(next);
  };

  const handleScreenshotTargetChange = async (value: ScreenshotPreferences['captureTarget']) => {
    const next = { ...screenshotPreferences, captureTarget: value };
    setLocalScreenshotPreferences(next);
    await saveScreenshotPreferences(next);
  };

  const handleScreenshotModeChange = async (value: ScreenshotPreferences['captureMode']) => {
    const next = { ...screenshotPreferences, captureMode: value };
    setLocalScreenshotPreferences(next);
    await saveScreenshotPreferences(next);
  };

  const savePreferences = async (prefs: RecordingPreferences) => {
    setSaving(true);
    try {
      await invoke('set_recording_preferences', { preferences: prefs });
      onSave?.(prefs);

      // Show success toast with device details
      const micDevice = prefs.preferred_mic_device || 'Default';
      const systemDevice = prefs.preferred_system_device || 'Default';
      toast.success("Device preferences saved", {
        description: `Microphone: ${micDevice}, System Audio: ${systemDevice}`
      });
    } catch (error) {
      console.error('Failed to save recording preferences:', error);
      toast.error("Failed to save device preferences", {
        description: error instanceof Error ? error.message : String(error)
      });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="animate-pulse">
        <div className="h-4 bg-gray-200 rounded w-1/4 mb-4"></div>
        <div className="h-8 bg-gray-200 rounded mb-4"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">Recording Settings</h3>
        <p className="max-w-3xl text-sm leading-6 text-gray-600 mb-6">
          Configure what Meetily keeps from a recording session. These choices affect local audio retention, screenshot timeline context, participant recording reminders, and the default audio devices selected when a new meeting starts.
        </p>
      </div>

      {/* Auto Save Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Save Audio Recordings</div>
          <div className="max-w-2xl text-sm leading-6 text-gray-600">
            Automatically save the meeting audio file when recording stops. Keep this on when you want playback, retranscription, or auditability later; turn it off when you only need transcript and summary outputs.
          </div>
        </div>
        <Switch
          checked={preferences.auto_save}
          onCheckedChange={handleAutoSaveToggle}
          disabled={saving}
        />
      </div>

      {/* Folder Location - Only shown when auto_save is enabled */}
      {preferences.auto_save && (
        <div className="space-y-4">
          <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">Save Location</div>
            <p className="mb-3 text-sm leading-6 text-gray-600">
              Meetily writes saved audio files to this local folder. Moving or deleting files here can affect playback and later export workflows for existing meetings.
            </p>
            <div className="text-sm text-gray-600 mb-3 break-all">
              {preferences.save_folder || 'Default folder'}
            </div>
            <button
              onClick={handleOpenFolder}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-50 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div>

          <div className="p-4 border rounded-lg bg-blue-50">
            <div className="text-sm text-blue-800">
              <strong>File Format:</strong> {preferences.file_format.toUpperCase()} files
            </div>
            <div className="text-xs text-blue-600 mt-1">
              Recordings are saved with timestamp: recording_YYYYMMDD_HHMMSS.{preferences.file_format}
            </div>
            <div className="mt-2 text-xs leading-5 text-blue-700">
              The timestamped filename makes it easier to match a local audio file with the meeting timeline shown in Meetily.
            </div>
          </div>
        </div>
      )}

      {/* Info when auto_save is disabled */}
      {!preferences.auto_save && (
        <div className="p-4 border rounded-lg bg-yellow-50">
          <div className="text-sm leading-6 text-yellow-800">
            Audio files will not be retained after recording stops. Enable "Save Audio Recordings" if you need playback, retranscription, or a local source file for sharing later.
          </div>
        </div>
      )}

      {/* Recording Notification Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">Recording Start Notification</div>
          <div className="max-w-2xl text-sm leading-6 text-gray-600">
            Show a reminder when recording begins so you can tell participants that the meeting is being captured. This is a local reminder and does not notify other meeting attendees automatically.
          </div>
        </div>
        <Switch
          checked={showRecordingNotification}
          onCheckedChange={handleNotificationToggle}
        />
      </div>

      {/* Periodic Screenshot Capture */}
      <div className="p-4 border rounded-lg">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1">
            <div className="flex items-center gap-2 font-medium">
              <Camera className="w-4 h-4 text-gray-600" />
              Meeting Screenshots
            </div>
            <div className="max-w-2xl text-sm leading-6 text-gray-600 mt-1">
              Capture periodic snapshots during recordings for timeline context, speaker identification, and summary grounding. Meetily captures the detected call window by default and skips capture when the meeting window is unavailable.
            </div>
          </div>
          <Switch
            checked={screenshotPreferences.enabled}
            onCheckedChange={handleScreenshotToggle}
            disabled={savingScreenshotPreferences}
          />
        </div>

        {screenshotPreferences.enabled && (
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <label className="block">
              <span className="text-sm font-medium text-gray-700">Capture mode</span>
              <select
                value={screenshotPreferences.captureMode}
                onChange={(event) =>
                  handleScreenshotModeChange(
                    event.target.value as ScreenshotPreferences['captureMode']
                  )
                }
                disabled={savingScreenshotPreferences}
                className="mt-1 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              >
                <option value="interval">Interval only</option>
                <option value="speechEvent">Speech-event assisted</option>
                <option value="manualOnly">Manual only</option>
              </select>
              <span className="mt-1 block text-xs leading-5 text-gray-500">
                Speech-event assisted keeps the interval cadence and adds rate-limited snapshots
                around final transcript segments. Manual only keeps capture available from meeting
                controls without background snapshots.
              </span>
            </label>

            <label className="block">
              <span className="text-sm font-medium text-gray-700">Capture interval</span>
              <select
                value={screenshotPreferences.intervalSeconds}
                onChange={(event) => handleScreenshotIntervalChange(event.target.value)}
                disabled={savingScreenshotPreferences || screenshotPreferences.captureMode === 'manualOnly'}
                className="mt-1 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              >
                <option value={30}>Every 30 seconds</option>
                <option value={60}>Every minute</option>
                <option value={120}>Every 2 minutes</option>
                <option value={300}>Every 5 minutes</option>
              </select>
            </label>

            <label className="block">
              <span className="text-sm font-medium text-gray-700">Capture target</span>
              <select
                value={screenshotPreferences.captureTarget}
                onChange={(event) =>
                  handleScreenshotTargetChange(
                    event.target.value as ScreenshotPreferences['captureTarget']
                  )
                }
                disabled={savingScreenshotPreferences}
                className="mt-1 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              >
                <option value="callWindow">Detected call window</option>
                <option value="fullScreen">Full screen with warning</option>
              </select>
            </label>

            <div className="rounded-md bg-blue-50 px-3 py-2 text-sm text-blue-800 sm:col-span-2">
              Screenshots are stored locally with each meeting, filtered for timeline usefulness, and can be deleted from the meeting timeline. Full-screen capture may include other visible apps and should only be used when you explicitly need it.
            </div>
          </div>
        )}
      </div>

      {/* Device Preferences */}
      <div className="space-y-4">
        <div className="border-t pt-6">
          <h4 className="text-base font-medium text-gray-900 mb-4">Default Audio Devices</h4>
          <p className="max-w-3xl text-sm leading-6 text-gray-600 mb-4">
            Set the microphone and system audio devices Meetily should prefer for new recordings. If a saved device is unplugged or unavailable, Meetily falls back to the current system default so recording can still start.
          </p>

          <div className="border rounded-lg p-4 bg-gray-50">
            <DeviceSelection
              selectedDevices={{
                micDevice: preferences.preferred_mic_device,
                systemDevice: preferences.preferred_system_device
              }}
              onDeviceChange={handleDeviceChange}
              disabled={saving}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
