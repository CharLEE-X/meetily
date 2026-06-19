import { Summary } from '@/types';
import { AgentKind, AgentSetupStatus, McpStatus } from '@/services/mcpService';
import {
  AgentContextBudgetPreset,
  AgentContextConsent,
  AgentContextInputItem,
  buildAgentContextPackage,
  serializeAgentContextPackage,
} from './agentContextPackage';
import {
  AgentPromptTemplateId,
  DEFAULT_AGENT_PROMPT_TEMPLATES,
  getAgentPromptTemplate,
  renderAgentPromptTemplate,
} from './agentPromptTemplates';

export type AgentTarget = AgentKind | 'manual';
export type WorkflowMode = 'off' | 'ask' | 'auto';
export type WorkflowActionId =
  | 'review-summary'
  | 'extract-follow-ups'
  | 'draft-linear-issues'
  | 'draft-follow-up-message'
  | 'daily-digest'
  | 'weekly-digest'
  | 'next-meeting-prep'
  | 'open-loops'
  | 'project-status-update'
  | 'decision-log'
  | 'role-brief'
  | 'open-agent-handoff';

export interface AgentWorkflowSettings {
  defaultAgent: AgentTarget;
  mode: WorkflowMode;
  enabledActions: WorkflowActionId[];
  budgetPreset: AgentContextBudgetPreset;
  consent: AgentContextConsent;
  rules: AgentWorkflowRule[];
  promptTemplateId: AgentPromptTemplateId;
  promptTemplateOverrides: Record<string, string>;
  skillPackInstalled: boolean;
  skillPackVersion: string | null;
  updatedAt: string | null;
}

export interface AgentWorkflowRule {
  id: string;
  name: string;
  enabled: boolean;
  agent: AgentTarget;
  mode: WorkflowMode;
  enabledActions: WorkflowActionId[];
  budgetPreset: AgentContextBudgetPreset;
  promptTemplateId: AgentPromptTemplateId;
  templateId: string;
  match: {
    titleKeywords: string[];
    calendarKeywords: string[];
    projectKeywords: string[];
    templateIds: string[];
  };
  consent: AgentContextConsent;
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
  meetingStartedAt?: string | null;
  meetingEndedAt?: string | null;
  templateId?: string | null;
  calendarText?: string | null;
  projectText?: string | null;
  summary: Summary | { markdown?: string } | null;
  mcpUrl: string | null;
  budgetPreset?: AgentContextBudgetPreset;
  customBudget?: {
    maxCharacters?: number;
    maxTranscriptExcerpts?: number;
  };
  consent?: AgentContextConsent;
  actionItems?: AgentContextInputItem[];
  decisions?: AgentContextInputItem[];
  risks?: AgentContextInputItem[];
  transcriptExcerpts?: AgentContextInputItem[];
  screenshotsOcr?: AgentContextInputItem[];
  calendarMetadata?: AgentContextInputItem[];
  artifacts?: AgentContextInputItem[];
}

export interface ResolvedAgentWorkflowRule {
  settings: AgentWorkflowSettings;
  matchedRule: AgentWorkflowRule | null;
  preview: string;
  blockedReason: string | null;
}

export interface PreparedAgentWorkflow {
  run: AgentWorkflowRun;
  prompt: string;
  canRun: boolean;
  reason: string | null;
}

