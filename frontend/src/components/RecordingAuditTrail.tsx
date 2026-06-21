'use client';

import { useEffect, useMemo, useState } from 'react';
import { ShieldCheck } from 'lucide-react';
import {
  listRecordingAuditEvents,
  RecordingAuditEvent,
  subscribeToRecordingAuditEvents,
} from '@/services/recordingAuditService';

interface RecordingAuditTrailProps {
  meetingId?: string;
  limit?: number;
  compact?: boolean;
}

const EVENT_LABELS: Record<string, string> = {
  recording_preflight_shown: 'Recording preflight shown',
  recording_started_with_scope: 'Recording started',
  screenshot_capture_enabled: 'Screenshot capture enabled',
  screenshot_capture_disabled: 'Screenshot capture disabled',
  screenshot_capture_started: 'Screenshot capture started',
  screenshot_capture_stopped: 'Screenshot capture stopped',
  screenshot_capture_paused: 'Screenshot capture paused',
  screenshot_capture_resumed: 'Screenshot capture resumed',
  screenshot_capture_triggered: 'Speech-event screenshot requested',
  screenshot_images_deleted: 'Screenshot content removed',
  speaker_labeling_enabled: 'Visual speaker labels enabled',
  speaker_labeling_disabled: 'Visual speaker labels disabled',
  speaker_labels_cleared: 'Speaker labels cleared',
  calendar_context_attached: 'Calendar context attached',
  calendar_context_detached: 'Calendar context detached',
  notes_export_reviewed: 'Notes export reviewed',
  reminders_reviewed: 'Reminders reviewed',
  agent_automation_reviewed: 'Agent automation reviewed',
  agent_automation_disabled: 'Agent automation disabled',
  sensitive_capture_stopped: 'Sensitive capture stopped',
};

function formatEventTime(timestamp: string) {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return timestamp;
  }
  return date.toLocaleString([], {
    day: '2-digit',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function formatMetadata(event: RecordingAuditEvent) {
  const entries = Object.entries(event.metadata).filter(([, value]) => value !== null);
  if (entries.length === 0) {
    return 'No sensitive content stored';
  }

  return entries
    .map(([key, value]) => `${key}: ${Array.isArray(value) ? value.join(', ') : String(value)}`)
    .join(' · ');
}

export function RecordingAuditTrail({
  meetingId,
  limit = 8,
  compact = false,
}: RecordingAuditTrailProps) {
  const [events, setEvents] = useState<RecordingAuditEvent[]>([]);

  useEffect(() => {
    const refreshEvents = () => {
      setEvents(listRecordingAuditEvents({ meetingId, limit }));
    };

    refreshEvents();
    return subscribeToRecordingAuditEvents(refreshEvents);
  }, [limit, meetingId]);

  const title = useMemo(
    () => (meetingId ? 'Recording privacy history' : 'Recent recording privacy history'),
    [meetingId],
  );

  return (
    <section className={`rounded-2xl border border-slate-200 bg-white ${compact ? 'p-4' : 'p-5'} shadow-[0_1px_2px_rgba(15,23,42,0.04)]`}>
      <div className="flex items-start gap-3">
        <div className="rounded-xl bg-emerald-50 p-2 text-emerald-700">
          <ShieldCheck className="h-4 w-4" />
        </div>
        <div>
          <h3 className="text-sm font-semibold text-slate-950">{title}</h3>
          <p className="mt-1 text-xs leading-5 text-slate-500">
            Local audit entries for sensitive recording decisions. RecallX stores event type,
            time, source, and safe metadata only, never transcript text, screenshot images,
            tokens, or private calendar descriptions.
          </p>
        </div>
      </div>

      <div className="mt-4 divide-y divide-slate-100 overflow-hidden rounded-xl border border-slate-100">
        {events.length === 0 ? (
          <div className="px-4 py-5 text-sm text-slate-500">
            No sensitive recording decisions have been recorded yet.
          </div>
        ) : events.map((event) => (
          <div key={event.id} className="bg-white px-4 py-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="text-sm font-medium text-slate-900">
                {EVENT_LABELS[event.type] ?? event.type}
              </div>
              <div className="text-xs text-slate-500">{formatEventTime(event.timestamp)}</div>
            </div>
            <div className="mt-1 text-xs text-slate-500">
              {event.actor} · {formatMetadata(event)}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
