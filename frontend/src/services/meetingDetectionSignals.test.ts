import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildAmbientMeetingCandidate,
  detectMeetingProviderFromText,
  scoreMeetingActivitySignals,
} from './meetingDetectionSignals.ts';

const now = new Date('2026-06-18T14:00:00.000Z');

test('detects meeting providers from URLs and call window titles', () => {
  assert.equal(detectMeetingProviderFromText('https://meet.google.com/abc-defg-hij'), 'google-meet');
  assert.equal(detectMeetingProviderFromText('Join Microsoft Teams Meeting'), 'teams');
  assert.equal(detectMeetingProviderFromText('Zoom Meeting 123 456'), 'zoom');
  assert.equal(detectMeetingProviderFromText('Slack huddle with engineering'), 'slack');
  assert.equal(detectMeetingProviderFromText('Slack call with engineering'), 'slack');
  assert.equal(detectMeetingProviderFromText('Engineering huddle planning doc'), 'unknown');
  assert.equal(detectMeetingProviderFromText('Quarterly planning'), 'unknown');
});

test('scores active Teams window plus active mic as high confidence', () => {
  const result = scoreMeetingActivitySignals({
    activeAppName: 'Microsoft Teams',
    activeWindowTitle: 'Weekly Sync | Microsoft Teams',
    runningApps: ['Microsoft Teams'],
    browserTabs: [],
    micActivity: { isActive: true, peakLevel: 0.12, rmsLevel: 0.04 },
    checkedAt: now.toISOString(),
  });

  assert.equal(result.isLikelyMeeting, true);
  assert.equal(result.provider, 'teams');
  assert.ok(result.confidence >= 80);
  assert.ok(result.reasons.includes('Active meeting window'));
  assert.ok(result.reasons.includes('Microphone activity'));
});

test('builds openable ambient candidate from active browser meeting URL', () => {
  const candidate = buildAmbientMeetingCandidate({
    activeAppName: 'Google Chrome',
    activeWindowTitle: 'Google Meet',
    runningApps: ['Google Chrome'],
    browserTabs: [{
      browser: 'Google Chrome',
      provider: 'google-meet',
      title: 'Daily Standup - Google Meet',
      url: 'https://meet.google.com/abc-defg-hij',
      isActive: true,
      permissionStatus: 'available',
      checkedAt: now.toISOString(),
      freshnessMs: 120,
    }],
    micActivity: { isActive: false, peakLevel: 0, rmsLevel: 0 },
    checkedAt: now.toISOString(),
  }, now);

  assert.ok(candidate);
  assert.equal(candidate.provider, 'google-meet');
  assert.equal(candidate.meetingUrl, 'https://meet.google.com/abc-defg-hij');
  assert.equal(candidate.source, 'ambient');
});

test('scores Slack huddle from active window metadata and exposes window bounds', () => {
  const result = scoreMeetingActivitySignals({
    activeAppName: 'Slack',
    activeWindowTitle: 'Engineering huddle - Slack',
    activeWindowBounds: { x: 120, y: 80, width: 1280, height: 720 },
    runningApps: ['Slack'],
    browserTabs: [],
    micActivity: { isActive: true, peakLevel: 0.19, rmsLevel: 0.07 },
    checkedAt: now.toISOString(),
    signalFreshnessMs: 75,
  });

  assert.equal(result.isLikelyMeeting, true);
  assert.equal(result.provider, 'slack');
  assert.ok(result.reasons.includes('Active meeting window'));
});

test('reports degraded permission state without making process-only signals eligible', () => {
  const result = scoreMeetingActivitySignals({
    activeAppName: 'Notes',
    activeWindowTitle: null,
    runningApps: ['Microsoft Teams'],
    browserTabs: [],
    micActivity: { isActive: false, peakLevel: 0, rmsLevel: 0 },
    checkedAt: now.toISOString(),
    missingPermissions: ['accessibility', 'browserAutomation'],
    permissionStatus: {
      activeWindow: 'denied',
      browserAutomation: 'limited',
    },
    degradedMode: true,
  });

  assert.equal(result.isLikelyMeeting, false);
  assert.equal(result.provider, 'unknown');
  assert.ok(result.confidence < 65);
  assert.ok(result.reasons.includes('Limited by missing permissions'));
});

test('does not prompt from Slack process-only plus microphone activity', () => {
  const candidate = buildAmbientMeetingCandidate({
    activeAppName: 'Slack',
    activeWindowTitle: 'Slack | Engineering',
    runningApps: ['Slack'],
    browserTabs: [],
    micActivity: { isActive: true, peakLevel: 0.22, rmsLevel: 0.08 },
    checkedAt: now.toISOString(),
  }, now);

  assert.equal(candidate, null);
});

test('ignores weak process-only signals', () => {
  const candidate = buildAmbientMeetingCandidate({
    activeAppName: 'Notes',
    activeWindowTitle: 'Notes',
    runningApps: ['Microsoft Teams'],
    browserTabs: [],
    micActivity: { isActive: false, peakLevel: 0, rmsLevel: 0 },
    checkedAt: now.toISOString(),
  }, now);

  assert.equal(candidate, null);
});
