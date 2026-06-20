"use client"

import { useEffect, useState, useRef } from "react"
import { Switch } from "./ui/switch"
import { CalendarClock, FolderOpen, Monitor, Moon, Sun, type LucideIcon } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import Analytics from "@/lib/analytics"
import AnalyticsConsentSwitch from "./AnalyticsConsentSwitch"
import { ThemePreference, useConfig, NotificationSettings } from "@/contexts/ConfigContext"
import {
  ApprovedCalendarEvent,
  MEETING_DETECTION_SETTINGS_EVENT,
  MeetingDetectionMode,
  MeetingDetectionSettings,
  addApprovedCalendarEvent,
  getApprovedCalendarEvents,
  getMeetingDetectionSettings,
  saveMeetingDetectionSettings,
} from "@/services/meetingDetectionService"

function localDateTimeValue(offsetMinutes = 0): string {
  const date = new Date(Date.now() + offsetMinutes * 60 * 1000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function PreferenceSettings() {
  const {
    notificationSettings,
    storageLocations,
    isLoadingPreferences,
    loadPreferences,
    updateNotificationSettings,
    themePreference,
    setThemePreference
  } = useConfig();

  const [notificationsEnabled, setNotificationsEnabled] = useState<boolean | null>(null);
  const [isInitialLoad, setIsInitialLoad] = useState(true);
  const [previousNotificationsEnabled, setPreviousNotificationsEnabled] = useState<boolean | null>(null);
  const [meetingDetectionSettings, setMeetingDetectionSettings] = useState<MeetingDetectionSettings>(() => getMeetingDetectionSettings());
  const [approvedEventCount, setApprovedEventCount] = useState(() => getApprovedCalendarEvents().length);
  const [localEventTitle, setLocalEventTitle] = useState("Upcoming meeting");
  const [localEventUrl, setLocalEventUrl] = useState("");
  const [localEventStart, setLocalEventStart] = useState(() => localDateTimeValue(5));
  const [localEventEnd, setLocalEventEnd] = useState(() => localDateTimeValue(35));
  const hasTrackedViewRef = useRef(false);

  // Lazy load preferences on mount (only loads if not already cached)
  useEffect(() => {
    loadPreferences();
    // Reset tracking ref on mount (every tab visit)
    hasTrackedViewRef.current = false;
  }, [loadPreferences]);

  // Track preferences viewed analytics on every tab visit (once per mount)
  useEffect(() => {
    if (hasTrackedViewRef.current) return;

    const trackPreferencesViewed = async () => {
      // Wait for notification settings to be available (either from cache or after loading)
      if (notificationSettings) {
        await Analytics.track('preferences_viewed', {
          notifications_enabled: notificationSettings.notification_preferences.show_recording_started ? 'true' : 'false'
        });
        hasTrackedViewRef.current = true;
      } else if (!isLoadingPreferences) {
        // If not loading and no settings available, track with default value
        await Analytics.track('preferences_viewed', {
          notifications_enabled: 'false'
        });
        hasTrackedViewRef.current = true;
      }
    };

    trackPreferencesViewed();
  }, [notificationSettings, isLoadingPreferences]);

  // Update notificationsEnabled when notificationSettings are loaded from global state
  useEffect(() => {
    if (notificationSettings) {
      // Notification enabled means both started and stopped notifications are enabled
      const enabled =
        notificationSettings.notification_preferences.show_recording_started &&
        notificationSettings.notification_preferences.show_recording_stopped;
      setNotificationsEnabled(enabled);
      if (isInitialLoad) {
        setPreviousNotificationsEnabled(enabled);
        setIsInitialLoad(false);
      }
    } else if (!isLoadingPreferences) {
      // If not loading and no settings, use default
      setNotificationsEnabled(true);
      if (isInitialLoad) {
        setPreviousNotificationsEnabled(true);
        setIsInitialLoad(false);
      }
    }
  }, [notificationSettings, isLoadingPreferences, isInitialLoad])

  useEffect(() => {
    // Skip update on initial load or if value hasn't actually changed
    if (isInitialLoad || notificationsEnabled === null || notificationsEnabled === previousNotificationsEnabled) return;
    if (!notificationSettings) return;

    const handleUpdateNotificationSettings = async () => {
      console.log("Updating notification settings to:", notificationsEnabled);

      try {
        // Update the notification preferences
        const updatedSettings: NotificationSettings = {
          ...notificationSettings,
          notification_preferences: {
            ...notificationSettings.notification_preferences,
            show_recording_started: notificationsEnabled,
            show_recording_stopped: notificationsEnabled,
          }
        };

        console.log("Calling updateNotificationSettings with:", updatedSettings);
        await updateNotificationSettings(updatedSettings);
        setPreviousNotificationsEnabled(notificationsEnabled);
        console.log("Successfully updated notification settings to:", notificationsEnabled);

        // Track notification preference change - only fires when user manually toggles
        await Analytics.track('notification_settings_changed', {
          notifications_enabled: notificationsEnabled.toString()
        });
      } catch (error) {
        console.error('Failed to update notification settings:', error);
      }
    };

    handleUpdateNotificationSettings();
  }, [notificationsEnabled, notificationSettings, isInitialLoad, previousNotificationsEnabled, updateNotificationSettings])

  const handleOpenFolder = async (folderType: 'database' | 'models' | 'recordings') => {
    try {
      switch (folderType) {
        case 'database':
          await invoke('open_database_folder');
          break;
        case 'models':
          await invoke('open_models_folder');
          break;
        case 'recordings':
          await invoke('open_recordings_folder');
          break;
      }

      // Track storage folder access
      await Analytics.track('storage_folder_opened', {
        folder_type: folderType
      });
    } catch (error) {
      console.error(`Failed to open ${folderType} folder:`, error);
    }
  };

  const updateMeetingDetectionSettings = (nextSettings: MeetingDetectionSettings) => {
    const saved = saveMeetingDetectionSettings(nextSettings);
    setMeetingDetectionSettings(saved);
    setApprovedEventCount(getApprovedCalendarEvents().length);
    window.dispatchEvent(new Event(MEETING_DETECTION_SETTINGS_EVENT));
  };

  const handleMeetingDetectionModeChange = async (mode: MeetingDetectionMode) => {
    updateMeetingDetectionSettings({ ...meetingDetectionSettings, mode });
    await Analytics.track('meeting_detection_mode_changed', { mode });
  };

  const canAddLocalEvent = Boolean(
    localEventTitle.trim() &&
    localEventUrl.trim() &&
    Number.isFinite(Date.parse(localEventStart)) &&
    Number.isFinite(Date.parse(localEventEnd))
  );

  const handleAddLocalEvent = async () => {
    if (!canAddLocalEvent) return;

    const event: ApprovedCalendarEvent = {
      id: `local-${Date.now()}`,
      calendarId: "local-approved-events",
      calendarName: "Local approved events",
      source: "local-app",
      provider: "manual",
      title: localEventTitle.trim(),
      meetingUrl: localEventUrl.trim(),
      startAt: new Date(localEventStart).toISOString(),
      endAt: new Date(localEventEnd).toISOString(),
      attendees: [],
      updatedAt: new Date().toISOString(),
    };

    addApprovedCalendarEvent(event);
    setApprovedEventCount(getApprovedCalendarEvents().length);
    window.dispatchEvent(new Event(MEETING_DETECTION_SETTINGS_EVENT));
    await Analytics.track('meeting_detection_local_event_added', { provider: 'manual' });
  };

  // Show loading only if we're actually loading and don't have cached data
  if (isLoadingPreferences && !notificationSettings && !storageLocations) {
    return <div className="max-w-2xl mx-auto p-6">Loading Preferences...</div>
  }

  // Show loading if notificationsEnabled hasn't been determined yet
  if (notificationsEnabled === null && !isLoadingPreferences) {
    return <div className="max-w-2xl mx-auto p-6">Loading Preferences...</div>
  }

  // Ensure we have a boolean value for the Switch component
  const notificationsEnabledValue = notificationsEnabled ?? false;
  const themeOptions: Array<{
    value: ThemePreference;
    label: string;
    description: string;
    icon: LucideIcon;
  }> = [
    {
      value: 'system',
      label: 'System',
      description: 'Follow the current macOS appearance and update automatically when it changes.',
      icon: Monitor,
    },
    {
      value: 'light',
      label: 'Light',
      description: 'Use the bright interface regardless of the system appearance.',
      icon: Sun,
    },
    {
      value: 'dark',
      label: 'Dark',
      description: 'Use the darker interface for low-light work and reduced glare.',
      icon: Moon,
    },
  ];

  return (
    <div className="space-y-6">
      {/* Theme Section */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">Appearance</h3>
          <p className="mt-2 max-w-3xl text-sm leading-6 text-gray-600">
            Choose how Meetily should look across the desktop app. System follows your macOS Light or Dark appearance, while Light and Dark keep the app pinned to that mode.
          </p>
        </div>
        <div className="mt-5 grid gap-3 md:grid-cols-3">
          {themeOptions.map((option) => {
            const Icon = option.icon;
            const selected = themePreference === option.value;
            return (
              <button
                key={option.value}
                type="button"
                onClick={() => setThemePreference(option.value)}
                className={`rounded-lg border p-4 text-left transition-colors ${
                  selected
                    ? 'border-blue-300 bg-blue-50 text-blue-950 ring-1 ring-blue-200'
                    : 'border-gray-200 bg-gray-50 text-gray-900 hover:border-gray-300 hover:bg-white'
                }`}
                aria-pressed={selected}
              >
                <div className="flex items-center gap-2">
                  <Icon className={`h-4 w-4 ${selected ? 'text-blue-700' : 'text-gray-500'}`} />
                  <span className="text-sm font-semibold">{option.label}</span>
                </div>
                <p className={`mt-2 text-xs leading-5 ${selected ? 'text-blue-800' : 'text-gray-600'}`}>
                  {option.description}
                </p>
              </button>
            );
          })}
        </div>
      </div>

      {/* Notifications Section */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Notifications</h3>
            <p className="max-w-2xl text-sm leading-6 text-gray-600">
              Controls the local macOS notifications shown when Meetily starts and stops recording. These reminders help you confirm recording state and prompt consent conversations, but they do not start or stop recording by themselves.
            </p>
          </div>
          <Switch checked={notificationsEnabledValue} onCheckedChange={setNotificationsEnabled} />
        </div>
      </div>

      {/* Meeting Detection Section */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="flex items-center gap-2">
              <CalendarClock className="h-5 w-5 text-blue-600" />
              <h3 className="text-lg font-semibold text-gray-900">Meeting detection</h3>
            </div>
            <p className="mt-2 max-w-2xl text-sm text-gray-600">
              Detect upcoming meetings from approved calendar metadata, meeting apps, active call windows, browser meeting tabs, and optional mic activity. Meetily uses this to suggest titles, prompts, and join actions; it never joins or records silently.
            </p>
          </div>
          <div className="rounded-full bg-gray-100 px-3 py-1 text-xs text-gray-700">
            {approvedEventCount} approved event{approvedEventCount === 1 ? "" : "s"}
          </div>
        </div>

        <div className="mt-5 grid gap-4 md:grid-cols-3">
          <div>
            <label className="text-sm font-medium text-gray-900" htmlFor="meeting-detection-mode">Mode</label>
            <p className="mt-1 text-xs leading-5 text-gray-500">Disabled hides prompts, Prompt only asks before action, and Auto-open can open the approved meeting link without starting a recording.</p>
            <select
              id="meeting-detection-mode"
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              value={meetingDetectionSettings.mode}
              onChange={(event) => handleMeetingDetectionModeChange(event.target.value as MeetingDetectionMode)}
            >
              <option value="disabled">Disabled</option>
              <option value="prompt">Prompt only</option>
              <option value="autoOpen">Auto-open link</option>
            </select>
          </div>
          <div>
            <label className="text-sm font-medium text-gray-900" htmlFor="meeting-detection-lookahead">Lookahead minutes</label>
            <p className="mt-1 text-xs leading-5 text-gray-500">How early Meetily should start considering an approved meeting relevant for prompts and metadata.</p>
            <input
              id="meeting-detection-lookahead"
              type="number"
              min={1}
              max={120}
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              value={meetingDetectionSettings.lookaheadMinutes}
              onChange={(event) => updateMeetingDetectionSettings({
                ...meetingDetectionSettings,
                lookaheadMinutes: Number(event.target.value),
              })}
            />
          </div>
          <div>
            <label className="text-sm font-medium text-gray-900" htmlFor="meeting-detection-stale">Hide after minutes</label>
            <p className="mt-1 text-xs leading-5 text-gray-500">How long after a meeting starts the prompt should stay visible before being dismissed as stale.</p>
            <input
              id="meeting-detection-stale"
              type="number"
              min={1}
              max={120}
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              value={meetingDetectionSettings.staleAfterMinutes}
              onChange={(event) => updateMeetingDetectionSettings({
                ...meetingDetectionSettings,
                staleAfterMinutes: Number(event.target.value),
              })}
            />
          </div>
        </div>

        <div className="mt-5 rounded-lg border border-gray-200 bg-gray-50 p-4">
          <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
            <div>
              <div className="text-sm font-medium text-gray-900">Ambient meeting signals</div>
              <p className="mt-1 text-sm text-gray-600">
                Look for Teams, Zoom, Google Meet, active call windows, and browser tabs when calendar metadata is missing. These checks run locally and are used only to decide whether a prompt should appear.
              </p>
            </div>
            <Switch
              checked={meetingDetectionSettings.ambientDetectionEnabled}
              onCheckedChange={(checked) => updateMeetingDetectionSettings({
                ...meetingDetectionSettings,
                ambientDetectionEnabled: checked,
              })}
            />
          </div>
          {meetingDetectionSettings.ambientDetectionEnabled && (
            <div className="mt-4 grid gap-4 md:grid-cols-2">
              <div className="rounded-md border border-gray-200 bg-white p-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="text-sm font-medium text-gray-900">Use mic activity as a signal</div>
                    <p className="mt-1 text-xs text-gray-600">
                      Measures local input levels only. It cannot identify which app is using the microphone.
                    </p>
                  </div>
                  <Switch
                    checked={meetingDetectionSettings.ambientMicSignalEnabled}
                    onCheckedChange={(checked) => updateMeetingDetectionSettings({
                      ...meetingDetectionSettings,
                      ambientMicSignalEnabled: checked,
                    })}
                  />
                </div>
              </div>
              <div>
                <label className="text-sm font-medium text-gray-900" htmlFor="ambient-confidence">Prompt confidence</label>
                <input
                  id="ambient-confidence"
                  type="number"
                  min={50}
                  max={95}
                  className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
                  value={meetingDetectionSettings.ambientMinimumConfidence}
                  onChange={(event) => updateMeetingDetectionSettings({
                    ...meetingDetectionSettings,
                    ambientMinimumConfidence: Number(event.target.value),
                  })}
                />
                <p className="mt-1 text-xs text-gray-500">Higher values reduce false prompts but may miss meetings.</p>
              </div>
            </div>
          )}
        </div>

        <div className="mt-5 rounded-lg border border-gray-200 bg-gray-50 p-4">
          <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
            <div>
              <div className="text-sm font-medium text-gray-900">Quiet hours</div>
              <p className="mt-1 text-sm text-gray-600">Hide meeting prompts during focus or non-working hours. Quiet hours can cross midnight, so a 22:00 to 07:00 window covers overnight focus time.</p>
            </div>
            <Switch
              checked={meetingDetectionSettings.quietHoursEnabled}
              onCheckedChange={(checked) => updateMeetingDetectionSettings({
                ...meetingDetectionSettings,
                quietHoursEnabled: checked,
              })}
            />
          </div>
          {meetingDetectionSettings.quietHoursEnabled && (
            <div className="mt-4 grid gap-4 sm:grid-cols-2">
              <div>
                <label className="text-sm font-medium text-gray-900" htmlFor="quiet-hours-start">Start</label>
                <input
                  id="quiet-hours-start"
                  type="time"
                  className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
                  value={meetingDetectionSettings.quietHoursStart}
                  onChange={(event) => updateMeetingDetectionSettings({
                    ...meetingDetectionSettings,
                    quietHoursStart: event.target.value,
                  })}
                />
              </div>
              <div>
                <label className="text-sm font-medium text-gray-900" htmlFor="quiet-hours-end">End</label>
                <input
                  id="quiet-hours-end"
                  type="time"
                  className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
                  value={meetingDetectionSettings.quietHoursEnd}
                  onChange={(event) => updateMeetingDetectionSettings({
                    ...meetingDetectionSettings,
                    quietHoursEnd: event.target.value,
                  })}
                />
              </div>
            </div>
          )}
        </div>

        <div className="mt-4 rounded-md border border-amber-200 bg-amber-50 p-3 text-xs text-amber-900">
          Auto-open launches the meeting URL only after opt-in. It does not click provider join buttons, start recording, or enable microphone/camera.
        </div>

        <div className="mt-5 rounded-lg border border-gray-200 bg-gray-50 p-4">
          <div className="flex flex-col gap-1">
            <div className="text-sm font-medium text-gray-900">Approved local event</div>
            <p className="text-sm text-gray-600">
              Add a local event while calendar sync is not connected. This stores only the title, time, and meeting link needed for a prompt. Supported links: Google Meet, Zoom, and Microsoft Teams.
            </p>
          </div>
          <div className="mt-4 grid gap-3 md:grid-cols-2">
            <div>
              <label className="text-sm font-medium text-gray-900" htmlFor="local-event-title">Title</label>
              <input
                id="local-event-title"
                className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
                value={localEventTitle}
                onChange={(event) => setLocalEventTitle(event.target.value)}
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-900" htmlFor="local-event-url">Meeting URL</label>
              <input
                id="local-event-url"
                className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
                placeholder="https://meet.google.com/..."
                value={localEventUrl}
                onChange={(event) => setLocalEventUrl(event.target.value)}
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-900" htmlFor="local-event-start">Start</label>
              <input
                id="local-event-start"
                type="datetime-local"
                className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
                value={localEventStart}
                onChange={(event) => setLocalEventStart(event.target.value)}
              />
            </div>
            <div>
              <label className="text-sm font-medium text-gray-900" htmlFor="local-event-end">End</label>
              <input
                id="local-event-end"
                type="datetime-local"
                className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
                value={localEventEnd}
                onChange={(event) => setLocalEventEnd(event.target.value)}
              />
            </div>
          </div>
          <button
            type="button"
            className="mt-4 rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            disabled={!canAddLocalEvent}
            onClick={handleAddLocalEvent}
          >
            Add approved event
          </button>
        </div>
      </div>

      {/* Data Storage Locations Section */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Data Storage Locations</h3>
        <p className="max-w-3xl text-sm leading-6 text-gray-600 mb-6">
          View and open the local folders Meetily uses for recordings and app-managed data. Audio files, screenshots, transcripts, and summaries are stored on this Mac unless you explicitly export them or use a cloud AI provider.
        </p>

        <div className="space-y-4">
          {/* Database Location */}
          {/* <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">Database</div>
            <div className="text-sm text-gray-600 mb-3 break-all font-mono text-xs">
              {storageLocations?.database || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('database')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Models Location */}
          {/* <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">Whisper Models</div>
            <div className="text-sm text-gray-600 mb-3 break-all font-mono text-xs">
              {storageLocations?.models || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('models')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div> */}

          {/* Recordings Location */}
          <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">Meeting Recordings</div>
            <p className="mb-3 text-sm leading-6 text-gray-600">
              This folder contains saved meeting audio and related recording artifacts. Deleting files here can remove playback or export sources for existing meetings.
            </p>
            <div className="text-sm text-gray-600 mb-3 break-all font-mono text-xs">
              {storageLocations?.recordings || 'Loading...'}
            </div>
            <button
              onClick={() => handleOpenFolder('recordings')}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-100 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              Open Folder
            </button>
          </div>
        </div>

        <div className="mt-4 p-3 bg-blue-50 rounded-md">
          <p className="text-xs text-blue-800">
            <strong>Note:</strong> Database and models are stored together in your application data directory for unified management.
          </p>
        </div>
      </div>

      {/* Analytics Section */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="mb-4">
          <h3 className="text-lg font-semibold text-gray-900">Analytics privacy</h3>
          <p className="mt-2 max-w-3xl text-sm leading-6 text-gray-600">
            Decide whether Meetily can send product usage events that help prioritize fixes and features. Meeting audio, transcripts, summaries, screenshots, and note contents are not analytics payloads.
          </p>
        </div>
        <AnalyticsConsentSwitch />
      </div>
    </div>
  )
}
