'use client';

import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { RecordingControls } from '@/components/RecordingControls';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import { useRecordingState, RecordingStatus } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useConfig } from '@/contexts/ConfigContext';
import { StatusOverlays } from '@/app/_components/StatusOverlays';
import Analytics from '@/lib/analytics';
import { SettingsModals } from './_components/SettingsModal';
import { TranscriptPanel } from './_components/TranscriptPanel';
import { useModalState } from '@/hooks/useModalState';
import { useRecordingStateSync } from '@/hooks/useRecordingStateSync';
import { useRecordingStart } from '@/hooks/useRecordingStart';
import { useRecordingStop } from '@/hooks/useRecordingStop';
import { useTranscriptRecovery } from '@/hooks/useTranscriptRecovery';
import { TranscriptRecovery } from '@/components/TranscriptRecovery';
import { MeetingDetectionPrompt } from '@/components/MeetingDetectionPrompt';
import { indexedDBService } from '@/services/indexedDBService';
import { toast } from 'sonner';
import { useRouter } from 'next/navigation';
import { useImportDialog } from '@/contexts/ImportDialogContext';
import { Bot, CheckCircle2, Clock3, FileText, MessageCircle, Mic, Settings, Upload } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface HomeDashboardProps {
  meetings: Array<{ id: string; title: string }>;
  hasMicrophone: boolean;
  transcriptModelLabel: string;
  summaryModelLabel: string;
  importEnabled: boolean;
  onStartRecording: () => void;
  onOpenMeeting: (meetingId: string) => void;
  onOpenSummaryChat: () => void;
  onOpenSettings: () => void;
  onImportAudio: () => void;
}

