export type AgentContextBudgetPreset = 'minimal' | 'standard' | 'detailed' | 'custom';
export type AgentContextTarget = 'prompt' | 'codex' | 'claude' | 'mcp';
export type AgentContextSourceType =
  | 'summary'
  | 'action_item'
  | 'decision'
  | 'risk'
  | 'transcript'
  | 'screenshot_ocr'
  | 'calendar'
  | 'artifact';

export interface AgentContextBudget {
  preset: AgentContextBudgetPreset;
  maxCharacters: number;
  maxTranscriptExcerpts: number;
}

export interface AgentContextConsent {
  includeSummary?: boolean;
  includeActionItems?: boolean;
  includeDecisions?: boolean;
  includeRisks?: boolean;
  includeTranscriptExcerpts?: boolean;
  includeFullTranscript?: boolean;
  includeScreenshotsOcr?: boolean;
  includeCalendarMetadata?: boolean;
  includeArtifacts?: boolean;
}

export interface AgentMeetingContextInput {
  meetingId: string;
  meetingTitle: string;
  meetingStartedAt?: string | null;
  meetingEndedAt?: string | null;
  summaryText?: string | null;
  actionItems?: AgentContextInputItem[];
  decisions?: AgentContextInputItem[];
  risks?: AgentContextInputItem[];
  transcriptExcerpts?: AgentContextInputItem[];
  screenshotsOcr?: AgentContextInputItem[];
  calendarMetadata?: AgentContextInputItem[];
  artifacts?: AgentContextInputItem[];
}

export interface AgentContextInputItem {
  id?: string;
  text: string;
  sourceLabel?: string;
  timestamp?: string | null;
  startsAt?: string | null;
  endsAt?: string | null;
  confidence?: number | null;
  metadata?: Record<string, string | number | boolean | null | undefined>;
}

export interface AgentContextCitation {
  id: string;
  sourceType: AgentContextSourceType;
  meetingId: string;
  sourceId?: string;
  sourceLabel?: string;
  timestamp?: string | null;
  startsAt?: string | null;
  endsAt?: string | null;
  confidence?: number | null;
  metadata?: Record<string, string | number | boolean | null | undefined>;
}

export interface AgentContextSection {
  id: string;
  title: string;
  sourceType: AgentContextSourceType;
  content: string;
  citations: AgentContextCitation[];
  truncated: boolean;
}

export interface AgentContextPackage {
  schemaVersion: '2026-06-20';
  meeting: {
    id: string;
    title: string;
    startedAt: string | null;
    endedAt: string | null;
  };
  budget: AgentContextBudget;
  sections: AgentContextSection[];
  redactions: string[];
  omittedSources: Array<{
    sourceType: AgentContextSourceType;
    reason: string;
    count: number;
  }>;
  generatedAt: string;
}

