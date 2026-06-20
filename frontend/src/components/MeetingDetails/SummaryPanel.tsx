"use client";

import { Summary, SummaryResponse, Transcript } from '@/types';
import { EditableTitle } from '@/components/EditableTitle';
import { BlockNoteSummaryView, BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { EmptyStateSummary } from '@/components/EmptyStateSummary';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { SummaryGeneratorButtonGroup } from './SummaryGeneratorButtonGroup';
import { SummaryUpdaterButtonGroup } from './SummaryUpdaterButtonGroup';
import { ExportMeetingDialog } from './ExportMeetingDialog';
import { ReminderDraftReview } from './ReminderDraftReview';
import { AppleNotesExportPanel } from './AppleNotesExportPanel';
import { CalendarEventPanel } from './CalendarEventPanel';
import { AgentWorkflowRunsPanel } from '@/components/AgentWorkflowRunsPanel';
import { RecordingAuditTrail } from '@/components/RecordingAuditTrail';
import { PostRecordingReviewChecklist } from './PostRecordingReviewChecklist';
import { MeetingChatPanel } from './MeetingChatPanel';
import { MeetingChatCitation } from '@/services/meetingChatService';
import Analytics from '@/lib/analytics';
import { useEffect, useRef, useState, RefObject } from 'react';
import { toast } from 'sonner';
import { Languages, ChevronDown, FileText, MessageCircle, Bot, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import { LanguagePickerPopover } from '@/components/LanguagePickerPopover';
import { useRecentLanguages } from '@/hooks/useRecentLanguages';
import { labelForCode } from '@/lib/summary-languages';
import {
  readMeetingSummaryLanguage,
  saveMeetingSummaryLanguage,
  SummaryLanguageStorage,
} from '@/lib/summary-language-preferences';
import { triggerMeetingAgentWorkflow } from '@/services/agentWorkflowTriggerService';

interface SummaryPanelProps {
  meeting: {
    id: string;
    title: string;
    created_at: string;
  };
  meetingTitle: string;
  onTitleChange: (title: string) => void;
  isEditingTitle: boolean;
  onStartEditTitle: () => void;
  onFinishEditTitle: () => void;
  isTitleDirty: boolean;
  summaryRef: RefObject<BlockNoteSummaryViewRef>;
  isSaving: boolean;
  onSaveAll: () => Promise<void>;
  onCopySummary: () => Promise<void>;
  onOpenFolder: () => Promise<void>;
  aiSummary: Summary | null;
  summaryStatus: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';
  transcripts: Transcript[];
  modelConfig: ModelConfig;
  setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
  onSaveModelConfig: (config?: ModelConfig) => Promise<void>;
  onGenerateSummary: (customPrompt: string) => Promise<void>;
  onStopGeneration: () => void;
  customPrompt: string;
  summaryResponse: SummaryResponse | null;
  onSaveSummary: (summary: Summary | { markdown?: string; summary_json?: any[] }) => Promise<void>;
  onSummaryChange: (summary: Summary) => void;
  onDirtyChange: (isDirty: boolean) => void;
  summaryError: string | null;
  onRegenerateSummary: () => Promise<void>;
  getSummaryStatusMessage: (status: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error') => string;
  availableTemplates: Array<{ id: string, name: string, description: string }>;
  selectedTemplate: string;
  onTemplateSelect: (templateId: string, templateName: string) => void;
  isModelConfigLoading?: boolean;
  onOpenModelSettings?: (openFn: () => void) => void;
  onTranscriptCitationSelect?: (citation: MeetingChatCitation) => void | Promise<void>;
}

export function SummaryPanel({
  meeting,
  meetingTitle,
  onTitleChange,
  isEditingTitle,
  onStartEditTitle,
  onFinishEditTitle,
  isTitleDirty,
  summaryRef,
  isSaving,
  onSaveAll,
  onCopySummary,
  onOpenFolder,
  aiSummary,
  summaryStatus,
  transcripts,
  modelConfig,
  setModelConfig,
  onSaveModelConfig,
  onGenerateSummary,
  onStopGeneration,
  customPrompt,
  summaryResponse,
  onSaveSummary,
  onSummaryChange,
  onDirtyChange,
  summaryError,
  onRegenerateSummary,
  getSummaryStatusMessage,
  availableTemplates,
  selectedTemplate,
  onTemplateSelect,
  isModelConfigLoading = false,
  onOpenModelSettings,
  onTranscriptCitationSelect
}: SummaryPanelProps) {
  const [summaryLang, setSummaryLang] = useState<string | null>(null);
  const [summaryLangStorage, setSummaryLangStorage] = useState<SummaryLanguageStorage>('metadata');
  const [langPickerOpen, setLangPickerOpen] = useState(false);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [activeView, setActiveView] = useState<'summary' | 'chat'>('summary');
  const [isTriggeringAutomation, setIsTriggeringAutomation] = useState(false);
  const languageLoadVersionRef = useRef(0);
  const activeMeetingIdRef = useRef(meeting.id);
  const languageSaveVersionRef = useRef(0);
  const languageSaveLoopRunningRef = useRef(false);
  const latestLanguageSaveRequestRef = useRef<{
    version: number;
    meetingId: string;
    language: string | null;
    rollback: {
      language: string | null;
      storage: SummaryLanguageStorage;
    };
  } | null>(null);
  activeMeetingIdRef.current = meeting.id;
  const { addRecent } = useRecentLanguages();

  const effectiveLangLabel = summaryLang ? labelForCode(summaryLang) : 'Auto';
  const isLocalFallbackLanguage = summaryLangStorage === 'local_fallback';
  const autoSubtitle = isLocalFallbackLanguage
    ? 'Saved on this device for folderless meetings'
    : 'Uses dominant transcript language';

  useEffect(() => {
    let cancelled = false;
    const loadVersion = languageLoadVersionRef.current + 1;
    languageLoadVersionRef.current = loadVersion;

    const loadSummaryLanguage = async () => {
      try {
        const stored = await readMeetingSummaryLanguage(meeting.id);
        if (!cancelled && languageLoadVersionRef.current === loadVersion) {
          setSummaryLang(stored.language);
          setSummaryLangStorage(stored.storage);
        }
      } catch (err) {
        console.error('Failed to load summary language:', err);
        toast.warning('Could not load saved summary language', {
          description: 'Using Auto until meeting metadata can be read.',
        });
        if (!cancelled && languageLoadVersionRef.current === loadVersion) setSummaryLang(null);
      }
    };

    loadSummaryLanguage();

    return () => {
      cancelled = true;
    };
  }, [meeting.id]);

  const persistLatestLanguageSelection = async () => {
    if (languageSaveLoopRunningRef.current) return;
    languageSaveLoopRunningRef.current = true;

    try {
      while (true) {
        const request = latestLanguageSaveRequestRef.current;
        if (!request) return;

        try {
          const saved = await saveMeetingSummaryLanguage(request.meetingId, request.language);
          const latest = latestLanguageSaveRequestRef.current;
          if (
            latest?.version === request.version &&
            activeMeetingIdRef.current === request.meetingId
          ) {
            setSummaryLang(saved.language);
            setSummaryLangStorage(saved.storage);
            if (saved.storage === 'local_fallback') {
              toast.info('Summary language saved on this device', {
                description: 'This meeting has no recording folder, so the preference cannot be written to meeting metadata.',
              });
            }
            if (request.language) {
              addRecent(request.language);
            }
            return;
          }

          if (latest?.version === request.version) return;
        } catch (err) {
          const latest = latestLanguageSaveRequestRef.current;
          if (
            latest?.version === request.version &&
            activeMeetingIdRef.current === request.meetingId
          ) {
            console.error('Failed to persist summary language:', err);
            toast.error('Failed to save summary language');
            setSummaryLang(request.rollback.language);
            setSummaryLangStorage(request.rollback.storage);
            return;
          }

          console.warn('Ignoring failed stale summary language save:', err);
          if (latest?.version === request.version) return;
        }
      }
    } finally {
      languageSaveLoopRunningRef.current = false;
    }
  };

  const handleLangChange = (code: string | null) => {
    const previous = summaryLang;
    const previousStorage = summaryLangStorage;
    const nextStored = code;
    languageLoadVersionRef.current += 1;
    latestLanguageSaveRequestRef.current = {
      version: languageSaveVersionRef.current + 1,
      meetingId: meeting.id,
      language: nextStored,
      rollback: {
        language: previous,
        storage: previousStorage,
      },
    };
    languageSaveVersionRef.current += 1;
    setSummaryLang(nextStored);
    setLangPickerOpen(false);
    void persistLatestLanguageSelection();
  };

  const isSummaryLoading = summaryStatus === 'processing' || summaryStatus === 'summarizing' || summaryStatus === 'regenerating';

  useEffect(() => {
    setActiveView('summary');
  }, [meeting.id]);

  const handleTriggerAutomation = async (source: 'manual-summary' | 'manual-chat') => {
    if (isTriggeringAutomation) return;
    setIsTriggeringAutomation(true);
    try {
      await triggerMeetingAgentWorkflow({
        meetingId: meeting.id,
        meetingTitle,
        templateId: selectedTemplate,
        summary: aiSummary,
        source,
      });
    } finally {
      setIsTriggeringAutomation(false);
    }
  };

  const languageSlot = (
    <Popover open={langPickerOpen} onOpenChange={setLangPickerOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          title={`Summary language: ${effectiveLangLabel}${isLocalFallbackLanguage ? ' (saved on this device)' : ''}`}
          aria-label="Set summary language"
        >
          <Languages size={18} />
          <span className="hidden lg:inline">{effectiveLangLabel}</span>
          <ChevronDown size={14} className="text-slate-400" />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        className="w-auto p-0 border-0 shadow-none bg-transparent"
      >
        <LanguagePickerPopover
          value={summaryLang}
          onChange={handleLangChange}
          onClose={() => setLangPickerOpen(false)}
          autoSubtitle={autoSubtitle}
        />
      </PopoverContent>
    </Popover>
  );

  return (
    <div className="flex min-w-0 flex-1 flex-col overflow-hidden bg-white/95">
      {/* Title area */}
      <div className="border-b border-slate-200/80 bg-white/90 p-4">
        {/* <EditableTitle
          title={meetingTitle}
          isEditing={isEditingTitle}
          onStartEditing={onStartEditTitle}
          onFinishEditing={onFinishEditTitle}
          onChange={onTitleChange}
        /> */}

        <div className="flex w-full flex-wrap items-center justify-between gap-3">
          <div className="inline-flex rounded-lg border border-slate-200 bg-slate-50 p-1" role="tablist" aria-label="Meeting details view">
            <button
              type="button"
              role="tab"
              id="meeting-summary-tab"
              aria-selected={activeView === 'summary'}
              aria-controls="meeting-summary-panel"
              onClick={() => setActiveView('summary')}
              className={`inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition ${activeView === 'summary'
                ? 'bg-white text-slate-950 shadow-sm'
                : 'text-slate-500 hover:text-slate-800'
                }`}
            >
              <FileText size={16} />
              Summary
            </button>
            <button
              type="button"
              role="tab"
              id="meeting-chat-tab"
              aria-selected={activeView === 'chat'}
              aria-controls="meeting-chat-panel"
              onClick={() => setActiveView('chat')}
              className={`inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition ${activeView === 'chat'
                ? 'bg-white text-slate-950 shadow-sm'
                : 'text-slate-500 hover:text-slate-800'
                }`}
            >
              <MessageCircle size={16} />
              Chat
            </button>
          </div>

          {/* Button groups - only show when summary exists */}
          {activeView === 'summary' && aiSummary && !isSummaryLoading && (
            <div className="flex min-w-0 flex-wrap items-center justify-end gap-2">
              {/* Left-aligned: Summary Generator Button Group */}
              <div className="min-w-0 flex-shrink-0">
                <SummaryGeneratorButtonGroup
                  modelConfig={modelConfig}
                  setModelConfig={setModelConfig}
                  onSaveModelConfig={onSaveModelConfig}
                  onGenerateSummary={onGenerateSummary}
                  onStopGeneration={onStopGeneration}
                  customPrompt={customPrompt}
                  summaryStatus={summaryStatus}
                  availableTemplates={availableTemplates}
                  selectedTemplate={selectedTemplate}
                  onTemplateSelect={onTemplateSelect}
                  hasTranscripts={transcripts.length > 0}
                  hasSummary={!!aiSummary}
                  isModelConfigLoading={isModelConfigLoading}
                  onOpenModelSettings={onOpenModelSettings}
                  languageSlot={languageSlot}
                />
              </div>

              {/* Right-aligned: Summary Updater Button Group */}
              <div className="min-w-0 flex-shrink-0">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void handleTriggerAutomation('manual-summary')}
                  disabled={isTriggeringAutomation}
                  title="Run the configured post-meeting agent automation again for this summary"
                  className="mr-2"
                >
                  {isTriggeringAutomation ? <Loader2 size={16} className="animate-spin" /> : <Bot size={16} />}
                  Run automation
                </Button>
                <SummaryUpdaterButtonGroup
                  isSaving={isSaving}
                  isDirty={isTitleDirty || (summaryRef.current?.isDirty || false)}
                  onSave={onSaveAll}
                  onCopy={onCopySummary}
                  onFind={() => {
                    // TODO: Implement find in summary functionality
                    console.log('Find in summary clicked');
                  }}
                  onOpenFolder={onOpenFolder}
                  onExport={() => setExportDialogOpen(true)}
                  hasSummary={!!aiSummary}
                />
              </div>
            </div>
          )}
        </div>
      </div>

      <ExportMeetingDialog
        meetingId={meeting.id}
        open={exportDialogOpen}
        onOpenChange={setExportDialogOpen}
      />

      <div
        id="meeting-chat-panel"
        role="tabpanel"
        aria-labelledby="meeting-chat-tab"
        hidden={activeView !== 'chat'}
        className={activeView === 'chat' ? 'flex min-h-0 flex-1 flex-col' : 'hidden'}
      >
        <MeetingChatPanel
          meetingId={meeting.id}
          meetingTitle={meetingTitle}
          modelConfig={modelConfig}
          transcriptCount={transcripts.length}
          onTranscriptCitationSelect={onTranscriptCitationSelect}
          onRunAutomation={() => handleTriggerAutomation('manual-chat')}
          isRunningAutomation={isTriggeringAutomation}
        />
      </div>

      <div
        id="meeting-summary-panel"
        role="tabpanel"
        aria-labelledby="meeting-summary-tab"
        hidden={activeView !== 'summary'}
        className={activeView === 'summary' ? 'flex min-h-0 flex-1 flex-col' : 'hidden'}
      >
        {isSummaryLoading ? (
        <div className="flex flex-col h-full">
          {/* Show button group during generation */}
          <div className="flex items-center justify-center pb-4 pt-8">
            <SummaryGeneratorButtonGroup
              modelConfig={modelConfig}
              setModelConfig={setModelConfig}
              onSaveModelConfig={onSaveModelConfig}
              onGenerateSummary={onGenerateSummary}
              onStopGeneration={onStopGeneration}
              customPrompt={customPrompt}
              summaryStatus={summaryStatus}
              availableTemplates={availableTemplates}
              selectedTemplate={selectedTemplate}
              onTemplateSelect={onTemplateSelect}
              hasTranscripts={transcripts.length > 0}
              isModelConfigLoading={isModelConfigLoading}
              onOpenModelSettings={onOpenModelSettings}
            />
          </div>
          {/* Loading spinner */}
          <div className="flex items-center justify-center flex-1">
            <div className="text-center">
              <div className="mb-4 inline-block h-12 w-12 animate-spin rounded-full border-b-2 border-t-2 border-emerald-700"></div>
              <p className="font-medium text-slate-600">Generating AI Summary...</p>
            </div>
          </div>
        </div>
      ) : !aiSummary ? (
        <div className="flex flex-col h-full">
          {/* Centered Summary Generator Button Group when no summary */}
          <div className="flex flex-wrap items-center justify-center gap-2 pb-4 pt-8">
            <SummaryGeneratorButtonGroup
              modelConfig={modelConfig}
              setModelConfig={setModelConfig}
              onSaveModelConfig={onSaveModelConfig}
              onGenerateSummary={onGenerateSummary}
              onStopGeneration={onStopGeneration}
              customPrompt={customPrompt}
              summaryStatus={summaryStatus}
              availableTemplates={availableTemplates}
              selectedTemplate={selectedTemplate}
              onTemplateSelect={onTemplateSelect}
              hasTranscripts={transcripts.length > 0}
              hasSummary={false}
              isModelConfigLoading={isModelConfigLoading}
              onOpenModelSettings={onOpenModelSettings}
              languageSlot={transcripts.length > 0 ? languageSlot : undefined}
            />
          </div>
          <PostRecordingReviewChecklist meetingId={meeting.id} hasSummary={false} />
          {/* Empty state message */}
          <EmptyStateSummary
            onGenerate={() => onGenerateSummary(customPrompt)}
            hasModel={modelConfig.provider !== null && modelConfig.model !== null}
            isGenerating={isSummaryLoading}
          />
        </div>
      ) : transcripts?.length > 0 && (
        <div className="min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden">
          {summaryResponse && (
            <div className="fixed bottom-0 left-0 right-0 max-h-1/3 overflow-y-auto border-t border-slate-200 bg-white p-4 shadow-[0_-18px_45px_rgba(15,23,42,0.12)]">
              <h3 className="text-lg font-semibold mb-2">Meeting Summary</h3>
              <div className="grid grid-cols-2 gap-4">
                <div className="rounded-xl border border-slate-200 bg-white p-4 shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
                  <h4 className="font-medium mb-1">Key Points</h4>
                  <ul className="list-disc pl-4">
                    {summaryResponse.summary.key_points.blocks.map((block, i) => (
                      <li key={i} className="text-sm">{block.content}</li>
                    ))}
                  </ul>
                </div>
                <div className="mt-4 rounded-xl border border-slate-200 bg-white p-4 shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
                  <h4 className="font-medium mb-1">Action Items</h4>
                  <ul className="list-disc pl-4">
                    {summaryResponse.summary.action_items.blocks.map((block, i) => (
                      <li key={i} className="text-sm">{block.content}</li>
                    ))}
                  </ul>
                </div>
                <div className="mt-4 rounded-xl border border-slate-200 bg-white p-4 shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
                  <h4 className="font-medium mb-1">Decisions</h4>
                  <ul className="list-disc pl-4">
                    {summaryResponse.summary.decisions.blocks.map((block, i) => (
                      <li key={i} className="text-sm">{block.content}</li>
                    ))}
                  </ul>
                </div>
                <div className="mt-4 rounded-xl border border-slate-200 bg-white p-4 shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
                  <h4 className="font-medium mb-1">Main Topics</h4>
                  <ul className="list-disc pl-4">
                    {summaryResponse.summary.main_topics.blocks.map((block, i) => (
                      <li key={i} className="text-sm">{block.content}</li>
                    ))}
                  </ul>
                </div>
              </div>
              {summaryResponse.raw_summary ? (
                <div className="mt-4">
                  <h4 className="font-medium mb-1">Full Summary</h4>
                  <p className="text-sm whitespace-pre-wrap">{summaryResponse.raw_summary}</p>
                </div>
              ) : null}
            </div>
          )}
          <div className="w-full min-w-0 overflow-x-hidden p-6">
            <BlockNoteSummaryView
              ref={summaryRef}
              summaryData={aiSummary}
              onSave={onSaveSummary}
              onSummaryChange={onSummaryChange}
              onDirtyChange={onDirtyChange}
              status={summaryStatus}
              error={summaryError}
              onRegenerateSummary={() => {
                Analytics.trackButtonClick('regenerate_summary', 'meeting_details');
                onRegenerateSummary();
              }}
              meeting={{
                id: meeting.id,
                title: meetingTitle,
                created_at: meeting.created_at
              }}
            />
          </div>
          <PostRecordingReviewChecklist meetingId={meeting.id} hasSummary={!!aiSummary} />
          <div id="calendar-review">
            <CalendarEventPanel meetingId={meeting.id} hasSummary={!!aiSummary} />
          </div>
          <div id="notes-review">
            <AppleNotesExportPanel meetingId={meeting.id} hasSummary={!!aiSummary} summaryStatus={summaryStatus} />
          </div>
          <div id="reminders-review">
            <ReminderDraftReview meetingId={meeting.id} hasSummary={!!aiSummary} />
          </div>
          <div id="agent-review" className="mx-6 mb-6 mt-2 rounded-2xl border border-gray-200 bg-white p-4">
            <h3 className="text-sm font-semibold text-gray-900">Agent workflow runs</h3>
            <div className="mt-3">
              <AgentWorkflowRunsPanel meetingId={meeting.id} limit={4} />
            </div>
          </div>
          <div className="mx-6 mb-6 mt-2">
            <RecordingAuditTrail meetingId={meeting.id} limit={6} compact />
          </div>
          {summaryStatus !== 'idle' && (
            <div className={`mx-6 mb-6 mt-2 rounded-2xl border p-4 ${summaryStatus === 'error' ? 'border-red-200 bg-red-50 text-red-700' :
              summaryStatus === 'completed' ? 'border-emerald-200 bg-emerald-50 text-emerald-700' :
                'border-slate-200 bg-slate-50 text-slate-700'
              }`}>
              <p className="text-sm font-medium">{getSummaryStatusMessage(summaryStatus)}</p>
            </div>
          )}
        </div>
        )}
      </div>
    </div>
  );
}
