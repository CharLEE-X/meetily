import { Summary } from '@/types';
import { AgentKind, AgentSetupStatus, McpStatus } from '@/services/mcpService';

export type AgentTarget = AgentKind | 'manual';
export type WorkflowMode = 'off' | 'ask' | 'auto';
export type WorkflowActionId =
  | 'review-summary'
  | 'extract-follow-ups'
  | 'draft-linear-issues'
  | 'draft-follow-up-message'
  | 'open-agent-handoff';

export interface AgentWorkflowSettings {
  defaultAgent: AgentTarget;
  mode: WorkflowMode;
  enabledActions: WorkflowActionId[];
  skillPackInstalled: boolean;
  skillPackVersion: string | null;
  updatedAt: string | null;
}

export interface AgentWorkflowRun {
  id: string;
  meetingId: string;
  meetingTitle: string;
  agent: AgentTarget;
  actions: WorkflowActionId[];
  mode: WorkflowMode;
  status: 'queued' | 'waitingForApproval' | 'prepared' | 'running' | 'completed' | 'failed' | 'canceled' | 'skipped';
  createdAt: string;
  message: string;
}

export interface AgentWorkflowContext {
  meetingId: string;
  meetingTitle: string;
  summary: Summary | { markdown?: string } | null;
  mcpUrl: string | null;
}

export interface PreparedAgentWorkflow {
  run: AgentWorkflowRun;
  prompt: string;
  canRun: boolean;
  reason: string | null;
}

export const MEETILY_SKILL_PACK_VERSION = '2026.06.18';

export const AGENT_WORKFLOW_ACTIONS: Array<{
  id: WorkflowActionId;
  label: string;
  description: string;
  requiresApproval: boolean;
}> = [
  {
    id: 'review-summary',
    label: 'Review summary',
    description: 'Ask the agent to check the summary for decisions, risks, and missing context.',
    requiresApproval: false,
  },
  {
    id: 'extract-follow-ups',
    label: 'Extract follow-ups',
    description: 'Ask the agent to identify owners, next steps, and ambiguous action items.',
    requiresApproval: false,
  },
  {
    id: 'draft-linear-issues',
    label: 'Draft Linear issues',
    description: 'Prepare reviewable Linear issue proposals. Creation always needs approval.',
    requiresApproval: true,
  },
  {
    id: 'draft-follow-up-message',
    label: 'Draft follow-up message',
    description: 'Prepare a concise email or chat follow-up from the meeting outcomes.',
    requiresApproval: false,
  },
  {
    id: 'open-agent-handoff',
    label: 'Open agent handoff',
    description: 'Prepare instructions for the selected agent or a manual MCP client.',
    requiresApproval: false,
  },
];

export const AGENT_SUPPORT_MATRIX: Array<{
  agent: AgentTarget;
  label: string;
  setup: string;
  invocation: string;
  handoff: string;
}> = [
  {
    agent: 'codex',
    label: 'Codex',
    setup: 'Meetily can write an MCP server entry to ~/.codex/config.toml.',
    invocation: 'Direct task execution is not launched by Meetily in this release.',
    handoff: 'Copy the generated prompt into Codex; it references the local MCP server.',
  },
  {
    agent: 'claude',
    label: 'Claude Desktop',
    setup: 'Meetily can write an MCP server entry to Claude Desktop config.',
    invocation: 'Meetily does not drive Claude Desktop automatically.',
    handoff: 'Open Claude and paste the generated prompt after setup.',
  },
  {
    agent: 'cursor',
    label: 'Cursor',
    setup: 'Meetily can write an MCP server entry to ~/.cursor/mcp.json.',
    invocation: 'Meetily does not start Cursor agent tasks automatically.',
    handoff: 'Open Cursor and paste the generated prompt after setup.',
  },
  {
    agent: 'manual',
    label: 'Manual MCP',
    setup: 'Use the documented MCP endpoint and trusted client token flow.',
    invocation: 'Manual only.',
    handoff: 'Copy the prompt into any authorized local MCP client.',
  },
];

const SETTINGS_KEY = 'meetily.agentWorkflowSettings';
const RUNS_KEY = 'meetily.agentWorkflowRuns';

const defaultSettings: AgentWorkflowSettings = {
  defaultAgent: 'manual',
  mode: 'off',
  enabledActions: ['review-summary', 'extract-follow-ups'],
  skillPackInstalled: false,
  skillPackVersion: null,
  updatedAt: null,
};

function hasLocalStorage(): boolean {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function readJson<T>(key: string, fallback: T): T {
  if (!hasLocalStorage()) return fallback;
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) return fallback;
    return JSON.parse(raw) as T;
  } catch (error) {
    console.warn(`Failed to read ${key}:`, error);
    return fallback;
  }
}

function writeJson<T>(key: string, value: T) {
  if (!hasLocalStorage()) return;
  window.localStorage.setItem(key, JSON.stringify(value));
}

function sanitizeSettings(value: Partial<AgentWorkflowSettings> | null | undefined): AgentWorkflowSettings {
  const allowedActions = new Set(AGENT_WORKFLOW_ACTIONS.map((action) => action.id));
  const enabledActions = Array.isArray(value?.enabledActions)
    ? value.enabledActions.filter((action): action is WorkflowActionId => allowedActions.has(action as WorkflowActionId))
    : defaultSettings.enabledActions;

  const defaultAgent = value?.defaultAgent && ['claude', 'codex', 'cursor', 'manual'].includes(value.defaultAgent)
    ? value.defaultAgent
    : defaultSettings.defaultAgent;
  const mode = value?.mode && ['off', 'ask', 'auto'].includes(value.mode)
    ? value.mode
    : defaultSettings.mode;

  return {
    defaultAgent,
    mode,
    enabledActions: enabledActions.length ? enabledActions : defaultSettings.enabledActions,
    skillPackInstalled: Boolean(value?.skillPackInstalled),
    skillPackVersion: value?.skillPackVersion ?? null,
    updatedAt: value?.updatedAt ?? null,
  };
}

