"use client";

import { ChangeEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Copy,
  Download,
  FileInput,
  FileText,
  Plus,
  RotateCcw,
  Save,
  Trash2,
  Wand2,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  ALLOWED_TEMPLATE_VARIABLES,
  SummaryTemplate,
  SummaryTemplateInfo,
  createBlankSummaryTemplate,
  deleteSummaryTemplate,
  duplicateSummaryTemplate,
  exportSummaryTemplates,
  getSummaryTemplate,
  importSummaryTemplates,
  listSummaryTemplates,
  normalizeTemplateId,
  notifySummaryTemplatesChanged,
  renderTemplatePreview,
  restoreDefaultSummaryTemplates,
  saveSummaryTemplate,
  validateSummaryTemplate,
} from "@/services/summaryTemplateService";

function fieldClass() {
  return "space-y-2";
}

function labelClass() {
  return "text-sm font-medium text-slate-900";
}

export function SummaryTemplateSettings() {
  const [templates, setTemplates] = useState<SummaryTemplateInfo[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedSource, setSelectedSource] = useState<SummaryTemplateInfo["source"]>("custom");
  const [draftId, setDraftId] = useState("");
  const [draft, setDraft] = useState<SummaryTemplate>(() => createBlankSummaryTemplate());
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const selectedTemplate = templates.find((template) => template.id === selectedId);
  const preview = useMemo(() => renderTemplatePreview(draft), [draft]);
  const canEdit = selectedSource === "custom" || !selectedId;

  const refreshTemplates = useCallback(async (preferredId?: string) => {
    const nextTemplates = await listSummaryTemplates();
    setTemplates(nextTemplates);

    const nextId = preferredId && nextTemplates.some((template) => template.id === preferredId)
      ? preferredId
      : nextTemplates[0]?.id ?? null;

    setSelectedId(nextId);
    return nextId;
  }, []);

  const loadTemplate = useCallback(async (templateId: string) => {
    setIsLoading(true);
    try {
      const record = await getSummaryTemplate(templateId);
      setSelectedSource(record.source);
      setDraftId(record.id);
      setDraft({
        ...record.template,
        id: record.id,
        variables: record.template.variables?.length ? record.template.variables : ["meeting_title", "transcript"],
        sections: record.template.sections.map((section) => ({
          ...section,
          item_format: section.item_format ?? null,
          example_item_format: section.example_item_format ?? null,
        })),
      });
    } catch (error) {
      toast.error("Failed to load template", { description: String(error) });
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshTemplates().catch((error) => {
      toast.error("Failed to load summary templates", { description: String(error) });
    });
  }, [refreshTemplates]);

  useEffect(() => {
    if (selectedId) {
      loadTemplate(selectedId);
    }
  }, [loadTemplate, selectedId]);

  const updateDraft = (patch: Partial<SummaryTemplate>) => {
    setDraft((current) => ({ ...current, ...patch }));
  };

  const handleNew = () => {
    const id = normalizeTemplateId(`custom_template_${Date.now()}`);
    setSelectedId(null);
    setSelectedSource("custom");
    setDraftId(id);
    setDraft(createBlankSummaryTemplate(id));
  };

  const handleSave = async () => {
    const id = normalizeTemplateId(draftId || draft.name);
    if (!id) {
      toast.error("Template ID is required");
      return;
    }

    setIsSaving(true);
    try {
      const templateToSave = { ...draft, id, schema_version: 1 };
      await validateSummaryTemplate(templateToSave);
      await saveSummaryTemplate(id, templateToSave);
      toast.success("Template saved");
      notifySummaryTemplatesChanged();
      await refreshTemplates(id);
    } catch (error) {
      toast.error("Template could not be saved", { description: String(error) });
    } finally {
      setIsSaving(false);
    }
  };

  const handleDuplicate = async () => {
    if (!selectedId) return;

    const baseName = draft.name.trim() || selectedTemplate?.name || "Template";
    const newId = normalizeTemplateId(`${selectedId}_copy_${Date.now()}`);

    try {
      await duplicateSummaryTemplate(selectedId, newId, `${baseName} copy`);
      toast.success("Template duplicated");
      notifySummaryTemplatesChanged();
      await refreshTemplates(newId);
    } catch (error) {
      toast.error("Template could not be duplicated", { description: String(error) });
    }
  };

  const handleDelete = async () => {
    if (!selectedId || selectedSource !== "custom") return;
    if (!window.confirm(`Delete "${draft.name || selectedId}"? This cannot be undone.`)) return;

    try {
      await deleteSummaryTemplate(selectedId);
      toast.success("Template deleted");
      notifySummaryTemplatesChanged();
      await refreshTemplates();
    } catch (error) {
      toast.error("Template could not be deleted", { description: String(error) });
    }
  };

  const handleExport = async () => {
    try {
      const bundle = await exportSummaryTemplates();
      const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = "meetily-summary-templates.json";
      link.click();
      URL.revokeObjectURL(url);
      toast.success("Custom templates exported");
    } catch (error) {
      toast.error("Templates could not be exported", { description: String(error) });
    }
  };

  const handleImportFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;

    try {
      const bundleJson = await file.text();
      const imported = await importSummaryTemplates(bundleJson);
      toast.success(`Imported ${imported.length} template${imported.length === 1 ? "" : "s"}`);
      notifySummaryTemplatesChanged();
      await refreshTemplates(imported[0]?.id);
    } catch (error) {
      toast.error("Templates could not be imported", { description: String(error) });
    }
  };

  const handleRestoreDefaults = async () => {
    try {
      const restored = await restoreDefaultSummaryTemplates();
      toast.success(`Restored ${restored.length} editable copies`);
      notifySummaryTemplatesChanged();
      await refreshTemplates(restored[0]?.id);
    } catch (error) {
      toast.error("Defaults could not be restored", { description: String(error) });
    }
  };

  return (
    <div className="mt-6 grid gap-6 lg:grid-cols-[320px_minmax(0,1fr)]">
      <section className="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-lg font-semibold text-slate-950">Templates</h3>
            <p className="mt-2 text-sm leading-6 text-slate-600">
              Choose the structure used when Meetily generates or regenerates a meeting summary.
            </p>
          </div>
          <Button size="icon" variant="outline" onClick={handleNew} title="Create template">
            <Plus className="h-4 w-4" />
          </Button>
        </div>

        <div className="mt-5 space-y-2">
          {templates.map((template) => (
            <button
              key={template.id}
              type="button"
              onClick={() => setSelectedId(template.id)}
              className={`w-full rounded-lg border px-3 py-3 text-left transition ${
                selectedId === template.id
                  ? "border-emerald-300 bg-emerald-50"
                  : "border-slate-200 bg-white hover:border-slate-300 hover:bg-slate-50"
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <span className="text-sm font-semibold text-slate-950">{template.name}</span>
                <span className="shrink-0 rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-medium text-slate-600">
                  {template.source === "custom" ? "Custom" : "Built-in"}
                </span>
              </div>
              <p className="mt-1 line-clamp-2 text-xs leading-5 text-slate-600">{template.description}</p>
            </button>
          ))}
        </div>

        <div className="mt-5 grid grid-cols-2 gap-2">
          <Button variant="outline" onClick={handleExport}>
            <Download className="h-4 w-4" />
            Export
          </Button>
          <Button variant="outline" onClick={() => fileInputRef.current?.click()}>
            <FileInput className="h-4 w-4" />
            Import
          </Button>
          <Button variant="outline" onClick={handleRestoreDefaults} className="col-span-2">
            <RotateCcw className="h-4 w-4" />
            Restore editable defaults
          </Button>
        </div>
        <input ref={fileInputRef} type="file" accept="application/json,.json" className="hidden" onChange={handleImportFile} />
      </section>

      <section className="rounded-lg border border-slate-200 bg-white p-5 shadow-sm">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h3 className="text-lg font-semibold text-slate-950">Template editor</h3>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-600">
              Edit custom templates directly. Built-in templates are protected; duplicate one first to make an editable copy.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" onClick={handleDuplicate} disabled={!selectedId}>
              <Copy className="h-4 w-4" />
              Duplicate
            </Button>
            <Button variant="destructive" onClick={handleDelete} disabled={selectedSource !== "custom" || !selectedId}>
              <Trash2 className="h-4 w-4" />
              Delete
            </Button>
            <Button onClick={handleSave} disabled={!canEdit || isSaving || isLoading}>
              <Save className="h-4 w-4" />
              Save
            </Button>
          </div>
        </div>

        {!canEdit && (
          <div className="mt-5 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
            This is a built-in template. Use Duplicate to create an editable custom copy.
          </div>
        )}

        <div className="mt-6 grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
          <div className="space-y-5">
            <div className="grid gap-4 md:grid-cols-2">
              <div className={fieldClass()}>
                <label className={labelClass()} htmlFor="summary-template-id">Template ID</label>
                <Input
                  id="summary-template-id"
                  value={draftId}
                  disabled={!canEdit}
                  onChange={(event) => setDraftId(normalizeTemplateId(event.target.value))}
                  placeholder="project_sync_custom"
                />
              </div>
              <div className={fieldClass()}>
                <label className={labelClass()} htmlFor="summary-template-name">Name</label>
                <Input
                  id="summary-template-name"
                  value={draft.name}
                  disabled={!canEdit}
                  onChange={(event) => updateDraft({ name: event.target.value })}
                  placeholder="Engineering sync"
                />
              </div>
            </div>

            <div className={fieldClass()}>
              <label className={labelClass()} htmlFor="summary-template-description">Description</label>
              <Textarea
                id="summary-template-description"
                value={draft.description}
                disabled={!canEdit}
                onChange={(event) => updateDraft({ description: event.target.value })}
                placeholder="When to use this template"
              />
            </div>

            <div className={fieldClass()}>
              <label className={labelClass()}>Available variables</label>
              <div className="flex flex-wrap gap-2">
                {ALLOWED_TEMPLATE_VARIABLES.map((variable) => {
                  const selected = draft.variables.includes(variable);
                  return (
                    <button
                      key={variable}
                      type="button"
                      disabled={!canEdit}
                      onClick={() => updateDraft({
                        variables: selected
                          ? draft.variables.filter((item) => item !== variable)
                          : [...draft.variables, variable],
                      })}
                      className={`rounded-full border px-3 py-1 text-xs font-medium transition ${
                        selected ? "border-emerald-300 bg-emerald-50 text-emerald-800" : "border-slate-200 text-slate-600"
                      }`}
                    >
                      {"{{"}{variable}{"}}"}
                    </button>
                  );
                })}
              </div>
            </div>

            <div className={fieldClass()}>
              <label className={labelClass()} htmlFor="summary-template-guidance">Template guidance</label>
              <Textarea
                id="summary-template-guidance"
                value={draft.custom_instructions ?? ""}
                disabled={!canEdit}
                onChange={(event) => updateDraft({ custom_instructions: event.target.value })}
                placeholder="High-level rules the model should follow for this template"
              />
            </div>

            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-semibold text-slate-950">Sections</h4>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!canEdit}
                  onClick={() => updateDraft({
                    sections: [
                      ...draft.sections,
                      { title: "New section", instruction: "", format: "paragraph", item_format: null, example_item_format: null },
                    ],
                  })}
                >
                  <Plus className="h-4 w-4" />
                  Add section
                </Button>
              </div>

              {draft.sections.map((section, index) => (
                <div key={`${section.title}-${index}`} className="rounded-lg border border-slate-200 p-4">
                  <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_140px_auto]">
                    <Input
                      value={section.title}
                      disabled={!canEdit}
                      onChange={(event) => updateDraft({
                        sections: draft.sections.map((item, itemIndex) => itemIndex === index ? { ...item, title: event.target.value } : item),
                      })}
                      placeholder="Section title"
                    />
                    <select
                      value={section.format}
                      disabled={!canEdit}
                      onChange={(event) => updateDraft({
                        sections: draft.sections.map((item, itemIndex) => itemIndex === index ? { ...item, format: event.target.value as SummaryTemplate["sections"][number]["format"] } : item),
                      })}
                      className="h-10 rounded-xl border border-slate-200 bg-white px-3 text-sm text-slate-950"
                    >
                      <option value="paragraph">Paragraph</option>
                      <option value="list">List</option>
                      <option value="string">String</option>
                    </select>
                    <Button
                      variant="ghost"
                      size="icon"
                      disabled={!canEdit || draft.sections.length === 1}
                      onClick={() => updateDraft({ sections: draft.sections.filter((_, itemIndex) => itemIndex !== index) })}
                      title="Remove section"
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                  <Textarea
                    className="mt-3"
                    value={section.instruction}
                    disabled={!canEdit}
                    onChange={(event) => updateDraft({
                      sections: draft.sections.map((item, itemIndex) => itemIndex === index ? { ...item, instruction: event.target.value } : item),
                    })}
                    placeholder="Tell the model what to extract for this section"
                  />
                  <Input
                    className="mt-3"
                    value={section.item_format ?? ""}
                    disabled={!canEdit}
                    onChange={(event) => updateDraft({
                      sections: draft.sections.map((item, itemIndex) => itemIndex === index ? { ...item, item_format: event.target.value || null } : item),
                    })}
                    placeholder="Optional item format, for example Owner | Task | Due"
                  />
                </div>
              ))}
            </div>
          </div>

          <aside className="rounded-lg border border-slate-200 bg-slate-50 p-4">
            <div className="flex items-center gap-2 text-sm font-semibold text-slate-950">
              <FileText className="h-4 w-4" />
              Preview
            </div>
            <p className="mt-2 text-xs leading-5 text-slate-600">
              This preview shows the markdown shape the summary should follow before the model fills it with meeting content.
            </p>
            <pre className="mt-4 max-h-[560px] overflow-auto whitespace-pre-wrap rounded-lg bg-white p-4 text-xs leading-5 text-slate-800 ring-1 ring-slate-200">
              {preview}
            </pre>
            <Button
              variant="outline"
              className="mt-4 w-full"
              onClick={() => validateSummaryTemplate({ ...draft, id: draftId || null }).then(
                (name) => toast.success("Template is valid", { description: name }),
                (error) => toast.error("Template is invalid", { description: String(error) }),
              )}
            >
              <Wand2 className="h-4 w-4" />
              Validate template
            </Button>
          </aside>
        </div>
      </section>
    </div>
  );
}
