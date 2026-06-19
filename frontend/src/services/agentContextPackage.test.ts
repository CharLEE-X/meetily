import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildAgentContextPackage,
  resolveAgentContextBudget,
  serializeAgentContextPackage,
} from './agentContextPackage.ts';

const baseInput = {
  meetingId: 'meeting-123',
  meetingTitle: 'Platform planning',
  meetingStartedAt: '2026-06-20T09:00:00.000Z',
  summaryText: 'We agreed to let agents own GitHub and Linear work while Meetily prepares context.',
  actionItems: [{
    id: 'action-1',
    text: 'Adrian will prepare the Codex handoff flow.',
    timestamp: '00:12:03',
    sourceLabel: 'Action Items',
  }],
  decisions: [{
    id: 'decision-1',
    text: 'Meetily will not create Linear issues directly.',
    timestamp: '00:08:14',
    sourceLabel: 'Key Decisions',
  }],
  risks: [{
    id: 'risk-1',
    text: 'Silent external writes would violate the consent model.',
    timestamp: '00:18:44',
    sourceLabel: 'Risks',
  }],
  transcriptExcerpts: [
    { id: 'segment-1', text: 'First source-backed transcript segment.', startsAt: '00:01:00', endsAt: '00:01:08' },
    { id: 'segment-2', text: 'Second source-backed transcript segment.', startsAt: '00:02:00', endsAt: '00:02:08' },
    { id: 'segment-3', text: 'Third source-backed transcript segment.', startsAt: '00:03:00', endsAt: '00:03:08' },
  ],
  screenshotsOcr: [{ id: 'shot-1', text: 'Visible dashboard text from screen OCR.' }],
  calendarMetadata: [{ id: 'cal-1', text: 'Calendar event: Platform planning at 09:00.' }],
  artifacts: [{ id: 'notes-1', text: 'Apple Notes export exists in Meetily folder.' }],
};

test('builds source-cited summary, decisions, actions, and risks by default', () => {
  const pkg = buildAgentContextPackage(baseInput, { generatedAt: '2026-06-20T10:00:00.000Z' });

  assert.equal(pkg.meeting.id, 'meeting-123');
  assert.deepEqual(pkg.sections.map((section) => section.sourceType), [
    'summary',
    'decision',
    'action_item',
    'risk',
  ]);
  assert.equal(pkg.sections[1].citations[0].id, 'decision-1');
  assert.equal(pkg.sections[1].citations[0].timestamp, '00:08:14');
  assert.equal(pkg.sections[2].citations[0].sourceLabel, 'Action Items');
});

test('requires explicit consent for transcript, screenshots, calendar metadata, and artifacts', () => {
  const pkg = buildAgentContextPackage(baseInput);

  assert.ok(pkg.omittedSources.some((source) => source.sourceType === 'transcript'));
  assert.ok(pkg.omittedSources.some((source) => source.sourceType === 'screenshot_ocr'));
  assert.ok(pkg.omittedSources.some((source) => source.sourceType === 'calendar'));
  assert.ok(pkg.omittedSources.some((source) => source.sourceType === 'artifact'));
  assert.ok(pkg.redactions.some((redaction) => redaction.includes('Transcript Excerpts omitted')));
});

test('applies budget presets and transcript excerpt limits', () => {
  const pkg = buildAgentContextPackage(baseInput, {
    budgetPreset: 'minimal',
    consent: { includeTranscriptExcerpts: true },
  });
  const transcript = pkg.sections.find((section) => section.sourceType === 'transcript');

  assert.equal(resolveAgentContextBudget('minimal').maxTranscriptExcerpts, 1);
  assert.equal(transcript?.citations.length, 1);
  assert.ok(pkg.omittedSources.some((source) => source.sourceType === 'transcript' && source.count === 2));
});

test('supports custom budgets and marks sections truncated when character budget is exhausted', () => {
  const pkg = buildAgentContextPackage({
    ...baseInput,
    summaryText: 'A'.repeat(1200),
  }, {
    budgetPreset: 'custom',
    customBudget: { maxCharacters: 900, maxTranscriptExcerpts: 2 },
  });

  assert.equal(pkg.budget.preset, 'custom');
  assert.equal(pkg.budget.maxCharacters, 900);
  assert.equal(pkg.sections[0].truncated, true);
  assert.ok(pkg.sections[0].content.length <= 910);
});

test('serializes package for Codex and keeps source references visible', () => {
  const pkg = buildAgentContextPackage(baseInput, {
    consent: { includeTranscriptExcerpts: true, includeCalendarMetadata: true },
  });
  const serialized = serializeAgentContextPackage(pkg, 'codex');

  assert.match(serialized, /Target: codex/);
  assert.match(serialized, /Meeting ID: meeting-123/);
  assert.match(serialized, /\[decision-1\] Meetily will not create Linear issues directly\./);
  assert.match(serialized, /- \[segment-1\] transcript @ 00:01:00-00:01:08/);
  assert.match(serialized, /- \[cal-1\] calendar/);
});