export const MEETILY_SKILL_PACK_VERSION = '2026.06.19';
export { DEFAULT_AGENT_PROMPT_TEMPLATES, getAgentPromptTemplate };

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
    id: 'daily-digest',
    label: 'Daily digest',
    description: 'Summarize the day into decisions, commitments, risks, and open questions.',
    requiresApproval: false,
  },
  {
    id: 'weekly-digest',
    label: 'Weekly digest',
    description: 'Group recent meetings into repeated themes, commitments, decisions, and risks.',
    requiresApproval: false,
  },
  {
    id: 'next-meeting-prep',
    label: 'Next-meeting prep',
    description: 'Prepare context, unresolved questions, and suggested agenda points for the next related call.',
    requiresApproval: false,
  },
  {
    id: 'open-loops',
    label: 'Open loops',
    description: 'Find unresolved questions, ownerless action items, and follow-ups that need confirmation.',
    requiresApproval: false,
  },
  {
    id: 'project-status-update',
    label: 'Project status update',
    description: 'Turn meeting outcomes into a stakeholder-ready project status draft.',
    requiresApproval: false,
  },
  {
    id: 'decision-log',
    label: 'Decision log',
    description: 'Extract decision records with rationale, owner, confidence, and source references.',
    requiresApproval: false,
  },
  {
    id: 'role-brief',
    label: 'Role-based brief',
    description: 'Prepare product, engineering, sales, hiring, manager, founder, or customer-success briefs.',
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

const defaultConsent: AgentContextConsent = {
  includeSummary: true,
  includeActionItems: true,
  includeDecisions: true,
  includeRisks: true,
  includeTranscriptExcerpts: false,
  includeFullTranscript: false,
  includeScreenshotsOcr: false,
  includeCalendarMetadata: false,
  includeArtifacts: false,
};

const defaultSettings: AgentWorkflowSettings = {
  defaultAgent: 'manual',
  mode: 'off',
  enabledActions: ['review-summary', 'extract-follow-ups'],
  budgetPreset: 'standard',
  consent: defaultConsent,
  rules: [],
  promptTemplateId: 'codex-implementation-handoff',
  promptTemplateOverrides: {},
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
  const allowedAgents = new Set<AgentTarget>(['claude', 'codex', 'cursor', 'manual']);
  const allowedModes = new Set<WorkflowMode>(['off', 'ask', 'auto']);
  const allowedBudgets = new Set<AgentContextBudgetPreset>(['minimal', 'standard', 'detailed', 'custom']);
  const allowedPromptTemplates = new Set(DEFAULT_AGENT_PROMPT_TEMPLATES.map((template) => template.id));
  const enabledActions = Array.isArray(value?.enabledActions)
    ? value.enabledActions.filter((action): action is WorkflowActionId => allowedActions.has(action as WorkflowActionId))
    : defaultSettings.enabledActions;

  const defaultAgent = value?.defaultAgent && allowedAgents.has(value.defaultAgent)
    ? value.defaultAgent
    : defaultSettings.defaultAgent;
  const mode = value?.mode && allowedModes.has(value.mode)
    ? value.mode
    : defaultSettings.mode;
  const budgetPreset = value?.budgetPreset && allowedBudgets.has(value.budgetPreset)
    ? value.budgetPreset
    : defaultSettings.budgetPreset;

  const consent = {
    ...defaultConsent,
    ...(value?.consent ?? {}),
  };
  const promptTemplateId = value?.promptTemplateId && allowedPromptTemplates.has(value.promptTemplateId)
    ? value.promptTemplateId
    : defaultSettings.promptTemplateId;
  const promptTemplateOverrides = value?.promptTemplateOverrides && typeof value.promptTemplateOverrides === 'object'
    ? Object.fromEntries(
      Object.entries(value.promptTemplateOverrides)
        .filter(([key, override]) => allowedPromptTemplates.has(key as AgentPromptTemplateId) && typeof override === 'string')
        .map(([key, override]) => [key, String(override).slice(0, 20000)])
    )
    : {};

  const rules = Array.isArray(value?.rules)
    ? value.rules.map((rule, index): AgentWorkflowRule => {
      const ruleActions = Array.isArray(rule.enabledActions)
        ? rule.enabledActions.filter((action): action is WorkflowActionId => allowedActions.has(action as WorkflowActionId))
        : enabledActions;
      return {
        id: typeof rule.id === 'string' && rule.id ? rule.id : `rule-${index + 1}`,
        name: typeof rule.name === 'string' && rule.name.trim() ? rule.name.trim() : `Rule ${index + 1}`,
        enabled: rule.enabled !== false,
        agent: rule.agent && allowedAgents.has(rule.agent) ? rule.agent : defaultAgent,
        mode: rule.mode && allowedModes.has(rule.mode) ? rule.mode : mode,
        enabledActions: ruleActions.length ? ruleActions : enabledActions,
        budgetPreset: rule.budgetPreset && allowedBudgets.has(rule.budgetPreset) ? rule.budgetPreset : budgetPreset,
        promptTemplateId: rule.promptTemplateId && allowedPromptTemplates.has(rule.promptTemplateId) ? rule.promptTemplateId : promptTemplateId,
        templateId: typeof rule.templateId === 'string' ? rule.templateId.trim() : '',
        match: {
          titleKeywords: normalizeKeywords(rule.match?.titleKeywords),
          calendarKeywords: normalizeKeywords(rule.match?.calendarKeywords),
          projectKeywords: normalizeKeywords(rule.match?.projectKeywords),
          templateIds: normalizeKeywords(rule.match?.templateIds),
        },
        consent: {
          ...consent,
          ...(rule.consent ?? {}),
        },
      };
    })
    : [];

  return {
    defaultAgent,
    mode,
    enabledActions: enabledActions.length ? enabledActions : defaultSettings.enabledActions,
    budgetPreset,
    consent,
    rules,
    promptTemplateId,
    promptTemplateOverrides,
    skillPackInstalled: Boolean(value?.skillPackInstalled),
    skillPackVersion: value?.skillPackVersion ?? null,
    updatedAt: value?.updatedAt ?? null,
  };
}

function normalizeKeywords(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => typeof item === 'string' ? item.trim().toLowerCase() : '')
    .filter(Boolean)
    .slice(0, 12);
}

