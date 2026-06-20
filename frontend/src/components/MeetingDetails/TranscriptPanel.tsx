"use client";

import { Transcript, TranscriptSegmentData } from '@/types';
import { VirtualizedTranscriptView, VirtualizedTranscriptViewHandle } from '@/components/VirtualizedTranscriptView';
import { TranscriptButtonGroup } from './TranscriptButtonGroup';
import { SpeakerScreenshotPanel } from './SpeakerScreenshotPanel';
import { forwardRef, useCallback, useImperativeHandle, useMemo, useRef, useState, type CSSProperties } from 'react';
import { TranscriptSpeakerLabelView } from '@/services/speakerService';

interface TranscriptPanelProps {
  transcripts: Transcript[];
  customPrompt: string;
  customPromptHistory?: string[];
  onPromptChange: (value: string) => void;
  onCopyTranscript: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  isRecording: boolean;
  disableAutoScroll?: boolean;

  // Optional pagination props (when using virtualization)
  usePagination?: boolean;
  segments?: TranscriptSegmentData[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;

  // Retranscription props
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
  className?: string;
  style?: CSSProperties;
}

export interface TranscriptPanelHandle {
  scrollToTranscript: (target: { transcriptId?: string | null; audioStartTime?: number | null }) => boolean;
}

export const TranscriptPanel = forwardRef<TranscriptPanelHandle, TranscriptPanelProps>(function TranscriptPanel({
  transcripts,
  customPrompt,
  customPromptHistory = [],
  onPromptChange,
  onCopyTranscript,
  onOpenMeetingFolder,
  isRecording,
  disableAutoScroll = false,
  usePagination = false,
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
  className = '',
  style,
}, ref) {
  const [speakerLabelsByTranscriptId, setSpeakerLabelsByTranscriptId] = useState<Record<string, TranscriptSpeakerLabelView>>({});
  const [highlightedSegmentId, setHighlightedSegmentId] = useState<string | null>(null);
  const transcriptViewRef = useRef<VirtualizedTranscriptViewHandle | null>(null);

  // Convert transcripts to segments if pagination is not used but we want virtualization
  const convertedSegments = useMemo(() => {
    if (usePagination && segments) {
      return segments;
    }
    // Convert transcripts to segments for virtualization
    return transcripts.map(t => ({
      id: t.id,
      timestamp: t.audio_start_time ?? 0,
      endTime: t.audio_end_time,
      text: t.text,
      confidence: t.confidence,
    }));
  }, [transcripts, usePagination, segments]);

  const handleSpeakerLabelsChange = useCallback((labelsByTranscriptId: Record<string, TranscriptSpeakerLabelView>) => {
    setSpeakerLabelsByTranscriptId(labelsByTranscriptId);
  }, []);

  const resolveTargetSegment = useCallback((target: { transcriptId?: string | null; audioStartTime?: number | null }) => {
    if (target.transcriptId) {
      const exact = convertedSegments.find((segment) => segment.id === target.transcriptId);
      if (exact) return exact;
    }

    if (typeof target.audioStartTime === 'number' && Number.isFinite(target.audioStartTime)) {
      let closest: TranscriptSegmentData | null = null;
      let closestDistance = Number.POSITIVE_INFINITY;

      convertedSegments.forEach((segment) => {
        const distance = Math.abs(segment.timestamp - target.audioStartTime!);
        if (distance < closestDistance) {
          closest = segment;
          closestDistance = distance;
        }
      });

      return closestDistance <= 0.75 ? closest : null;
    }

    return null;
  }, [convertedSegments]);

  useImperativeHandle(ref, () => ({
    scrollToTranscript: (target) => {
      const segment = resolveTargetSegment(target);
      if (!segment) return false;

      const didScroll = transcriptViewRef.current?.scrollToSegment({
        segmentId: segment.id,
        timestamp: segment.timestamp,
      }) ?? false;

      if (didScroll) {
        setHighlightedSegmentId(segment.id);
        window.setTimeout(() => {
          setHighlightedSegmentId((current) => current === segment.id ? null : current);
        }, 2400);
      }

      return didScroll;
    },
  }), [resolveTargetSegment]);

  return (
    <div
      className={`relative hidden min-w-0 shrink-0 flex-col border-r border-slate-200/80 bg-white/95 md:flex ${className}`}
      style={style}
    >
      {/* Title area */}
      <div className="border-b border-slate-200/80 bg-white/90 p-4">
        <TranscriptButtonGroup
          transcriptCount={usePagination ? (totalCount ?? convertedSegments.length) : (transcripts?.length || 0)}
          onCopyTranscript={onCopyTranscript}
          onOpenMeetingFolder={onOpenMeetingFolder}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onRefetchTranscripts={onRefetchTranscripts}
        />
      </div>

      {meetingId && (
        <SpeakerScreenshotPanel
          meetingId={meetingId}
          onSpeakerLabelsChange={handleSpeakerLabelsChange}
        />
      )}

      {/* Transcript content - use virtualized view for better performance */}
      <div className="flex-1 overflow-hidden pb-4">
        <VirtualizedTranscriptView
          ref={transcriptViewRef}
          segments={convertedSegments}
          isRecording={isRecording}
          isPaused={false}
          isProcessing={false}
          isStopping={false}
          enableStreaming={false}
          showConfidence={true}
          disableAutoScroll={disableAutoScroll}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          totalCount={totalCount}
          loadedCount={loadedCount}
          onLoadMore={onLoadMore}
          speakerLabelsBySegmentId={speakerLabelsByTranscriptId}
          highlightedSegmentId={highlightedSegmentId}
        />
      </div>

      {/* Custom prompt input at bottom of transcript section */}
      {!isRecording && convertedSegments.length > 0 && (
        <div className="space-y-2 border-t border-slate-200/80 bg-white/95 p-3">
          {customPromptHistory.length > 0 && (
            <select
              className="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-xs font-medium text-slate-700 shadow-[0_1px_2px_rgba(15,23,42,0.04)] outline-none transition-[border-color,box-shadow] focus:border-emerald-700/50 focus:ring-2 focus:ring-emerald-700/15"
              value=""
              onChange={(e) => {
                if (e.target.value) {
                  onPromptChange(e.target.value);
                }
              }}
              aria-label="Previous summary context"
            >
              <option value="">Previous summary context...</option>
              {customPromptHistory.map((prompt) => (
                <option key={prompt} value={prompt}>
                  {prompt.replace(/\s+/g, ' ').length > 90
                    ? `${prompt.replace(/\s+/g, ' ').slice(0, 90)}...`
                    : prompt.replace(/\s+/g, ' ')}
                </option>
              ))}
            </select>
          )}
          <textarea
            placeholder="Add context for AI summary. For example people involved, meeting overview, objective etc..."
            aria-label="Additional context for AI summary"
            className="min-h-[88px] w-full resize-y rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm leading-6 text-slate-900 shadow-[0_1px_2px_rgba(15,23,42,0.04)] outline-none transition-[border-color,box-shadow] placeholder:text-slate-400 focus:border-emerald-700/50 focus:ring-2 focus:ring-emerald-700/15"
            value={customPrompt}
            onChange={(e) => onPromptChange(e.target.value)}
          />
        </div>
      )}
    </div>
  );
});