const DEFAULT_CONSENT: Required<AgentContextConsent> = {
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

const BUDGET_PRESETS: Record<AgentContextBudgetPreset, AgentContextBudget> = {
  minimal: { preset: 'minimal', maxCharacters: 1600, maxTranscriptExcerpts: 1 },
  standard: { preset: 'standard', maxCharacters: 4800, maxTranscriptExcerpts: 4 },
  detailed: { preset: 'detailed', maxCharacters: 12000, maxTranscriptExcerpts: 12 },
  custom: { preset: 'custom', maxCharacters: 4800, maxTranscriptExcerpts: 4 },
};

export function resolveAgentContextBudget(
  preset: AgentContextBudgetPreset = 'standard',
  custom?: Partial<Omit<AgentContextBudget, 'preset'>>
): AgentContextBudget {
  const base = BUDGET_PRESETS[preset] ?? BUDGET_PRESETS.standard;
  if (preset !== 'custom') return { ...base };
  return {
    preset,
    maxCharacters: clampNumber(custom?.maxCharacters, 800, 24000, base.maxCharacters),
    maxTranscriptExcerpts: clampNumber(custom?.maxTranscriptExcerpts, 0, 30, base.maxTranscriptExcerpts),
  };
}

export function buildAgentContextPackage(
  input: AgentMeetingContextInput,
  options?: {
    budgetPreset?: AgentContextBudgetPreset;
    customBudget?: Partial<Omit<AgentContextBudget, 'preset'>>;
    consent?: AgentContextConsent;
    generatedAt?: string;
  }
): AgentContextPackage {
  const consent = { ...DEFAULT_CONSENT, ...(options?.consent ?? {}) };
  const budget = resolveAgentContextBudget(options?.budgetPreset ?? 'standard', options?.customBudget);
  const sections: AgentContextSection[] = [];
  const omittedSources: AgentContextPackage['omittedSources'] = [];
  const redactions: string[] = [];
  let remainingCharacters = budget.maxCharacters;

  const addSection = (
    sourceType: AgentContextSourceType,
    title: string,
    items: AgentContextInputItem[],
    allowed: boolean,
    deniedReason: string,
    maxItems?: number
  ) => {
    const normalizedItems = items
      .map((item) => ({ ...item, text: normalizeText(item.text) }))
      .filter((item) => item.text.length > 0);
    if (normalizedItems.length === 0) return;
    if (!allowed) {
      omittedSources.push({ sourceType, reason: deniedReason, count: normalizedItems.length });
      redactions.push(`${title} omitted: ${deniedReason}`);
      return;
    }
    if (remainingCharacters <= 0) {
      omittedSources.push({ sourceType, reason: 'context budget exhausted', count: normalizedItems.length });
      return;
    }

    const takeItems = typeof maxItems === 'number' ? normalizedItems.slice(0, maxItems) : normalizedItems;
    const citations: AgentContextCitation[] = [];
    const lines: string[] = [];
    let truncated = takeItems.length < normalizedItems.length;

    takeItems.forEach((item, index) => {
      if (remainingCharacters <= 0) {
        truncated = true;
        return;
      }
      const citation = citationFor(input.meetingId, sourceType, item, `${sourceType}-${sections.length + 1}-${index + 1}`);
      const prefix = `[${citation.id}]`;
      const available = Math.max(0, remainingCharacters - prefix.length - 3);
      const clipped = clipText(item.text, available);
      if (!clipped.text) {
        truncated = true;
        return;
      }
      citations.push(citation);
      lines.push(`${prefix} ${clipped.text}`);
      remainingCharacters -= prefix.length + 1 + clipped.text.length + 1;
      if (clipped.truncated) truncated = true;
    });

    if (lines.length === 0) return;
    sections.push({
      id: sourceType,
      title,
      sourceType,
      content: lines.join('\n'),
      citations,
      truncated,
    });

    const omittedCount = normalizedItems.length - citations.length;
    if (omittedCount > 0) {
      omittedSources.push({
        sourceType,
        reason: truncated ? 'context budget or item limit' : deniedReason,
        count: omittedCount,
      });
    }
  };

  addSection(
    'summary',
    'Summary',
    input.summaryText ? [{ text: input.summaryText, sourceLabel: 'meeting summary' }] : [],
    consent.includeSummary,
    'summary context is not approved'
  );
  addSection('decision', 'Decisions', input.decisions ?? [], consent.includeDecisions, 'decision context is not approved');
  addSection('action_item', 'Action Items', input.actionItems ?? [], consent.includeActionItems, 'action item context is not approved');
  addSection('risk', 'Risks', input.risks ?? [], consent.includeRisks, 'risk context is not approved');
  addSection(
    'transcript',
    consent.includeFullTranscript ? 'Transcript' : 'Transcript Excerpts',
    input.transcriptExcerpts ?? [],
    consent.includeTranscriptExcerpts || consent.includeFullTranscript,
    'transcript excerpts require explicit approval',
    consent.includeFullTranscript ? undefined : budget.maxTranscriptExcerpts
  );
  addSection('screenshot_ocr', 'Screenshot OCR', input.screenshotsOcr ?? [], consent.includeScreenshotsOcr, 'screenshot OCR requires explicit approval');
  addSection('calendar', 'Calendar Metadata', input.calendarMetadata ?? [], consent.includeCalendarMetadata, 'calendar metadata requires explicit approval');
  addSection('artifact', 'Artifact Links', input.artifacts ?? [], consent.includeArtifacts, 'artifact links require explicit approval');

  return {
    schemaVersion: '2026-06-20',
    meeting: {
      id: input.meetingId,
      title: input.meetingTitle,
      startedAt: input.meetingStartedAt ?? null,
      endedAt: input.meetingEndedAt ?? null,
    },
    budget,
    sections,
    redactions,
    omittedSources,
    generatedAt: options?.generatedAt ?? new Date().toISOString(),
  };
}

export function serializeAgentContextPackage(pkg: AgentContextPackage, target: AgentContextTarget = 'prompt'): string {
  const header = [
    `Meetily context package (${pkg.schemaVersion})`,
    `Target: ${target}`,
    `Meeting: ${pkg.meeting.title}`,
    `Meeting ID: ${pkg.meeting.id}`,
    pkg.meeting.startedAt ? `Started: ${pkg.meeting.startedAt}` : null,
    `Budget: ${pkg.budget.preset} (${pkg.budget.maxCharacters} chars)`,
  ].filter(Boolean);

  const sections = pkg.sections.map((section) => [
    `## ${section.title}`,
    section.content,
    section.truncated ? '_Section truncated by context budget._' : null,
  ].filter(Boolean).join('\n'));

  const citations = pkg.sections
    .flatMap((section) => section.citations)
    .map((citation) => {
      const timing = citation.startsAt || citation.timestamp
        ? ` @ ${citation.startsAt ?? citation.timestamp}${citation.endsAt ? `-${citation.endsAt}` : ''}`
        : '';
      return `- [${citation.id}] ${citation.sourceType}${timing}${citation.sourceLabel ? ` (${citation.sourceLabel})` : ''}`;
    });

  const omitted = pkg.omittedSources.map((source) => `- ${source.sourceType}: ${source.count} omitted (${source.reason})`);

  return [
    ...header,
    '',
    ...sections,
    citations.length > 0 ? '## Sources' : null,
    citations.length > 0 ? citations.join('\n') : null,
    omitted.length > 0 ? '## Omitted or redacted' : null,
    omitted.length > 0 ? omitted.join('\n') : null,
  ].filter(Boolean).join('\n');
}

function citationFor(meetingId: string, sourceType: AgentContextSourceType, item: AgentContextInputItem, fallbackId: string): AgentContextCitation {
  return {
    id: item.id || fallbackId,
    sourceType,
    meetingId,
    sourceId: item.id,
    sourceLabel: item.sourceLabel,
    timestamp: item.timestamp ?? null,
    startsAt: item.startsAt ?? null,
    endsAt: item.endsAt ?? null,
    confidence: item.confidence ?? null,
    metadata: item.metadata,
  };
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, ' ').trim();
}

function clipText(value: string, maxCharacters: number): { text: string; truncated: boolean } {
  if (maxCharacters <= 0) return { text: '', truncated: value.length > 0 };
  if (value.length <= maxCharacters) return { text: value, truncated: false };
  return { text: `${value.slice(0, Math.max(0, maxCharacters - 1)).trimEnd()}…`, truncated: true };
}

function clampNumber(value: number | undefined, min: number, max: number, fallback: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return fallback;
  return Math.min(max, Math.max(min, Math.floor(value)));
}
