import type { AgentTarget, WorkflowActionId } from './agentWorkflowService';

export type AgentPromptTemplateId =
  | 'codex-implementation-handoff'
  | 'repo-investigation'
  | 'pr-review'
  | 'docs-update'
  | 'linear-jira-grooming'
  | 'incident-follow-up'
  | 'product-planning'
  | 'open-loop-review';

export interface AgentPromptTemplate {
  id: AgentPromptTemplateId;
  label: string;
  agentLean: 'codex' | 'claude' | 'either';
  description: string;
  body: string;
}

export interface AgentPromptTemplateInput {
  meetingTitle: string;
  meetingId: string;
  mcpUrl: string | null;
  agent: AgentTarget;
  actions: WorkflowActionId[];
  contextPackage: string;
  actionInstructions: string;
}

const safetyBlock = [
  'Safety rules:',
  '- Do not invent facts. If the meeting context does not support a claim, mark it as an assumption or ask.',
  '- Cite meeting ids, timestamps, and source ids from the context package for every concrete task or claim.',
  '- Ask before destructive local commands, external writes, ticket creation, PR creation, or customer-facing messages unless the user explicitly approved them.',
  '- Prefer MCP tool references over copying large meeting content when the local MCP endpoint is available.',
].join('\n');

const mcpBlock = [
  'Useful RecallX MCP tools when authorized:',
  '- meetily_get_latest_meeting',
  '- meetily_ask_meetings',
  '- meetily_get_open_loops',
  '- meetily_prepare_next_meeting',
  '- meetily_get_daily_digest',
  '- meetily_get_weekly_digest',
].join('\n');

export const DEFAULT_AGENT_PROMPT_TEMPLATES: AgentPromptTemplate[] = [
  {
    id: 'codex-implementation-handoff',
    label: 'Codex implementation handoff',
    agentLean: 'codex',
    description: 'Turn decisions and follow-ups into codebase work for Codex.',
    body: [
      'You are Codex working from a RecallX meeting handoff.',
      'Goal: inspect the local repository, identify implementation tasks from the cited meeting context, make scoped code changes, test them, and prepare a concise result.',
      '',
      '{{safetyBlock}}',
      '',
      'Use your local codebase, GitHub/GitLab, Linear/Jira, shell, and test tools as available. Do not treat meeting context as source code truth; verify in the repo first.',
      '',
      'Requested workflows:',
      '{{actionInstructions}}',
      '',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
  {
    id: 'repo-investigation',
    label: 'Repository investigation',
    agentLean: 'codex',
    description: 'Ask an agent to map meeting questions to code areas, risks, and next steps.',
    body: [
      'Investigate the repository based on this meeting.',
      'Find the relevant modules, recent commits, tests, and ownership context. Return likely implementation paths, risks, and open questions.',
      '',
      '{{safetyBlock}}',
      '',
      '{{actionInstructions}}',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
  {
    id: 'pr-review',
    label: 'PR review',
    agentLean: 'codex',
    description: 'Review an existing PR or diff against meeting decisions.',
    body: [
      'Review the current PR or diff against the meeting context.',
      'Prioritize correctness, regressions, missing tests, unclear scope, and deviations from cited decisions.',
      '',
      '{{safetyBlock}}',
      '',
      'Return findings first, ordered by severity, with file references when available.',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
  {
    id: 'docs-update',
    label: 'Docs update',
    agentLean: 'either',
    description: 'Draft or implement documentation changes from decisions and follow-ups.',
    body: [
      'Update or draft documentation from this meeting.',
      'Preserve source-backed facts, identify docs that need updates, and separate confirmed decisions from assumptions.',
      '',
      '{{safetyBlock}}',
      '',
      '{{actionInstructions}}',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
  {
    id: 'linear-jira-grooming',
    label: 'Linear/Jira issue grooming',
    agentLean: 'claude',
    description: 'Draft issue updates, new tickets, and grooming notes without silent writes.',
    body: [
      'Groom project-management work from this meeting.',
      'Draft Linear/Jira issues, updates, labels, owners, priority suggestions, dependencies, and acceptance criteria.',
      '',
      '{{safetyBlock}}',
      '',
      'Do not create or update external tickets until the user approves the exact payload.',
      '{{actionInstructions}}',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
  {
    id: 'incident-follow-up',
    label: 'Incident follow-up',
    agentLean: 'claude',
    description: 'Create incident actions, timeline gaps, owner follow-ups, and comms drafts.',
    body: [
      'Prepare incident follow-up from this meeting.',
      'Extract timeline facts, unresolved questions, mitigations, owners, customer/internal comms, and follow-up tickets.',
      '',
      '{{safetyBlock}}',
      '',
      '{{actionInstructions}}',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
  {
    id: 'product-planning',
    label: 'Product planning',
    agentLean: 'claude',
    description: 'Convert product discussion into options, decisions, risks, and planning artifacts.',
    body: [
      'Create a product-planning brief from this meeting.',
      'Include decisions, options, tradeoffs, customer impact, sequencing, risks, and follow-up questions.',
      '',
      '{{safetyBlock}}',
      '',
      '{{actionInstructions}}',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
  {
    id: 'open-loop-review',
    label: 'Open-loop review',
    agentLean: 'either',
    description: 'Find unresolved actions, questions, risks, and owner gaps.',
    body: [
      'Review this meeting for open loops.',
      'List unresolved questions, ownerless tasks, hidden dependencies, stale commitments, and follow-up messages needed.',
      '',
      '{{safetyBlock}}',
      '',
      '{{actionInstructions}}',
      '{{mcpBlock}}',
      '',
      '{{contextPackage}}',
    ].join('\n'),
  },
];

export function getAgentPromptTemplate(id: string | null | undefined): AgentPromptTemplate {
  return DEFAULT_AGENT_PROMPT_TEMPLATES.find((template) => template.id === id)
    ?? DEFAULT_AGENT_PROMPT_TEMPLATES[0];
}

export function renderAgentPromptTemplate(
  templateId: string | null | undefined,
  input: AgentPromptTemplateInput,
  overrides: Record<string, string> = {}
): string {
  const template = getAgentPromptTemplate(templateId);
  const body = overrides[template.id]?.trim() || template.body;
  const replacements: Record<string, string> = {
    meetingTitle: input.meetingTitle,
    meetingId: input.meetingId,
    mcpUrl: input.mcpUrl ?? 'not enabled',
    agent: input.agent,
    actions: input.actions.join(', '),
    actionInstructions: input.actionInstructions,
    contextPackage: input.contextPackage,
    safetyBlock,
    mcpBlock,
  };

  return body.replace(/\{\{(\w+)\}\}/g, (_, key: string) => replacements[key] ?? '');
}
