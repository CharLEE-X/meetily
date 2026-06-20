import { invoke } from '@tauri-apps/api/core';

export interface ScreenshotPreferences {
  enabled: boolean;
  intervalSeconds: number;
  captureTarget: 'callWindow' | 'fullScreen';
  captureMode: 'interval' | 'speechEvent' | 'manualOnly';
  retentionDays: number;
}

export interface MeetingScreenshot {
  id: string;
  meetingId: string;
  capturedAt: string;
  recordingTime?: number | null;
  filePath?: string | null;
  thumbnailPath?: string | null;
  displayLabel?: string | null;
  status: string;
  redactionStatus: string;
  source: string;
  provider?: string | null;
  relevanceConfidence?: number | null;
  relevanceStatus?: string | null;
  captureTrigger?: string | null;
  speakerEvidence: boolean;
  skipReason?: string | null;
}

export interface ScreenshotCaptureStatus {
  meetingId: string;
  active: boolean;
  enabled: boolean;
  intervalSeconds: number;
  lastError?: string | null;
}

export async function getScreenshotPreferences(): Promise<ScreenshotPreferences> {
  return invoke<ScreenshotPreferences>('get_screenshot_preferences');
}

export async function setScreenshotPreferences(
  preferences: ScreenshotPreferences,
): Promise<ScreenshotPreferences> {
  return invoke<ScreenshotPreferences>('set_screenshot_preferences', { preferences });
}

export async function startMeetingScreenshotCapture(
  meetingId: string,
  recordingStartedAt?: string | null,
): Promise<ScreenshotCaptureStatus> {
  return invoke<ScreenshotCaptureStatus>('start_meeting_screenshot_capture', {
    meetingId,
    recordingStartedAt,
  });
}

export async function stopMeetingScreenshotCapture(
  meetingId: string,
): Promise<ScreenshotCaptureStatus> {
  return invoke<ScreenshotCaptureStatus>('stop_meeting_screenshot_capture', { meetingId });
}

export async function pauseMeetingScreenshotCapture(
  meetingId: string,
): Promise<ScreenshotCaptureStatus> {
  return invoke<ScreenshotCaptureStatus>('pause_meeting_screenshot_capture', { meetingId });
}

export async function resumeMeetingScreenshotCapture(
  meetingId: string,
): Promise<ScreenshotCaptureStatus> {
  return invoke<ScreenshotCaptureStatus>('resume_meeting_screenshot_capture', { meetingId });
}

export async function triggerMeetingScreenshotCapture(
  meetingId: string,
  recordingStartedAt?: string | null,
  triggerReason: 'speechEvent' | 'speakerChange' = 'speechEvent',
): Promise<ScreenshotCaptureStatus> {
  return invoke<ScreenshotCaptureStatus>('trigger_meeting_screenshot_capture', {
    meetingId,
    recordingStartedAt,
    triggerReason,
  });
}

export async function captureMeetingScreenshotNow(
  meetingId: string,
  recordingStartedAt?: string | null,
): Promise<MeetingScreenshot> {
  return invoke<MeetingScreenshot>('capture_meeting_screenshot_now', {
    meetingId,
    recordingStartedAt,
  });
}

export async function listMeetingScreenshots(meetingId: string): Promise<MeetingScreenshot[]> {
  return invoke<MeetingScreenshot[]>('list_meeting_screenshots', { meetingId });
}

export async function deleteMeetingScreenshot(
  screenshotId: string,
  deleteFile = true,
  removeMetadata = true,
): Promise<void> {
  return invoke<void>('delete_meeting_screenshot', { screenshotId, deleteFile, removeMetadata });
}

export async function attachMeetingScreenshots(
  fromMeetingId: string,
  toMeetingId: string,
): Promise<number> {
  return invoke<number>('attach_meeting_screenshots', { fromMeetingId, toMeetingId });
}
