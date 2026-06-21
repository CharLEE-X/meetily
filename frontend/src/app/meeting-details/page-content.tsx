"use client";
import { useState, useCallback, useEffect, useRef, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react';
import { motion } from 'framer-motion';
import { Summary, SummaryResponse } from '@/types';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import Analytics from '@/lib/analytics';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { TranscriptPanel, TranscriptPanelHandle } from '@/components/MeetingDetails/TranscriptPanel';
import { SummaryPanel } from '@/components/MeetingDetails/SummaryPanel';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { MeetingChatCitation } from '@/services/meetingChatService';
import { RecallXShell } from '@/components/recallx';

// Custom hooks
import { useMeetingData } from '@/hooks/meeting-details/useMeetingData';
import { useSummaryGeneration } from '@/hooks/meeting-details/useSummaryGeneration';
import { useTemplates } from '@/hooks/meeting-details/useTemplates';
import { useCopyOperations } from '@/hooks/meeting-details/useCopyOperations';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';
import { useConfig } from '@/contexts/ConfigContext';
import { exportMeeting, getExportSettings } from '@/services/exportService';

const SUMMARY_CONTEXT_HISTORY_KEY = 'meetily.summaryContextHistory';
const SUMMARY_CONTEXT_HISTORY_LIMIT = 8;
const TRANSCRIPT_PANE_WIDTH_KEY = 'meetily.meetingDetailsTranscriptPaneWidth';
const TRANSCRIPT_PANE_MIN_WIDTH = 360;
const TRANSCRIPT_PANE_MAX_WIDTH = 760;
const TRANSCRIPT_PANE_DEFAULT_WIDTH = 580;

function normalizeSummaryContext(value: string) {
  return value.trim();
}

function readSummaryContextHistory() {
  if (typeof window === 'undefined') {
    return [];
  }

  try {
    const raw = window.localStorage.getItem(SUMMARY_CONTEXT_HISTORY_KEY);
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .filter((value): value is string => typeof value === 'string')
      .map(normalizeSummaryContext)
      .filter(Boolean)
      .slice(0, SUMMARY_CONTEXT_HISTORY_LIMIT);
  } catch (error) {
    console.warn('Failed to load summary context history:', error);
    return [];
  }
}

function clampTranscriptPaneWidth(value: number) {
  return Math.min(TRANSCRIPT_PANE_MAX_WIDTH, Math.max(TRANSCRIPT_PANE_MIN_WIDTH, value));
}

function readTranscriptPaneWidth() {
  if (typeof window === 'undefined') {
    return TRANSCRIPT_PANE_DEFAULT_WIDTH;
  }

  const stored = Number(window.localStorage.getItem(TRANSCRIPT_PANE_WIDTH_KEY));
  return Number.isFinite(stored)
    ? clampTranscriptPaneWidth(stored)
    : TRANSCRIPT_PANE_DEFAULT_WIDTH;
}

export default function PageContent({
  meeting,
  summaryData,
  shouldAutoGenerate = false,
  onAutoGenerateComplete,
  onMeetingUpdated,
  onRefetchTranscripts,
  // Pagination props for efficient transcript loading
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
  loadUntilTranscript,
}: {
  meeting: any;
  summaryData: Summary | null;
  shouldAutoGenerate?: boolean;
  onAutoGenerateComplete?: () => void;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
  // Pagination props
  segments?: any[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;
  loadUntilTranscript?: (target: { transcriptId?: string | null; audioStartTime?: number | null }) => Promise<boolean>;
}) {
  console.log('📄 PAGE CONTENT: Initializing with data:', {
    meetingId: meeting.id,
    summaryDataKeys: summaryData ? Object.keys(summaryData) : null,
    transcriptsCount: meeting.transcripts?.length
  });

  // State
  const [customPrompt, setCustomPrompt] = useState<string>('');
  const [customPromptHistory, setCustomPromptHistory] = useState<string[]>([]);
  const [transcriptPaneWidth, setTranscriptPaneWidth] = useState<number>(TRANSCRIPT_PANE_DEFAULT_WIDTH);
  const [isRecording] = useState(false);
  const [summaryResponse] = useState<SummaryResponse | null>(null);

  // Ref to store the modal open function from SummaryGeneratorButtonGroup
  const openModelSettingsRef = useRef<(() => void) | null>(null);
  const autoExportedMeetingRef = useRef<string | null>(null);
  const transcriptPanelRef = useRef<TranscriptPanelHandle | null>(null);
  const pendingTranscriptJumpRef = useRef<{ transcriptId?: string | null; audioStartTime?: number | null } | null>(null);

  // Sidebar context
  const { serverAddress } = useSidebar();

  // Get model config from ConfigContext
  const { modelConfig, setModelConfig } = useConfig();

  // Custom hooks
  const meetingData = useMeetingData({ meeting, summaryData, onMeetingUpdated });
  const templates = useTemplates(meeting.id);

  // Callback to register the modal open function
  const handleRegisterModalOpen = (openFn: () => void) => {
    console.log('📝 Registering modal open function in PageContent');
    openModelSettingsRef.current = openFn;
  };

  // Callback to trigger modal open (called from error handler)
  const handleOpenModelSettings = () => {
    console.log('🔔 Opening model settings from PageContent');
    if (openModelSettingsRef.current) {
      openModelSettingsRef.current();
    } else {
      console.warn('⚠️ Modal open function not yet registered');
    }
  };

  // Save model config to backend database and sync via event
  const handleSaveModelConfig = async (config?: ModelConfig) => {
    if (!config) return;
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey ?? null,
        ollamaEndpoint: config.ollamaEndpoint ?? null,
      });

      // Emit event so ConfigContext and other listeners stay in sync
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      toast.success('Model settings saved successfully');
    } catch (error) {
      console.error('Failed to save model config:', error);
      toast.error('Failed to save model settings');
    }
  };

  const summaryGeneration = useSummaryGeneration({
    meeting,
    transcripts: meetingData.transcripts,
    modelConfig: modelConfig,
    isModelConfigLoading: false, // ConfigContext loads on mount
    selectedTemplate: templates.selectedTemplate,
    onMeetingUpdated,
    updateMeetingTitle: meetingData.updateMeetingTitle,
    setAiSummary: meetingData.setAiSummary,
    onOpenModelSettings: handleOpenModelSettings,
  });

  const copyOperations = useCopyOperations({
    meeting,
    transcripts: meetingData.transcripts,
    meetingTitle: meetingData.meetingTitle,
    aiSummary: meetingData.aiSummary,
    blockNoteSummaryRef: meetingData.blockNoteSummaryRef,
  });

  const meetingOperations = useMeetingOperations({
    meeting,
  });

  useEffect(() => {
    setCustomPromptHistory(readSummaryContextHistory());
    setTranscriptPaneWidth(readTranscriptPaneWidth());
  }, []);

  const handleResizeStart = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = transcriptPaneWidth;

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const nextWidth = clampTranscriptPaneWidth(startWidth + moveEvent.clientX - startX);
      setTranscriptPaneWidth(nextWidth);
    };

    const handlePointerUp = (upEvent: PointerEvent) => {
      const finalWidth = clampTranscriptPaneWidth(startWidth + upEvent.clientX - startX);
      setTranscriptPaneWidth(finalWidth);
      window.localStorage.setItem(TRANSCRIPT_PANE_WIDTH_KEY, String(finalWidth));
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp, { once: true });
  };

  const updateTranscriptPaneWidth = (nextWidth: number) => {
    const clampedWidth = clampTranscriptPaneWidth(nextWidth);
    setTranscriptPaneWidth(clampedWidth);
    window.localStorage.setItem(TRANSCRIPT_PANE_WIDTH_KEY, String(clampedWidth));
  };

  const handleResizeKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight' && event.key !== 'Home' && event.key !== 'End') {
      return;
    }

    event.preventDefault();
    if (event.key === 'Home') {
      updateTranscriptPaneWidth(TRANSCRIPT_PANE_MIN_WIDTH);
      return;
    }
    if (event.key === 'End') {
      updateTranscriptPaneWidth(TRANSCRIPT_PANE_MAX_WIDTH);
      return;
    }

    const step = event.shiftKey ? 48 : 16;
    updateTranscriptPaneWidth(transcriptPaneWidth + (event.key === 'ArrowRight' ? step : -step));
  };

  const rememberSummaryContext = (value: string) => {
    const normalized = normalizeSummaryContext(value);
    if (!normalized) {
      return;
    }

    setCustomPromptHistory((current) => {
      const next = [
        normalized,
        ...current.filter((entry) => normalizeSummaryContext(entry) !== normalized),
      ].slice(0, SUMMARY_CONTEXT_HISTORY_LIMIT);

      try {
        window.localStorage.setItem(SUMMARY_CONTEXT_HISTORY_KEY, JSON.stringify(next));
      } catch (error) {
        console.warn('Failed to save summary context history:', error);
      }

      return next;
    });
  };

  const handleGenerateSummary = async (prompt: string) => {
    rememberSummaryContext(prompt);
    await summaryGeneration.handleGenerateSummary(prompt);
  };

  const handleRegenerateSummary = async () => {
    rememberSummaryContext(customPrompt);
    await summaryGeneration.handleRegenerateSummary(customPrompt);
  };

  const scrollToTranscriptCitation = useCallback((target: { transcriptId?: string | null; audioStartTime?: number | null }) => {
    const didScroll = transcriptPanelRef.current?.scrollToTranscript(target) ?? false;
    if (didScroll) {
      pendingTranscriptJumpRef.current = null;
    }
    return didScroll;
  }, []);

  const handleTranscriptCitationSelect = useCallback(async (citation: MeetingChatCitation) => {
    if (citation.sourceType !== 'transcript') {
      return;
    }

    const target = {
      transcriptId: citation.transcriptId,
      audioStartTime: citation.audioStartTime,
    };

    if (scrollToTranscriptCitation(target)) {
      return;
    }

    if (!loadUntilTranscript) {
      toast.info('That transcript segment is not loaded yet.');
      return;
    }

    pendingTranscriptJumpRef.current = target;
    const loaded = await loadUntilTranscript(target);
    if (!loaded) {
      pendingTranscriptJumpRef.current = null;
      toast.info('Could not find that transcript segment in the loaded meeting.');
    }
  }, [loadUntilTranscript, scrollToTranscriptCitation]);

  useEffect(() => {
    const pendingJump = pendingTranscriptJumpRef.current;
    if (pendingJump) {
      scrollToTranscriptCitation(pendingJump);
    }
  }, [segments, meetingData.transcripts, scrollToTranscriptCitation]);

  // Track page view
  useEffect(() => {
    Analytics.trackPageView('meeting_details');
  }, []);

  // Auto-generate summary when flag is set
  useEffect(() => {
    let cancelled = false;

    const autoGenerate = async () => {
      if (shouldAutoGenerate && meetingData.transcripts.length > 0 && !cancelled) {
        console.log(`🤖 Auto-generating summary with ${modelConfig.provider}/${modelConfig.model}...`);
        await summaryGeneration.handleGenerateSummary('');

        // Notify parent that auto-generation is complete (only if not cancelled)
        if (onAutoGenerateComplete && !cancelled) {
          onAutoGenerateComplete();
        }
      }
    };

    autoGenerate();

    // Cleanup: cancel if component unmounts or meeting changes
    return () => {
      cancelled = true;
    };
  }, [shouldAutoGenerate, meeting.id]); // Re-run if meeting changes

  useEffect(() => {
    if (
      summaryGeneration.summaryStatus !== 'completed' ||
      !meetingData.aiSummary ||
      autoExportedMeetingRef.current === meeting.id
    ) {
      return;
    }

    let cancelled = false;
    const maybeAutoExport = async () => {
      try {
        const settings = await getExportSettings();
        if (cancelled || !settings.autoExportEnabled) return;

        autoExportedMeetingRef.current = meeting.id;
        const result = await exportMeeting(meeting.id, {
          format: settings.autoExportFormat,
          sections: settings.sections,
          destinationDir: settings.destinationDir ?? null,
          fileName: settings.fileNameTemplate,
          autoExport: true,
        });

        if (!cancelled) {
          toast.success('Meeting auto-exported', {
            description: result.filePath,
          });
        }
      } catch (error) {
        console.error('Auto-export failed:', error);
        if (!cancelled) {
          toast.error('Auto-export failed', {
            description: String(error),
          });
        }
      }
    };

    void maybeAutoExport();

    return () => {
      cancelled = true;
    };
  }, [summaryGeneration.summaryStatus, meeting.id, meetingData.aiSummary]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.7, ease: [0.32, 0.72, 0, 1] }}
      className="flex h-screen flex-col bg-recallx-black text-recallx-text"
    >
      <RecallXShell className="flex flex-1 overflow-hidden">
        <TranscriptPanel
          ref={transcriptPanelRef}
          transcripts={meetingData.transcripts}
          customPrompt={customPrompt}
          customPromptHistory={customPromptHistory}
          onPromptChange={setCustomPrompt}
          onCopyTranscript={copyOperations.handleCopyTranscript}
          onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
          isRecording={isRecording}
          disableAutoScroll={true}
          // Pagination props for efficient loading
          usePagination={true}
          segments={segments}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          totalCount={totalCount}
          loadedCount={loadedCount}
          onLoadMore={onLoadMore}
          // Retranscription props
          meetingId={meeting.id}
          meetingFolderPath={meeting.folder_path}
          onRefetchTranscripts={onRefetchTranscripts}
          style={{
            width: transcriptPaneWidth,
            minWidth: TRANSCRIPT_PANE_MIN_WIDTH,
            maxWidth: TRANSCRIPT_PANE_MAX_WIDTH,
          }}
        />
        <div
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize transcript and summary columns"
          aria-valuemin={TRANSCRIPT_PANE_MIN_WIDTH}
          aria-valuemax={TRANSCRIPT_PANE_MAX_WIDTH}
          aria-valuenow={transcriptPaneWidth}
          tabIndex={0}
          className="group hidden w-3 shrink-0 cursor-col-resize bg-transparent outline-none md:flex md:items-stretch md:justify-center"
          onPointerDown={handleResizeStart}
          onKeyDown={handleResizeKeyDown}
        >
          <span className="my-3 w-px rounded-full bg-white/10 transition-colors duration-700 recallx-ease group-hover:bg-recallx-acid group-focus:bg-recallx-acid" />
        </div>
        <SummaryPanel
          meeting={meeting}
          meetingTitle={meetingData.meetingTitle}
          onTitleChange={meetingData.handleTitleChange}
          isEditingTitle={meetingData.isEditingTitle}
          onStartEditTitle={() => meetingData.setIsEditingTitle(true)}
          onFinishEditTitle={() => meetingData.setIsEditingTitle(false)}
          isTitleDirty={meetingData.isTitleDirty}
          summaryRef={meetingData.blockNoteSummaryRef}
          isSaving={meetingData.isSaving}
          onSaveAll={meetingData.saveAllChanges}
          onCopySummary={copyOperations.handleCopySummary}
          onOpenFolder={meetingOperations.handleOpenMeetingFolder}
          aiSummary={meetingData.aiSummary}
          summaryStatus={summaryGeneration.summaryStatus}
          transcripts={meetingData.transcripts}
          modelConfig={modelConfig}
          setModelConfig={setModelConfig}
          onSaveModelConfig={handleSaveModelConfig}
          onGenerateSummary={handleGenerateSummary}
          onStopGeneration={summaryGeneration.handleStopGeneration}
          customPrompt={customPrompt}
          summaryResponse={summaryResponse}
          onSaveSummary={meetingData.handleSaveSummary}
          onSummaryChange={meetingData.handleSummaryChange}
          onDirtyChange={meetingData.setIsSummaryDirty}
          summaryError={summaryGeneration.summaryError}
          onRegenerateSummary={handleRegenerateSummary}
          getSummaryStatusMessage={summaryGeneration.getSummaryStatusMessage}
          availableTemplates={templates.availableTemplates}
          selectedTemplate={templates.selectedTemplate}
          onTemplateSelect={templates.handleTemplateSelection}
          isModelConfigLoading={false}
          onOpenModelSettings={handleRegisterModalOpen}
          onTranscriptCitationSelect={handleTranscriptCitationSelect}
        />
      </RecallXShell>
    </motion.div>
  );
}
