import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DEFAULT_AGENT_PROMPT_TEMPLATES,
  renderAgentPromptTemplate,
} from './agentPromptTemplates.ts';

const baseInput = {
  meetingTitle: 'Engineering sync',
  meetingId: 'meeting-123',
  mcpUrl: 'http://127.0.0.1:43118/mcp',
  agent: 'codex' as const,
  actions: ['review-summary', 'draft-linear-issues'] as const,
  actionInstructions: '- Draft source-backed implementation tasks.',
  contextPackage: 'Source [S1] meeting evidence.',
};

test('ships the required post-meeting prompt templates', () => {
  const ids = DEFAULT_AGENT_PROMPT_TEMPLATES.map((template) => template.id);

  assert.deepEqual(ids, [
    'codex-implementation-handoff',
    'repo-investigation',
    'pr-review',
    'docs-update',
    'linear-jira-grooming',
    'incident-follow-up',
    'product-planning',
    'open-loop-review',
  ]);
});

test('renders safety instructions, MCP tool references, and cited context', () => {
  const prompt = renderAgentPromptTemplate('codex-implementation-handoff', baseInput);

  assert.match(prompt, /Do not invent facts/);
  assert.match(prompt, /Ask before destructive local commands/);
  assert.match(prompt, /meetily_get_latest_meeting/);
  assert.match(prompt, /Source \[S1\] meeting evidence/);
  assert.match(prompt, /local repository/);
});

test('supports local template overrides', () => {
  const prompt = renderAgentPromptTemplate('open-loop-review', baseInput, {
    'open-loop-review': 'Custom {{meetingId}} {{contextPackage}}',
  });

  assert.equal(prompt, 'Custom meeting-123 Source [S1] meeting evidence.');
});
