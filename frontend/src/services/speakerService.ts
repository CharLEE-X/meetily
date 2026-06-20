import { invoke } from '@tauri-apps/api/core';
import { recordRecordingAuditEvent } from './recordingAuditService';

export interface SpeakerLabel {
  id: string;
  meetingId: string;
  displayName: string;
  source: string;
  status: string;
  confidence?: number | null;
}

export interface TranscriptSpeakerSegment {
  id: string;
  meetingId: string;
  transcriptId: string;
  speakerLabelId: string;
  startTime?: number | null;
  endTime?: number | null;
  source: string;
  confidence?: number | null;
}

export interface TranscriptSpeakerLabelView {
  displayName: string;
  status: string;
  source: string;
  confidence?: number | null;
}

export interface SpeakerLabelingResult {
  meetingId: string;
  labels: SpeakerLabel[];
  segments: TranscriptSpeakerSegment[];
  visualSuggestions: SpeakerLabelSuggestion[];
  strategy: string;
}

export interface SpeakerLabelSuggestion {
  transcriptId: string;
  displayName: string;
  confidence: number;
  startTime?: number | null;
  endTime?: number | null;
  source: string;
  snapshotId: string;
  provider?: string | null;
  activeMarker: string;
  autoApplied: boolean;
}

export interface SpeakerLabelingPreferences {
  autoApplyVisualSuggestions: boolean;
}

export async function runSpeakerLabeling(meetingId: string): Promise<SpeakerLabelingResult> {
  return invoke<SpeakerLabelingResult>('run_speaker_labeling', { meetingId });
}

export async function getSpeakerLabels(meetingId: string): Promise<SpeakerLabelingResult> {
  return invoke<SpeakerLabelingResult>('get_speaker_labels', { meetingId });
}

export async function clearSpeakerLabels(
  meetingId: string,
  includeConfirmed = false,
): Promise<void> {
  await invoke<void>('clear_speaker_labels', { meetingId, includeConfirmed });
  recordRecordingAuditEvent({
    type: 'speaker_labels_cleared',
    meetingId,
    actor: 'user',
    metadata: { includeConfirmed },
  });
}

export async function updateSpeakerLabel(
  labelId: string,
  displayName: string,
): Promise<SpeakerLabel> {
  return invoke<SpeakerLabel>('update_speaker_label', { labelId, displayName });
}

export async function acceptSpeakerLabel(labelId: string): Promise<SpeakerLabel> {
  return invoke<SpeakerLabel>('accept_speaker_label', { labelId });
}

export async function assignTranscriptSpeaker(
  meetingId: string,
  transcriptId: string,
  displayName: string,
): Promise<SpeakerLabelingResult> {
  return invoke<SpeakerLabelingResult>('assign_transcript_speaker', {
    meetingId,
    transcriptId,
    displayName,
  });
}

export async function mergeSpeakerLabels(
  sourceLabelId: string,
  targetLabelId: string,
): Promise<SpeakerLabelingResult> {
  return invoke<SpeakerLabelingResult>('merge_speaker_labels', { sourceLabelId, targetLabelId });
}

export async function undoLastSpeakerCorrection(meetingId: string): Promise<SpeakerLabelingResult> {
  return invoke<SpeakerLabelingResult>('undo_last_speaker_correction', { meetingId });
}

export async function getSpeakerLabelingPreferences(): Promise<SpeakerLabelingPreferences> {
  return invoke<SpeakerLabelingPreferences>('get_speaker_labeling_preferences');
}

export async function setSpeakerLabelingPreferences(
  preferences: SpeakerLabelingPreferences,
): Promise<SpeakerLabelingPreferences> {
  const saved = await invoke<SpeakerLabelingPreferences>('set_speaker_labeling_preferences', { preferences });
  recordRecordingAuditEvent({
    type: saved.autoApplyVisualSuggestions ? 'speaker_labeling_enabled' : 'speaker_labeling_disabled',
    actor: 'settings',
    metadata: {
      autoApplyVisualSuggestions: saved.autoApplyVisualSuggestions,
      reviewRequired: !saved.autoApplyVisualSuggestions,
    },
  });
  return saved;
}
