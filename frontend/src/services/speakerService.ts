import { invoke } from '@tauri-apps/api/core';

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

export interface SpeakerLabelingResult {
  meetingId: string;
  labels: SpeakerLabel[];
  segments: TranscriptSpeakerSegment[];
  strategy: string;
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
  return invoke<void>('clear_speaker_labels', { meetingId, includeConfirmed });
}

export async function updateSpeakerLabel(
  labelId: string,
  displayName: string,
): Promise<SpeakerLabel> {
  return invoke<SpeakerLabel>('update_speaker_label', { labelId, displayName });
}
