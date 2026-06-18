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
      title: 'Daily Standup - Google Meet',
      url: 'https://meet.google.com/abc-defg-hij',
      isActive: true,
    }],
    micActivity: { isActive: false, peakLevel: 0, rmsLevel: 0 },
    checkedAt: now.toISOString(),
  }, now);

  assert.ok(candidate);
  assert.equal(candidate.provider, 'google-meet');
  assert.equal(candidate.meetingUrl, 'https://meet.google.com/abc-defg-hij');
  assert.equal(candidate.source, 'ambient');
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
