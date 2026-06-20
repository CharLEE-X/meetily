"use client";

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Camera, EyeOff, FileX, RefreshCw, Trash2, Users } from 'lucide-react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import {
  getSpeakerLabels,
  runSpeakerLabeling,
  SpeakerLabel,
  TranscriptSpeakerSegment,
  updateSpeakerLabel,
} from '@/services/speakerService';
import {
  deleteMeetingScreenshot,
  listMeetingScreenshots,
  MeetingScreenshot,
} from '@/services/screenshotService';

interface SpeakerScreenshotPanelProps {
  meetingId: string;
  onSpeakerLabelsChange: (labelsByTranscriptId: Record<string, string>) => void;
}

export function SpeakerScreenshotPanel({
  meetingId,
  onSpeakerLabelsChange,
}: SpeakerScreenshotPanelProps) {
  const [labels, setLabels] = useState<SpeakerLabel[]>([]);
  const [segments, setSegments] = useState<TranscriptSpeakerSegment[]>([]);
  const [screenshots, setScreenshots] = useState<MeetingScreenshot[]>([]);
  const [loadingSpeakers, setLoadingSpeakers] = useState(false);
  const [savingLabelId, setSavingLabelId] = useState<string | null>(null);
  const [draftNames, setDraftNames] = useState<Record<string, string>>({});

  const labelsById = useMemo(() => {
    return labels.reduce<Record<string, SpeakerLabel>>((acc, label) => {
      acc[label.id] = label;
      return acc;
    }, {});
  }, [labels]);

  useEffect(() => {
    const labelsByTranscriptId = segments.reduce<Record<string, string>>((acc, segment) => {
      const label = labelsById[segment.speakerLabelId];
      if (label) {
        acc[segment.transcriptId] = label.displayName;
      }
      return acc;
    }, {});
    onSpeakerLabelsChange(labelsByTranscriptId);
  }, [segments, labelsById, onSpeakerLabelsChange]);

  const refreshSpeakers = useCallback(async () => {
    if (!meetingId) return;
    setLoadingSpeakers(true);
    try {
      const result = await getSpeakerLabels(meetingId);
      setLabels(result.labels);
      setSegments(result.segments);
      setDraftNames(
        result.labels.reduce<Record<string, string>>((acc, label) => {
          acc[label.id] = label.displayName;
          return acc;
        }, {})
      );
    } catch (error) {
      console.error('Failed to load speaker labels:', error);
    } finally {
      setLoadingSpeakers(false);
    }
  }, [meetingId]);

  const refreshScreenshots = useCallback(async () => {
    if (!meetingId) return;
    try {
      setScreenshots(await listMeetingScreenshots(meetingId));
    } catch (error) {
      console.error('Failed to load screenshots:', error);
    }
  }, [meetingId]);

  useEffect(() => {
    refreshSpeakers();
    refreshScreenshots();
  }, [refreshSpeakers, refreshScreenshots]);

  const handleRunSpeakerLabels = async () => {
    setLoadingSpeakers(true);
    try {
      const result = await runSpeakerLabeling(meetingId);
      setLabels(result.labels);
      setSegments(result.segments);
      setDraftNames(
        result.labels.reduce<Record<string, string>>((acc, label) => {
          acc[label.id] = label.displayName;
          return acc;
        }, {})
      );
      toast.success('Speaker labels updated');
    } catch (error) {
      console.error('Failed to run speaker labeling:', error);
      toast.error('Failed to update speaker labels');
    } finally {
      setLoadingSpeakers(false);
    }
  };

  const handleRenameSpeaker = async (label: SpeakerLabel) => {
    const nextName = draftNames[label.id]?.trim();
    if (!nextName || nextName === label.displayName) return;

    setSavingLabelId(label.id);
    try {
      const updated = await updateSpeakerLabel(label.id, nextName);
      setLabels((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setDraftNames((current) => ({ ...current, [updated.id]: updated.displayName }));
      toast.success('Speaker label saved');
    } catch (error) {
      console.error('Failed to rename speaker:', error);
      toast.error(String(error));
    } finally {
      setSavingLabelId(null);
    }
  };

  const handleRemoveScreenshotImage = async (screenshotId: string) => {
    try {
      await deleteMeetingScreenshot(screenshotId, true, false);
      await refreshScreenshots();
      toast.success('Screenshot image removed');
    } catch (error) {
      console.error('Failed to delete screenshot:', error);
      toast.error('Failed to delete screenshot');
    }
  };

  const handleRemoveScreenshotMetadata = async (screenshotId: string) => {
    try {
      await deleteMeetingScreenshot(screenshotId, true, true);
      setScreenshots((current) => current.filter((screenshot) => screenshot.id !== screenshotId));
      toast.success('Screenshot metadata removed');
    } catch (error) {
      console.error('Failed to remove screenshot metadata:', error);
      toast.error('Failed to remove screenshot metadata');
    }
  };

  const statusLabel = (screenshot: MeetingScreenshot) => {
    if (screenshot.status === 'captured') return 'Kept';
    if (screenshot.status === 'skipped') return 'Skipped';
    if (screenshot.status === 'deleted') return 'Deleted';
    return screenshot.status || 'Failed';
  };

  return (
    <div className="space-y-4 border-b border-slate-200/80 bg-[#f8faf8] px-4 py-3">
      <section>
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 text-sm font-semibold text-slate-900">
            <Users className="h-4 w-4 text-slate-500" />
            Speakers
          </div>
          <button
            type="button"
            onClick={handleRunSpeakerLabels}
            disabled={loadingSpeakers}
            className="inline-flex items-center gap-1 rounded-xl border border-slate-200 bg-white px-2.5 py-1.5 text-xs font-semibold text-slate-700 shadow-[0_1px_2px_rgba(15,23,42,0.04)] transition-colors hover:bg-slate-50 disabled:opacity-60"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loadingSpeakers ? 'animate-spin' : ''}`} />
            Detect
          </button>
        </div>
        <p className="mt-1 text-xs text-slate-500">
          Detected speaker labels are suggestions until you confirm or edit them.
        </p>

        {labels.length === 0 ? (
          <p className="mt-2 text-xs text-slate-500">No speaker labels yet.</p>
        ) : (
          <div className="mt-3 space-y-2">
            {labels.map((label) => (
              <div key={label.id} className="flex items-center gap-2">
                <input
                  value={draftNames[label.id] ?? label.displayName}
                  onChange={(event) =>
                    setDraftNames((current) => ({ ...current, [label.id]: event.target.value }))
                  }
                  onBlur={() => handleRenameSpeaker(label)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.currentTarget.blur();
                    }
                  }}
                  disabled={savingLabelId === label.id}
                  className="min-w-0 flex-1 rounded-xl border border-slate-200 bg-white px-2.5 py-1.5 text-xs text-slate-900 shadow-[0_1px_2px_rgba(15,23,42,0.04)] outline-none transition-[border-color,box-shadow] focus:border-emerald-700/50 focus:ring-2 focus:ring-emerald-700/15"
                />
                <span className="shrink-0 rounded-full border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-500">
                  {label.status}
                </span>
              </div>
            ))}
          </div>
        )}
      </section>

      <section>
        <div className="flex items-center gap-2 text-sm font-semibold text-slate-900">
          <Camera className="h-4 w-4 text-slate-500" />
          Screenshot Timeline
        </div>

        {screenshots.length === 0 ? (
          <p className="mt-2 text-xs text-slate-500">No screenshots captured for this meeting.</p>
        ) : (
          <div className="mt-3 flex gap-3 overflow-x-auto pb-1">
            {screenshots.map((screenshot) => (
              <div key={screenshot.id} className="w-44 shrink-0 overflow-hidden rounded-xl border border-slate-200 bg-white shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
                {screenshot.filePath && screenshot.status === 'captured' ? (
                  <img
                    src={convertFileSrc(screenshot.filePath)}
                    alt={screenshot.displayLabel ?? 'Meeting screenshot'}
                    className="h-24 w-full object-cover"
                  />
                ) : (
                  <div className={`flex h-24 flex-col justify-center gap-1 px-2 ${screenshot.status === 'deleted' ? 'bg-slate-100 text-slate-600' : 'bg-amber-50 text-amber-900'}`}>
                    <div className="flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wide">
                      <EyeOff className="h-3.5 w-3.5" />
                      {statusLabel(screenshot)}
                    </div>
                    <p
                      className="line-clamp-2 text-[11px] leading-snug"
                      title={screenshot.skipReason ?? 'No image payload is stored for this snapshot.'}
                    >
                      {screenshot.skipReason ?? 'No image payload is stored for this snapshot.'}
                    </p>
                  </div>
                )}
                <div className="space-y-1 px-2 py-1.5">
                  <div className="min-w-0">
                    <span className="block truncate text-xs font-semibold text-slate-700">
                      {screenshot.displayLabel ?? new Date(screenshot.capturedAt).toLocaleTimeString()}
                    </span>
                    <span className="block truncate text-[10px] text-slate-400">
                      {[
                        statusLabel(screenshot),
                        screenshot.captureTrigger,
                        screenshot.provider,
                        screenshot.relevanceStatus,
                        typeof screenshot.relevanceConfidence === 'number' ? `${Math.round(screenshot.relevanceConfidence * 100)}%` : null,
                      ]
                          .filter(Boolean)
                          .join(' · ')}
                    </span>
                    {screenshot.speakerEvidence ? (
                      <span className="mt-1 inline-flex rounded-full bg-emerald-50 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700">
                        Speaker evidence
                      </span>
                    ) : null}
                  </div>
                  <div className="flex items-center justify-end gap-1">
                    {screenshot.filePath ? (
                      <button
                        type="button"
                        onClick={() => handleRemoveScreenshotImage(screenshot.id)}
                        className="rounded-lg p-1 text-slate-400 transition-colors hover:bg-amber-50 hover:text-amber-700"
                        aria-label="Remove screenshot image but keep metadata"
                        title="Remove image but keep metadata"
                      >
                        <FileX className="h-3.5 w-3.5" />
                      </button>
                    ) : null}
                    <button
                      type="button"
                      onClick={() => handleRemoveScreenshotMetadata(screenshot.id)}
                      className="rounded-lg p-1 text-slate-400 transition-colors hover:bg-red-50 hover:text-red-600"
                      aria-label="Remove screenshot metadata"
                      title="Remove metadata"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
