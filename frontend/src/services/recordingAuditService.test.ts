// @ts-nocheck
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  sanitizeAuditMetadata,
  serializeAuditEvent,
} from './recordingAuditService.ts';

test('serializes audit events with actor, timestamp, meeting id, and safe metadata', () => {
  const event = serializeAuditEvent({
    type: 'screenshot_capture_started',
    meetingId: 'meeting-123',
    actor: 'recording-assistant',
    timestamp: '2026-06-20T09:00:00.000Z',
    metadata: {
      enabled: true,
      captureTarget: 'callWindow',
      captureMode: 'speechEvent',
      provider: 'macos-window-capture',
    },
  });

  assert.equal(event.type, 'screenshot_capture_started');
  assert.equal(event.meetingId, 'meeting-123');
  assert.equal(event.actor, 'recording-assistant');
  assert.equal(event.timestamp, '2026-06-20T09:00:00.000Z');
  assert.deepEqual(event.metadata, {
    enabled: true,
    captureTarget: 'callWindow',
    captureMode: 'speechEvent',
    provider: 'macos-window-capture',
  });
});

test('redacts transcript text, screenshots, tokens, calendar descriptions, and private identifiers', () => {
  const metadata = sanitizeAuditMetadata({
    enabled: true,
    transcriptText: 'The raw transcript must never be stored here.',
    screenshotImagePayload: 'base64-image-data',
    apiToken: 'sk-secret',
    calendarDescription: 'Private agenda text',
    attendeeEmails: ['person@example.com'],
    meetingTitle: 'Private customer roadmap call',
    deviceName: 'Adrian microphone',
    source: 'calendar',
    reviewRequired: true,
  });

  assert.deepEqual(metadata, {
    enabled: true,
    source: 'calendar',
    reviewRequired: true,
  });
});

test('ignores unapproved metadata keys and truncates approved strings', () => {
  const metadata = sanitizeAuditMetadata({
    source: 'x'.repeat(140),
    arbitraryObject: { nested: 'not allowed' },
    count: 3,
    reason: 'manual-stop',
  });

  assert.equal(typeof metadata.source, 'string');
  assert.ok(String(metadata.source).length <= 99);
  assert.equal(metadata.reason, 'manual-stop');
  assert.equal('count' in metadata, false);
  assert.equal('arbitraryObject' in metadata, false);
});