function containsKeyword(text: string | null | undefined, keywords: string[]): boolean {
  if (keywords.length === 0) return true;
  const haystack = (text ?? '').toLowerCase();
  return keywords.some((keyword) => haystack.includes(keyword));
}

function ruleMatches(context: AgentWorkflowContext, rule: AgentWorkflowRule): boolean {
  return (
    containsKeyword(context.meetingTitle, rule.match.titleKeywords) &&
    containsKeyword(context.calendarText, rule.match.calendarKeywords) &&
    containsKeyword(context.projectText, rule.match.projectKeywords) &&
    (rule.match.templateIds.length === 0 || (context.templateId ? rule.match.templateIds.includes(context.templateId.toLowerCase()) : false))
  );
}

function mergeRuleSettings(settings: AgentWorkflowSettings, rule: AgentWorkflowRule | null): AgentWorkflowSettings {
  if (!rule) return settings;
  return {
    ...settings,
    defaultAgent: rule.agent,
    mode: rule.mode,
    enabledActions: rule.enabledActions,
    budgetPreset: rule.budgetPreset,
    promptTemplateId: rule.promptTemplateId,
    consent: rule.consent,
  };
}

function readinessBlockReason(
  settings: AgentWorkflowSettings,
  status: McpStatus | null,
  agentStatuses: AgentSetupStatus[]
): string | null {
  if (settings.mode === 'off') return 'Post-meeting workflows are disabled.';
  if (!settings.skillPackInstalled) return 'Meetily skill pack is not installed.';
  if (settings.defaultAgent === 'manual') return null;

  const selectedAgent = agentStatuses.find((agent) => agent.agent === settings.defaultAgent);
  if (!status?.settings.enabled || !selectedAgent?.configured) {
    return 'Enable MCP and configure the selected agent before running post-meeting workflows.';
  }
  if (settings.mode === 'auto' && !selectedAgent.working) {
    return 'Auto-trigger requires the selected agent readiness check to be working.';
  }
  return null;
}

