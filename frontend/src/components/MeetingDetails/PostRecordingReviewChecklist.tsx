"use client";

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Bot, CalendarPlus, Camera, CheckCircle2, FileText, ListChecks, ListTodo, MessageSquareText, Users } from 'lucide-react';
import {
  completePostRecordingChecklistItem,
  getPostRecordingChecklistState,
  skipPostRecordingChecklist,
} from '@/services/postRecordingChecklistService';
import type {
  PostRecordingChecklistItemId,
  PostRecordingChecklistState,
} from '@/services/postRecordingChecklistService';
import { listMeetingScreenshots } from '@/services/screenshotService';
import { getSpeakerLabels } from '@/services/speakerService';
import { listAgentWorkflowRuns } from '@/services/agentWorkflowService';

interface PostRecordingReviewChecklistProps {
  meetingId: string;
  hasSummary: boolean;
}

interface ChecklistItem {
  id: PostRecordingChecklistItemId;
  label: string;
  detail: string;
  targetId?: string;
  icon: typeof Camera;
}

function scrollToTarget(targetId?: string) {
  if (!targetId) return;
  document.getElementById(targetId)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

export function PostRecordingReviewChecklist({
  meetingId,
  hasSummary,
}: PostRecordingReviewChecklistProps) {
  const [state, setState] = useState<PostRecordingChecklistState>(() => getPostRecordingChecklistState(meetingId));
  const [screenshotCount, setScreenshotCount] = useState(0);
  const [speakerSuggestionCount, setSpeakerSuggestionCount] = useState(0);
  const [agentRunCount, setAgentRunCount] = useState(0);

  const refreshContext = useCallback(async () => {
    const [screenshotsResult, speakerResult] = await Promise.allSettled([
      listMeetingScreenshots(meetingId),
      getSpeakerLabels(meetingId),
    ]);
    if (screenshotsResult.status === 'fulfilled') {
      setScreenshotCount(screenshotsResult.value.filter((item) => item.status !== 'deleted').length);
    }
    if (speakerResult.status === 'fulfilled') {
      setSpeakerSuggestionCount(
        (speakerResult.value.visualSuggestions?.length ?? 0) +
        speakerResult.value.labels.filter((label) => label.status !== 'confirmed').length
      );
    }
    setAgentRunCount(listAgentWorkflowRuns().filter((run) => run.meetingId === meetingId).length);
  }, [meetingId]);

  useEffect(() => {
    setState(getPostRecordingChecklistState(meetingId));
    void refreshContext();
    const interval = window.setInterval(() => {
      void refreshContext();
    }, 5000);
    const refreshOnFocus = () => {
      void refreshContext();
    };
    window.addEventListener('focus', refreshOnFocus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('focus', refreshOnFocus);
    };
  }, [meetingId, refreshContext]);

  const items = useMemo<ChecklistItem[]>(() => {
    const next: ChecklistItem[] = [];
    if (screenshotCount > 0) {
      next.push({
        id: 'screenshots',
        label: 'Review screenshots',
        detail: `${screenshotCount} screenshot${screenshotCount === 1 ? '' : 's'} available in the transcript column. Delete images or metadata before they are used in summaries, notes, or agent context.`,
        icon: Camera,
      });
    }
    if (speakerSuggestionCount > 0) {
      next.push({
        id: 'speakerLabels',
        label: 'Review speaker labels',
        detail: `${speakerSuggestionCount} generated speaker suggestion${speakerSuggestionCount === 1 ? '' : 's'} in the transcript column need confirmation or cleanup.`,
        icon: Users,
      });
    }
    if (hasSummary) {
      next.push(
        {
          id: 'summaryContext',
          label: 'Check summary context',
          detail: 'Confirm the summary uses the right context before exporting or handing it to agents.',
          icon: MessageSquareText,
        },
        {
          id: 'calendar',
          label: 'Review calendar link',
          detail: 'Calendar actions remain manual and only create or update Meetily-owned records after review.',
          targetId: 'calendar-review',
          icon: CalendarPlus,
        },
        {
          id: 'notes',
          label: 'Preview Apple Notes export',
          detail: 'Preview the destination and content before writing to Apple Notes.',
          targetId: 'notes-review',
          icon: FileText,
        },
        {
          id: 'reminders',
          label: 'Review follow-up reminders',
          detail: 'Edit, select, or discard reminder drafts before anything is created in Apple Reminders.',
          targetId: 'reminders-review',
          icon: ListTodo,
        },
        {
          id: 'agents',
          label: agentRunCount > 0 ? 'Review agent runs' : 'Prepare agent handoff',
          detail: 'Codex and Claude automations stay review-first; use this step to inspect runs or trigger one intentionally.',
          targetId: 'agent-review',
          icon: Bot,
        },
      );
    }
    return next;
  }, [agentRunCount, hasSummary, screenshotCount, speakerSuggestionCount]);

  const completed = new Set(state.completedItemIds);
  const remainingCount = items.filter((item) => !completed.has(item.id)).length;
  const shouldShow = items.length > 0 && !state.skipped && remainingCount > 0;

  if (!shouldShow) {
    return null;
  }

  const completeItem = (itemId: PostRecordingChecklistItemId) => {
    setState(completePostRecordingChecklistItem(meetingId, itemId));
  };

  const skipChecklist = () => {
    setState(skipPostRecordingChecklist(meetingId));
  };

  return (
    <section className="mx-6 mb-4 mt-2 rounded-2xl border border-emerald-200 bg-emerald-50/60 p-4 shadow-sm">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <div className="flex items-center gap-2 text-sm font-semibold text-emerald-950">
            <ListChecks className="h-4 w-4" />
            Post-recording review
          </div>
          <p className="mt-1 max-w-3xl text-sm leading-6 text-emerald-900">
            Review captured context before exporting, creating follow-ups, linking calendar records,
            or handing the meeting to an agent. This checklist stores progress locally per meeting.
          </p>
        </div>
        <button
          type="button"
          onClick={skipChecklist}
          className="rounded-md border border-emerald-200 bg-white px-3 py-2 text-sm font-medium text-emerald-800 hover:bg-emerald-50"
        >
          Skip checklist
        </button>
      </div>

      <div className="mt-4 grid gap-2">
        {items.map((item) => {
          const Icon = item.icon;
          const isDone = completed.has(item.id);
          return (
            <div key={item.id} className="flex flex-col gap-3 rounded-xl border border-emerald-100 bg-white p-3 sm:flex-row sm:items-center sm:justify-between">
              <div className="flex min-w-0 items-start gap-3">
                <div className={`rounded-lg p-2 ${isDone ? 'bg-emerald-100 text-emerald-700' : 'bg-slate-100 text-slate-600'}`}>
                  {isDone ? <CheckCircle2 className="h-4 w-4" /> : <Icon className="h-4 w-4" />}
                </div>
                <div className="min-w-0">
                  <div className="text-sm font-semibold text-slate-950">{item.label}</div>
                  <p className="mt-1 text-xs leading-5 text-slate-600">{item.detail}</p>
                </div>
              </div>
              <div className="flex shrink-0 flex-wrap gap-2">
                {item.targetId ? (
                  <button
                    type="button"
                    onClick={() => scrollToTarget(item.targetId)}
                    className="rounded-md border border-slate-200 bg-white px-3 py-2 text-xs font-medium text-slate-700 hover:bg-slate-50"
                  >
                    Go to review
                  </button>
                ) : null}
                <button
                  type="button"
                  onClick={() => completeItem(item.id)}
                  className="rounded-md bg-emerald-600 px-3 py-2 text-xs font-medium text-white hover:bg-emerald-700"
                >
                  Mark reviewed
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
