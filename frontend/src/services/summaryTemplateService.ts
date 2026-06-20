import { invoke } from '@tauri-apps/api/core';

export const SUMMARY_TEMPLATES_CHANGED_EVENT = 'meetily:summary-templates-changed';

export const ALLOWED_TEMPLATE_VARIABLES = [
  'meeting_title',
  'transcript',
  'participants',
  'date',
  'action_items',
  'custom_instructions',
] as const;

export interface SummaryTemplateSection {
  title: string;
  instruction: string;
  format: 'paragraph' | 'list' | 'string';
  item_format?: string | null;
  example_item_format?: string | null;
}

export interface SummaryTemplate {
  id?: string | null;
  schema_version: number;
  name: string;
  description: string;
  variables: string[];
  custom_instructions?: string | null;
  sections: SummaryTemplateSection[];
}

export interface SummaryTemplateInfo {
  id: string;
  name: string;
  description: string;
  source: 'builtIn' | 'custom';
}

export interface SummaryTemplateRecord {
  id: string;
  source: 'builtIn' | 'custom';
  template: SummaryTemplate;
}

export interface SummaryTemplateExportBundle {
  exported_at?: string;
  templates: SummaryTemplateRecord[];
}

export function createBlankSummaryTemplate(id?: string): SummaryTemplate {
  return {
    id: id ?? null,
    schema_version: 1,
    name: '',
    description: '',
    variables: ['meeting_title', 'transcript', 'custom_instructions'],
    custom_instructions: '',
    sections: [
      {
        title: 'Summary',
        instruction: 'Summarize the most important context, decisions, and outcomes.',
        format: 'paragraph',
        item_format: null,
        example_item_format: null,
      },
      {
        title: 'Action Items',
        instruction: 'List follow-ups with owner and due date when available.',
        format: 'list',
        item_format: 'Owner | Task | Due | Evidence',
        example_item_format: null,
      },
    ],
  };
}

export function normalizeTemplateId(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_\-\s]/g, '')
    .replace(/\s+/g, '_')
    .slice(0, 80);
}

export function renderTemplatePreview(template: SummaryTemplate): string {
  const title = template.name.trim() || 'Untitled template';
  const guidance = template.custom_instructions?.trim();
  const lines = [`# ${title}`, ''];

  if (guidance) {
    lines.push(`> ${guidance}`, '');
  }

  template.sections.forEach((section) => {
    const heading = section.title.trim() || 'Untitled section';
    lines.push(`## ${heading}`, '');

    if (section.format === 'list') {
      const itemFormat = section.item_format?.trim() || section.example_item_format?.trim();
      lines.push(`- ${itemFormat || 'Item with relevant details'}`, '');
      return;
    }

    lines.push(section.instruction.trim() || 'Generated content appears here.', '');
  });

  return lines.join('\n').trim();
}

export async function listSummaryTemplates() {
  return invoke<SummaryTemplateInfo[]>('api_list_templates');
}

export async function getSummaryTemplate(templateId: string) {
  return invoke<SummaryTemplateRecord>('api_get_template', { templateId });
}

export async function saveSummaryTemplate(id: string, template: SummaryTemplate) {
  return invoke<SummaryTemplateRecord>('api_save_custom_template', {
    request: {
      id,
      template: {
        ...template,
        id,
        schema_version: template.schema_version || 1,
      },
    },
  });
}

export async function deleteSummaryTemplate(templateId: string) {
  return invoke<void>('api_delete_custom_template', { templateId });
}

export async function duplicateSummaryTemplate(templateId: string, newTemplateId: string, newName: string) {
  return invoke<SummaryTemplateRecord>('api_duplicate_template', {
    templateId,
    newTemplateId,
    newName,
  });
}

export async function exportSummaryTemplates() {
  return invoke<SummaryTemplateExportBundle>('api_export_custom_templates');
}

export async function importSummaryTemplates(bundleJson: string) {
  return invoke<SummaryTemplateRecord[]>('api_import_custom_templates', { bundleJson });
}

export async function restoreDefaultSummaryTemplates() {
  return invoke<SummaryTemplateRecord[]>('api_restore_default_templates');
}

export async function validateSummaryTemplate(template: SummaryTemplate) {
  return invoke<string>('api_validate_template', { templateJson: JSON.stringify(template) });
}

export function notifySummaryTemplatesChanged() {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(SUMMARY_TEMPLATES_CHANGED_EVENT));
  }
}