export function getAgentWorkflowSettings(): AgentWorkflowSettings {
  return sanitizeSettings(readJson<Partial<AgentWorkflowSettings>>(SETTINGS_KEY, defaultSettings));
}

export function saveAgentWorkflowSettings(nextSettings: AgentWorkflowSettings): AgentWorkflowSettings {
  const sanitized = sanitizeSettings({
    ...nextSettings,
    updatedAt: new Date().toISOString(),
  });
  writeJson(SETTINGS_KEY, sanitized);
  return sanitized;
}

export function installMeetilySkillPack(current = getAgentWorkflowSettings()): AgentWorkflowSettings {
  return saveAgentWorkflowSettings({
    ...current,
    skillPackInstalled: true,
    skillPackVersion: MEETILY_SKILL_PACK_VERSION,
  });
}

export function removeMeetilySkillPack(current = getAgentWorkflowSettings()): AgentWorkflowSettings {
  return saveAgentWorkflowSettings({
    ...current,
    skillPackInstalled: false,
    skillPackVersion: null,
    mode: 'off',
  });
}

export function listAgentWorkflowRuns(): AgentWorkflowRun[] {
  return readJson<AgentWorkflowRun[]>(RUNS_KEY, []).slice(0, 25);
}

function saveRun(run: AgentWorkflowRun) {
  const next = [run, ...listAgentWorkflowRuns()].slice(0, 25);
  writeJson(RUNS_KEY, next);
}

function summaryToText(summary: AgentWorkflowContext['summary']): string {
  if (!summary) return 'No summary payload available.';
  if ('markdown' in summary && typeof summary.markdown === 'string') {
    return summary.markdown.slice(0, 6000);
  }

  return Object.entries(summary as Summary)
    .map(([key, section]) => {
      const blocks = section.blocks?.map((block) => `- ${block.content}`).join('\n') ?? '';
      return `## ${section.title || key}\n${blocks}`;
    })
    .join('\n\n')
    .slice(0, 6000);
}

export function buildLinearFollowUpTemplate(context: AgentWorkflowContext, actions: WorkflowActionId[]): string {
  const actionLabels = actions
    .map((actionId) => AGENT_WORKFLOW_ACTIONS.find((action) => action.id === actionId)?.label)
    .filter(Boolean)
    .join(', ');

  return [
    'You are helping process a Meetily meeting.',
    '',
    `Meeting: ${context.meetingTitle}`,
    `Meeting ID: ${context.meetingId}`,
    context.mcpUrl ? `Local MCP endpoint: ${context.mcpUrl}` : 'Local MCP endpoint: not enabled',
    `Requested workflows: ${actionLabels || 'Review summary'}`,
    '',
    'Privacy and safety rules:',
    '- Use only the summary/action items below or explicitly authorized MCP reads.',
    '- Do not request screenshots, raw transcript, or private files unless the user approves.',
    '- Do not create Linear issues automatically.',
    '- For Linear follow-ups, propose drafts first and wait for explicit user approval.',
    '',
    'If proposing Linear issues, return this structure:',
    '- title',
    '- description with concise meeting context',
    '- owner if known',
    '- priority suggestion',
    '- confidence',
    '- source meeting reference',
    '',
    'Meeting summary:',
    summaryToText(context.summary),
  ].join('\n');
}

export function prepareAgentWorkflow(
  context: AgentWorkflowContext,
  status: McpStatus | null,
  agentStatuses: AgentSetupStatus[],
  settings = getAgentWorkflowSettings()
): PreparedAgentWorkflow {
  const createdAt = new Date().toISOString();
  const baseRun = {
    id: `workflow-${Date.now()}`,
    meetingId: context.meetingId,
    meetingTitle: context.meetingTitle,
    agent: settings.defaultAgent,
    actions: settings.enabledActions,
    mode: settings.mode,
    createdAt,
  };

  if (settings.mode === 'off' || !settings.skillPackInstalled) {
    const run: AgentWorkflowRun = {
      ...baseRun,
      status: 'skipped',
      message: settings.skillPackInstalled
        ? 'Post-meeting workflows are disabled.'
        : 'Meetily skill pack is not installed.',
    };
    saveRun(run);
    return { run, prompt: '', canRun: false, reason: run.message };
  }

  const needsMcp = settings.defaultAgent !== 'manual';
  const selectedAgent = agentStatuses.find((agent) => agent.agent === settings.defaultAgent);
  if (needsMcp && (!status?.settings.enabled || !selectedAgent?.configured)) {
    const run: AgentWorkflowRun = {
      ...baseRun,
      status: 'failed',
      message: 'Enable MCP and configure the selected agent before running post-meeting workflows.',
    };
    saveRun(run);
    return { run, prompt: '', canRun: false, reason: run.message };
  }

  const prompt = buildLinearFollowUpTemplate(context, settings.enabledActions);
  const run: AgentWorkflowRun = {
    ...baseRun,
    status: settings.mode === 'ask' ? 'waitingForApproval' : 'prepared',
    message: settings.mode === 'ask'
      ? 'Workflow prepared and waiting for user approval.'
      : 'Workflow handoff prepared automatically.',
  };
  saveRun(run);
  return { run, prompt, canRun: true, reason: null };
}
