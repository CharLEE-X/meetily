export type RecordingAuditEventType =
  | 'recording_preflight_shown'
  | 'recording_started_with_scope'
  | 'screenshot_capture_enabled'
  | 'screenshot_capture_disabled'
  | 'screenshot_capture_started'
  | 'screenshot_capture_stopped'
  | 'screenshot_capture_paused'
  | 'screenshot_capture_resumed'
  | 'screenshot_capture_triggered'
  | 'screenshot_images_deleted'
  | 'speaker_labeling_enabled'
  | 'speaker_labeling_disabled'
  | 'speaker_labels_cleared'
  | 'calendar_context_attached'
  | 'calendar_context_detached'
  | 'notes_export_reviewed'
  | 'reminders_reviewed'
  | 'agent_automation_reviewed'
  | 'agent_automation_disabled'
  | 'sensitive_capture_stopped';

export type RecordingAuditActor = 'user' | 'system' | 'recording-assistant' | 'settings';

export type RecordingAuditMetadataValue = string | number | boolean | null | RecordingAuditMetadataValue[];

export interface RecordingAuditEvent {
  id: string;
  type: RecordingAuditEventType;
  meetingId?: string;
  timestamp: string;
  actor: RecordingAuditActor;
  metadata: Record<string, RecordingAuditMetadataValue>;
}

export interface RecordingAuditEventInput {
  type: RecordingAuditEventType;
  meetingId?: string | null;
  actor?: RecordingAuditActor;
  metadata?: Record<string, unknown>;
  timestamp?: string;
}

const STORAGE_KEY = 'meetily.recordingAuditEvents';
const MAX_EVENTS = 200;
const MAX_STRING_LENGTH = 96;
const CHANGE_EVENT = 'meetily-recording-audit-events-changed';

const ALLOWED_METADATA_KEYS = new Set([
  'action',
  'autoApplyVisualSuggestions',
  'captureMode',
  'captureTarget',
  'destination',
  'enabled',
  'includeConfirmed',
  'meetingSource',
  'mode',
  'permission',
  'provider',
  'reason',
  'reviewRequired',
  'scope',
  'source',
  'state',
  'status',
  'target',
  'triggerReason',
]);

const SENSITIVE_KEY_PATTERN = /(transcript|summary|ocr|image|payload|token|secret|key|description|attendee|email|windowtitle|title|text|prompt|content|path|url|device)/i;

function canUseStorage() {
  return typeof window !== 'undefined' && Boolean(window.localStorage);
}

function sanitizePrimitive(value: unknown): RecordingAuditMetadataValue | undefined {
  if (typeof value === 'boolean' || typeof value === 'number' || value === null) {
    return value;
  }

  if (typeof value === 'string') {
    return value.length > MAX_STRING_LENGTH ? `${value.slice(0, MAX_STRING_LENGTH)}...` : value;
  }

  if (Array.isArray(value)) {
    const sanitizedItems = value
      .map(sanitizePrimitive)
      .filter((item): item is RecordingAuditMetadataValue => item !== undefined)
      .slice(0, 8);
    return sanitizedItems;
  }

  return undefined;
}

export function sanitizeAuditMetadata(metadata: Record<string, unknown> = {}) {
  return Object.entries(metadata).reduce<Record<string, RecordingAuditMetadataValue>>((safe, [key, value]) => {
    if (!ALLOWED_METADATA_KEYS.has(key) || SENSITIVE_KEY_PATTERN.test(key)) {
      return safe;
    }

    const sanitized = sanitizePrimitive(value);
    if (sanitized !== undefined) {
      safe[key] = sanitized;
    }
    return safe;
  }, {});
}

export function serializeAuditEvent(input: RecordingAuditEventInput): RecordingAuditEvent {
  return {
    id: `recording-audit-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    type: input.type,
    meetingId: input.meetingId || undefined,
    timestamp: input.timestamp ?? new Date().toISOString(),
    actor: input.actor ?? 'user',
    metadata: sanitizeAuditMetadata(input.metadata),
  };
}

export function readRecordingAuditEvents(): RecordingAuditEvent[] {
  if (!canUseStorage()) {
    return [];
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return [];
    }

    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed
      .filter((event): event is RecordingAuditEvent => (
        typeof event?.id === 'string' &&
        typeof event?.type === 'string' &&
        typeof event?.timestamp === 'string' &&
        typeof event?.actor === 'string' &&
        typeof event?.metadata === 'object' &&
        event.metadata !== null
      ))
      .slice(0, MAX_EVENTS);
  } catch (error) {
    console.warn('Failed to read recording audit events:', error);
    return [];
  }
}

export function listRecordingAuditEvents(options: { meetingId?: string | null; limit?: number } = {}) {
  const events = readRecordingAuditEvents();
  const filtered = options.meetingId
    ? events.filter((event) => event.meetingId === options.meetingId)
    : events;
  return filtered.slice(0, options.limit ?? 20);
}

export function recordRecordingAuditEvent(input: RecordingAuditEventInput): RecordingAuditEvent | null {
  if (!canUseStorage()) {
    return null;
  }

  const event = serializeAuditEvent(input);
  const next = [event, ...readRecordingAuditEvents()].slice(0, MAX_EVENTS);

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    window.dispatchEvent(new CustomEvent(CHANGE_EVENT, { detail: event }));
    return event;
  } catch (error) {
    console.warn('Failed to record recording audit event:', error);
    return null;
  }
}

export function subscribeToRecordingAuditEvents(callback: () => void) {
  if (typeof window === 'undefined') {
    return () => {};
  }

  window.addEventListener(CHANGE_EVENT, callback);
  window.addEventListener('storage', callback);
  return () => {
    window.removeEventListener(CHANGE_EVENT, callback);
    window.removeEventListener('storage', callback);
  };
}