export function resolveAgentWorkflowRule(
  context: AgentWorkflowContext,
  status: McpStatus | null,
  agentStatuses: AgentSetupStatus[],
  settings = getAgentWorkflowSettings()
): ResolvedAgentWorkflowRule {
  const matchedRule = settings.rules.find((rule) => rule.enabled && ruleMatches(context, rule)) ?? null;
  const effectiveSettings = mergeRuleSettings(settings, matchedRule);
  const actionLabels = effectiveSettings.enabledActions
    .map((actionId) => AGENT_WORKFLOW_ACTIONS.find((action) => action.id === actionId)?.label)
    .filter(Boolean)
    .join(', ');
  const preview = [
    matchedRule ? `Rule "${matchedRule.name}" matched.` : 'No rule matched; using global defaults.',
    `Agent: ${effectiveSettings.defaultAgent}.`,
    `Mode: ${effectiveSettings.mode}.`,
    `Budget: ${effectiveSettings.budgetPreset}.`,
    `Template: ${getAgentPromptTemplate(effectiveSettings.promptTemplateId).label}.`,
    `Actions: ${actionLabels || 'Review summary'}.`,
  ].join(' ');

  return {
    settings: effectiveSettings,
    matchedRule,
    preview,
    blockedReason: readinessBlockReason(effectiveSettings, status, agentStatuses),
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

function buildSerializedContextPackage(context: AgentWorkflowContext): string {
  const contextPackage = buildAgentContextPackage({
    meetingId: context.meetingId,
    meetingTitle: context.meetingTitle,
    meetingStartedAt: context.meetingStartedAt,
    meetingEndedAt: context.meetingEndedAt,
    summaryText: summaryToText(context.summary),
    actionItems: context.actionItems,
    decisions: context.decisions,
    risks: context.risks,
    transcriptExcerpts: context.transcriptExcerpts,
    screenshotsOcr: context.screenshotsOcr,
    calendarMetadata: context.calendarMetadata,
    artifacts: context.artifacts,
  }, {
    budgetPreset: context.budgetPreset ?? 'standard',
    customBudget: context.customBudget,
    consent: context.consent,
  });
  return serializeAgentContextPackage(contextPackage, context.mcpUrl ? 'mcp' : 'prompt');
}

function actionInstruction(actionId: WorkflowActionId): string {
  switch (actionId) {
    case 'review-summary':
      return 'Review the summary for missing decisions, unclear risks, weak action items, and unsupported claims.';
    case 'extract-follow-ups':
      return 'Extract follow-ups with owner, next step, due date if implied, source evidence, and confidence.';
    case 'draft-linear-issues':
      return 'Draft Linear issue proposals only. Include title, description, owner if known, priority suggestion, source meeting reference, and confidence.';
    case 'draft-follow-up-message':
      return 'Draft a concise follow-up message suitable for email or chat, with decisions, actions, and any questions needing confirmation.';
    case 'daily-digest':
      return 'Create a daily digest covering meetings, commitments I made, commitments others made, decisions, risks, and open questions.';
    case 'weekly-digest':
      return 'Create a weekly digest grouped by repeated themes, progress, decisions, commitments, risks, and recommended next actions.';
    case 'next-meeting-prep':
      return 'Prepare a next-meeting brief with prior decisions, unresolved questions, promised follow-ups, likely agenda, and suggested questions.';
    case 'open-loops':
      return 'Identify unresolved questions, ownerless actions, risks without mitigation, and decisions that need confirmation.';
    case 'project-status-update':
      return 'Draft a stakeholder-ready project status update with progress, blockers, risks, decisions, owners, and next milestones.';
    case 'decision-log':
      return 'Extract decision-log entries with decision, rationale, alternatives/tradeoffs if available, owner, date, source, and confidence.';
    case 'role-brief':
      return 'Prepare a role-specific brief. Ask which role if not specified: product, engineering, sales, hiring, manager, founder, or customer success.';
    case 'open-agent-handoff':
      return 'Prepare a clean handoff prompt for the selected agent, preserving meeting ids and source references.';
    default:
      return 'Review the meeting and produce concise, source-backed follow-up.';
  }
}

export function buildLinearFollowUpTemplate(
  context: AgentWorkflowContext,
  actions: WorkflowActionId[],
  agent: AgentTarget = 'manual',
  templateId: AgentPromptTemplateId = defaultSettings.promptTemplateId,
  promptTemplateOverrides: Record<string, string> = {}
): string {
  const actionLabels = actions
    .map((actionId) => AGENT_WORKFLOW_ACTIONS.find((action) => action.id === actionId)?.label)
    .filter(Boolean)
    .join(', ');
  const actionInstructions = actions
    .map((actionId) => `- ${actionInstruction(actionId)}`)
    .join('\n');

  return renderAgentPromptTemplate(templateId, {
    meetingTitle: context.meetingTitle,
    meetingId: context.meetingId,
    mcpUrl: context.mcpUrl,
    agent,
    actions,
    actionInstructions: actionInstructions || '- Review the meeting summary and produce concise follow-up.',
    contextPackage: [
      `Meeting: ${context.meetingTitle}`,
      `Meeting ID: ${context.meetingId}`,
      context.mcpUrl ? `Local MCP endpoint: ${context.mcpUrl}` : 'Local MCP endpoint: not enabled',
      `Requested workflows: ${actionLabels || 'Review summary'}`,
      '',
      'Source-cited context package:',
      buildSerializedContextPackage(context),
    ].join('\n'),
  }, promptTemplateOverrides);
}

export function prepareAgentWorkflow(
  context: AgentWorkflowContext,
  status: McpStatus | null,
  agentStatuses: AgentSetupStatus[],
  settings = getAgentWorkflowSettings()
): PreparedAgentWorkflow {
  const createdAt = new Date().toISOString();
  const resolution = resolveAgentWorkflowRule(context, status, agentStatuses, settings);
  const effectiveSettings = resolution.settings;
  const effectiveContext: AgentWorkflowContext = {
    ...context,
    budgetPreset: context.budgetPreset ?? effectiveSettings.budgetPreset,
    consent: {
      ...effectiveSettings.consent,
      ...(context.consent ?? {}),
    },
  };
  const baseRun = {
    id: `workflow-${Date.now()}`,
    meetingId: context.meetingId,
    meetingTitle: context.meetingTitle,
    agent: effectiveSettings.defaultAgent,
    actions: effectiveSettings.enabledActions,
    mode: effectiveSettings.mode,
    createdAt,
  };

  if (resolution.blockedReason) {
    const run: AgentWorkflowRun = {
      ...baseRun,
      status: effectiveSettings.mode === 'off' || !effectiveSettings.skillPackInstalled ? 'skipped' : 'failed',
      message: resolution.blockedReason,
    };
    saveRun(run);
    return { run, prompt: '', canRun: false, reason: run.message };
  }

  const prompt = buildLinearFollowUpTemplate(
    effectiveContext,
    effectiveSettings.enabledActions,
    effectiveSettings.defaultAgent,
    effectiveSettings.promptTemplateId,
    effectiveSettings.promptTemplateOverrides
  );
  const run: AgentWorkflowRun = {
    ...baseRun,
    status: effectiveSettings.mode === 'ask' ? 'waitingForApproval' : 'prepared',
    message: effectiveSettings.mode === 'ask'
      ? `${resolution.preview} Workflow prepared and waiting for user approval.`
      : `${resolution.preview} Workflow handoff prepared automatically.`,
  };
  saveRun(run);
  return { run, prompt, canRun: true, reason: null };
}