function HomeDashboard({
  meetings,
  hasMicrophone,
  transcriptModelLabel,
  summaryModelLabel,
  importEnabled,
  onStartRecording,
  onOpenMeeting,
  onOpenSummaryChat,
  onOpenSettings,
  onImportAudio,
}: HomeDashboardProps) {
  const recentMeetings = meetings.slice(0, 5);
  const setupItems = [
    {
      label: 'Microphone permission',
      detail: hasMicrophone ? 'Ready for recording' : 'Needs permission before capture',
      ready: hasMicrophone,
    },
    {
      label: 'Transcription model',
      detail: transcriptModelLabel,
      ready: Boolean(transcriptModelLabel),
    },
    {
      label: 'Summary model',
      detail: summaryModelLabel,
      ready: Boolean(summaryModelLabel),
    },
  ];

  return (
    <div className="flex min-h-0 flex-1 overflow-y-auto overflow-x-hidden bg-background px-8 py-8">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 pb-28">
        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-[0_18px_45px_rgba(15,23,42,0.06)]">
          <div className="flex flex-col gap-6 lg:flex-row lg:items-center lg:justify-between">
            <div>
              <p className="text-sm font-semibold uppercase tracking-wide text-emerald-700">Dashboard</p>
              <h1 className="mt-2 text-3xl font-semibold tracking-normal text-slate-950">Start, review, and automate meetings</h1>
              <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-600">
                Use this home view to start the next recording, jump back into recent meetings, ask across summaries, or check whether the local recording setup is ready.
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button type="button" onClick={onStartRecording} className="bg-red-600 text-white hover:bg-red-700">
                <Mic className="h-4 w-4" />
                Start recording
              </Button>
              <Button type="button" variant="outline" onClick={onOpenSummaryChat}>
                <MessageCircle className="h-4 w-4" />
                Ask meetings
              </Button>
              {importEnabled && (
                <Button type="button" variant="outline" onClick={onImportAudio}>
                  <Upload className="h-4 w-4" />
                  Import audio
                </Button>
              )}
            </div>
          </div>
        </section>

        <div className="grid gap-6 lg:grid-cols-[1.35fr_0.65fr]">
          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
            <div className="flex items-center justify-between gap-3">
              <div>
                <h2 className="text-base font-semibold text-slate-950">Recent meetings</h2>
                <p className="mt-1 text-sm text-slate-500">Continue review, summary export, reminders, or agent automation.</p>
              </div>
              <span className="rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-600">{meetings.length} total</span>
            </div>

            <div className="mt-4 divide-y divide-slate-200 overflow-hidden rounded-xl border border-slate-200">
              {recentMeetings.length === 0 ? (
                <div className="flex flex-col items-center justify-center px-6 py-12 text-center">
                  <FileText className="h-9 w-9 text-slate-300" />
                  <h3 className="mt-3 text-sm font-semibold text-slate-950">No meetings yet</h3>
                  <p className="mt-1 max-w-sm text-sm leading-6 text-slate-500">
                    Start a recording or import audio to create your first meeting transcript and summary.
                  </p>
                </div>
              ) : recentMeetings.map((meeting) => (
                <button
                  key={meeting.id}
                  type="button"
                  onClick={() => onOpenMeeting(meeting.id)}
                  className="flex w-full items-center justify-between gap-4 bg-white px-4 py-3 text-left transition hover:bg-slate-50"
                >
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium text-slate-950">{meeting.title || 'Untitled meeting'}</span>
                    <span className="mt-1 flex items-center gap-1 text-xs text-slate-500">
                      <Clock3 className="h-3.5 w-3.5" />
                      Open details
                    </span>
                  </span>
                  <FileText className="h-4 w-4 shrink-0 text-slate-400" />
                </button>
              ))}
            </div>
          </section>

          <div className="space-y-6">
            <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
              <h2 className="text-base font-semibold text-slate-950">Setup status</h2>
              <div className="mt-4 space-y-3">
                {setupItems.map((item) => (
                  <div key={item.label} className="flex items-start gap-3 rounded-xl border border-slate-200 bg-slate-50 p-3">
                    <CheckCircle2 className={`mt-0.5 h-4 w-4 shrink-0 ${item.ready ? 'text-emerald-600' : 'text-amber-600'}`} />
                    <div className="min-w-0">
                      <p className="text-sm font-medium text-slate-950">{item.label}</p>
                      <p className="mt-1 truncate text-xs text-slate-500">{item.detail}</p>
                    </div>
                  </div>
                ))}
              </div>
              <Button type="button" variant="outline" className="mt-4 w-full" onClick={onOpenSettings}>
                <Settings className="h-4 w-4" />
                Open settings
              </Button>
            </section>

            <section className="rounded-2xl border border-emerald-200 bg-emerald-50 p-5">
              <div className="flex items-start gap-3">
                <div className="rounded-xl bg-white p-2 text-emerald-700">
                  <Bot className="h-5 w-5" />
                </div>
                <div>
                  <h2 className="text-base font-semibold text-emerald-950">Agent-ready workflows</h2>
                  <p className="mt-2 text-sm leading-6 text-emerald-900">
                    After summaries are generated, Meetily can prepare Codex, Claude, Cursor, Linear, reminders, notes, and calendar handoffs from the meeting context.
                  </p>
                  <Button type="button" variant="outline" className="mt-4 border-emerald-300 bg-white text-emerald-800 hover:bg-emerald-100" onClick={onOpenSettings}>
                    Configure automation
                  </Button>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function Home() {
  // Local page state (not moved to contexts)
  const [isRecording, setIsRecordingState] = useState(false);
  const [barHeights, setBarHeights] = useState(['58%', '76%', '58%']);
  const [showRecoveryDialog, setShowRecoveryDialog] = useState(false);

  // Use contexts for state management
  const { meetingTitle } = useTranscripts();
  const { transcripts } = useTranscripts();
  const { transcriptModelConfig, selectedDevices, modelConfig, betaFeatures } = useConfig();
  const recordingState = useRecordingState();
  const { openImportDialog } = useImportDialog();

  // Extract status from global state
  const { status, isStopping, isProcessing, isSaving } = recordingState;

  // Hooks
  const { hasMicrophone } = usePermissionCheck();
  const { setIsMeetingActive, isCollapsed: sidebarCollapsed, refetchMeetings, meetings } = useSidebar();
  const { modals, messages, showModal, hideModal } = useModalState(transcriptModelConfig);
  const { isRecordingDisabled, setIsRecordingDisabled } = useRecordingStateSync(isRecording, setIsRecordingState, setIsMeetingActive);
  const { handleRecordingStart } = useRecordingStart(isRecording, setIsRecordingState, showModal);

  // Get handleRecordingStop function and setIsStopping (state comes from global context)
  const { handleRecordingStop, setIsStopping } = useRecordingStop(
    setIsRecordingState,
    setIsRecordingDisabled
  );

  // Recovery hook
  const {
    recoverableMeetings,
    isLoading: isLoadingRecovery,
    isRecovering,
    checkForRecoverableTranscripts,
    recoverMeeting,
    loadMeetingTranscripts,
    deleteRecoverableMeeting
  } = useTranscriptRecovery();

  const router = useRouter();

  useEffect(() => {
    // Track page view
    Analytics.trackPageView('home');
  }, []);

  // Startup recovery check
  useEffect(() => {
    const performStartupChecks = async () => {
      try {
        // Skip recovery check if currently recording or processing stop
        // This prevents the recovery dialog from showing when:
        if (recordingState.isRecording ||
          status === RecordingStatus.STOPPING ||
          status === RecordingStatus.PROCESSING_TRANSCRIPTS ||
          status === RecordingStatus.SAVING) {
          console.log('Skipping recovery check - recording in progress or processing');
          return;
        }

        // 1. Clean up old meetings (7+ days)
        try {
          await indexedDBService.deleteOldMeetings(7);
        } catch (error) {
          console.warn('⚠️ Failed to clean up old meetings:', error);
        }

        // 2. Clean up saved meetings (24+ hours after save)
        try {
          await indexedDBService.deleteSavedMeetings(24);
        } catch (error) {
          console.warn('⚠️ Failed to clean up saved meetings:', error);
        }

        // 3. Always check for recoverable meetings on startup
        // Don't skip based on sessionStorage - we need to check every time
        await checkForRecoverableTranscripts();
      } catch (error) {
        console.error('Failed to perform startup checks:', error);
      }
    };

    performStartupChecks();
  }, [checkForRecoverableTranscripts, recordingState.isRecording, status]);

  // Watch for recoverable meetings changes and show dialog once per session
  useEffect(() => {
    // Only show dialog if we have meetings and haven't shown it yet this session
    if (recoverableMeetings.length > 0) {
      const shownThisSession = sessionStorage.getItem('recovery_dialog_shown');
      if (!shownThisSession) {
        setShowRecoveryDialog(true);
        sessionStorage.setItem('recovery_dialog_shown', 'true');
      }
    }
  }, [recoverableMeetings]);

  // Handle recovery with toast notifications and navigation
  const handleRecovery = async (meetingId: string) => {
    try {
      const result = await recoverMeeting(meetingId);

      if (result.success) {
        toast.success('Meeting recovered successfully!', {
          description: result.audioRecoveryStatus?.status === 'success'
            ? 'Transcripts and audio recovered'
            : 'Transcripts recovered (no audio available)',
          action: result.meetingId ? {
            label: 'View Meeting',
            onClick: () => {
              router.push(`/meeting-details?id=${result.meetingId}`);
            }
          } : undefined,
          duration: 10000,
        });

        // Refresh sidebar to show the newly recovered meeting
        await refetchMeetings();

        // If no more recoverable meetings, clear session flag so dialog can show again
        if (recoverableMeetings.length === 0) {
          sessionStorage.removeItem('recovery_dialog_shown');
        }

        // Auto-navigate after a short delay
        if (result.meetingId) {
          setTimeout(() => {
            router.push(`/meeting-details?id=${result.meetingId}`);
          }, 2000);
        }
      }
    } catch (error) {
      toast.error('Failed to recover meeting', {
        description: error instanceof Error ? error.message : 'Unknown error occurred',
      });
      throw error;
    }
  };

  // Handle dialog close - clear session flag if no meetings left
  const handleDialogClose = () => {
    setShowRecoveryDialog(false);
    // If user closes dialog and there are no more meetings, clear the flag
    // This allows the dialog to show again next session if new meetings appear
    if (recoverableMeetings.length === 0) {
      sessionStorage.removeItem('recovery_dialog_shown');
    }
  };

  useEffect(() => {
    if (recordingState.isRecording) {
      const interval = setInterval(() => {
        setBarHeights(prev => {
          const newHeights = [...prev];
          newHeights[0] = Math.random() * 20 + 10 + 'px';
          newHeights[1] = Math.random() * 20 + 10 + 'px';
          newHeights[2] = Math.random() * 20 + 10 + 'px';
          return newHeights;
        });
      }, 300);

      return () => clearInterval(interval);
    }
  }, [recordingState.isRecording]);

  // Computed values using global status
  const isProcessingStop = status === RecordingStatus.PROCESSING_TRANSCRIPTS || isProcessing;
  const shouldShowTranscriptWorkspace = recordingState.isRecording || transcripts.length > 0 || isProcessingStop || status === RecordingStatus.SAVING || isStopping;
  const transcriptModelLabel = `${transcriptModelConfig.provider} / ${transcriptModelConfig.model}`;
  const summaryModelLabel = `${modelConfig.provider} / ${modelConfig.model}`;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
      className="flex h-screen flex-col bg-background"
    >
      {/* All Modals supported*/}
      <SettingsModals
        modals={modals}
        messages={messages}
        onClose={hideModal}
      />

      {/* Recovery Dialog */}
      <TranscriptRecovery
        isOpen={showRecoveryDialog}
        onClose={handleDialogClose}
        recoverableMeetings={recoverableMeetings}
        onRecover={handleRecovery}
        onDelete={deleteRecoverableMeeting}
        onLoadPreview={loadMeetingTranscripts}
      />
      <div className="flex flex-1 overflow-hidden">
        {shouldShowTranscriptWorkspace ? (
          <TranscriptPanel
            isProcessingStop={isProcessingStop}
            isStopping={isStopping}
            showModal={showModal}
          />
        ) : (
          <HomeDashboard
            meetings={meetings}
            hasMicrophone={hasMicrophone}
            transcriptModelLabel={transcriptModelLabel}
            summaryModelLabel={summaryModelLabel}
            importEnabled={betaFeatures.importAndRetranscribe}
            onStartRecording={handleRecordingStart}
            onOpenMeeting={(meetingId) => router.push(`/meeting-details?id=${meetingId}`)}
            onOpenSummaryChat={() => router.push('/summary-chat')}
            onOpenSettings={() => router.push('/settings')}
            onImportAudio={() => openImportDialog()}
          />
        )}

        {!isProcessingStop && status !== RecordingStatus.SAVING && (
          <MeetingDetectionPrompt
            sidebarCollapsed={sidebarCollapsed}
            onStartRecording={handleRecordingStart}
            isRecording={recordingState.isRecording}
          />
        )}

        {/* Recording controls - only show when permissions are granted or already recording and not showing status messages */}
        {shouldShowTranscriptWorkspace &&
          (hasMicrophone || isRecording) &&
          status !== RecordingStatus.PROCESSING_TRANSCRIPTS &&
          status !== RecordingStatus.SAVING && (
            <div className="fixed bottom-10 left-0 right-0 z-10">
              <div
                className="flex justify-center pl-6 transition-[margin] duration-300 ease-out"
                style={{
                  marginLeft: sidebarCollapsed ? '4.5rem' : '18rem'
                }}
              >
                <div className="w-2/3 max-w-[750px] flex justify-center">
                  <div className="flex items-center rounded-full border border-slate-200/80 bg-white/95 shadow-[0_22px_55px_rgba(15,23,42,0.16)]">
                    <RecordingControls
                      isRecording={recordingState.isRecording}
                      onRecordingStop={(callApi = true) => handleRecordingStop(callApi)}
                      onRecordingStart={handleRecordingStart}
                      onTranscriptReceived={() => { }} // Not actually used by RecordingControls
                      onStopInitiated={() => setIsStopping(true)}
                      barHeights={barHeights}
                      onTranscriptionError={(message) => {
                        showModal('errorAlert', message);
                      }}
                      isRecordingDisabled={isRecordingDisabled}
                      isParentProcessing={isProcessingStop}
                      selectedDevices={selectedDevices}
                      meetingName={meetingTitle}
                    />
                  </div>
                </div>
              </div>
            </div>
          )}

        {/* Status Overlays - Processing and Saving */}
        <StatusOverlays
          isProcessing={status === RecordingStatus.PROCESSING_TRANSCRIPTS && !recordingState.isRecording}
          isSaving={status === RecordingStatus.SAVING}
          sidebarCollapsed={sidebarCollapsed}
        />
      </div>
    </motion.div>
  );
}
