import { invoke } from '@tauri-apps/api/core';
import { recordRecordingAuditEvent } from './recordingAuditService';

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
  const saved = await invoke<ScreenshotPreferences>('set_screenshot_preferences', { preferences });
  recordRecordingAuditEvent({
    type: saved.enabled ? 'screenshot_capture_enabled' : 'screenshot_capture_disabled',
    actor: 'settings',
    metadata: {
      enabled: saved.enabled,
      captureTarget: saved.captureTarget,
      captureMode: saved.captureMode,
    },
  });
  return saved;
}

export async function startMeetingScreenshotCapture(
  meetingId: string,
  recordingStartedAt?: string | null,
): Promise<ScreenshotCaptureStatus> {
  const status = await invoke<ScreenshotCaptureStatus>('start_meeting_screenshot_capture', {
    meetingId,
    recordingStartedAt,
  });
  recordRecordingAuditEvent({
    type: 'screenshot_capture_started',
    meetingId,
    actor: 'recording-assistant',
    metadata: {
      enabled: status.enabled,
      status: status.active ? 'active' : 'inactive',
    },
  });
  return status;
}

export async function stopMeetingScreenshotCapture(
  meetingId: string,
): Promise<ScreenshotCaptureStatus> {
  const status = await invoke<ScreenshotCaptureStatus>('stop_meeting_screenshot_capture', { meetingId });
  recordRecordingAuditEvent({
    type: 'screenshot_capture_stopped',
    meetingId,
    actor: 'recording-assistant',
    metadata: {
      enabled: status.enabled,
      status: status.active ? 'active' : 'inactive',
    },
  });
  return status;
}

export async function pauseMeetingScreenshotCapture(
  meetingId: string,
): Promise<ScreenshotCaptureStatus> {
  const status = await invoke<ScreenshotCaptureStatus>('pause_meeting_screenshot_capture', { meetingId });
  recordRecordingAuditEvent({
    type: 'screenshot_capture_paused',
    meetingId,
    actor: 'user',
    metadata: { status: status.active ? 'active' : 'inactive' },
  });
  return status;
}

export async function resumeMeetingScreenshotCapture(
  meetingId: string,
): Promise<ScreenshotCaptureStatus> {
  const status = await invoke<ScreenshotCaptureStatus>('resume_meeting_screenshot_capture', { meetingId });
  recordRecordingAuditEvent({
    type: 'screenshot_capture_resumed',
    meetingId,
    actor: 'user',
    metadata: { status: status.active ? 'active' : 'inactive' },
  });
  return status;
}

export async function triggerMeetingScreenshotCapture(
  meetingId: string,
  recordingStartedAt?: string | null,
  triggerReason: 'speechEvent' | 'speakerChange' = 'speechEvent',
): Promise<ScreenshotCaptureStatus> {
  const status = await invoke<ScreenshotCaptureStatus>('trigger_meeting_screenshot_capture', {
    meetingId,
    recordingStartedAt,
    triggerReason,
  });
  recordRecordingAuditEvent({
    type: 'screenshot_capture_triggered',
    meetingId,
    actor: 'recording-assistant',
    metadata: { triggerReason, status: status.active ? 'active' : 'inactive' },
  });
  return status;
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
  await invoke<void>('delete_meeting_screenshot', { screenshotId, deleteFile, removeMetadata });
  recordRecordingAuditEvent({
    type: 'screenshot_images_deleted',
    actor: 'user',
    metadata: {
      action: removeMetadata && deleteFile
        ? 'delete-metadata-and-image'
        : removeMetadata
          ? 'delete-metadata'
          : deleteFile
            ? 'delete-image'
            : 'delete-record',
    },
  });
}

export async function attachMeetingScreenshots(
  fromMeetingId: string,
  toMeetingId: string,
): Promise<number> {
  return invoke<number>('attach_meeting_screenshots', { fromMeetingId, toMeetingId });
}
