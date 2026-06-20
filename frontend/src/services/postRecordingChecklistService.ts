export type PostRecordingChecklistItemId =
  | 'screenshots'
  | 'speakerLabels'
  | 'summaryContext'
  | 'calendar'
  | 'notes'
  | 'reminders'
  | 'agents';

export interface PostRecordingChecklistState {
  meetingId: string;
  completedItemIds: PostRecordingChecklistItemId[];
  skipped: boolean;
  updatedAt: string;
}

const STORAGE_KEY = 'meetily.postRecordingChecklistState';

function emptyState(meetingId: string): PostRecordingChecklistState {
  return {
    meetingId,
    completedItemIds: [],
    skipped: false,
    updatedAt: new Date().toISOString(),
  };
}

function readAllStates(): Record<string, PostRecordingChecklistState> {
  if (typeof window === 'undefined') return {};
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
  } catch (error) {
    console.warn('Failed to read post-recording checklist state:', error);
    return {};
  }
}

function writeAllStates(states: Record<string, PostRecordingChecklistState>) {
  if (typeof window === 'undefined') return;
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(states));
}

export function getPostRecordingChecklistState(meetingId: string): PostRecordingChecklistState {
  return readAllStates()[meetingId] ?? emptyState(meetingId);
}

export function savePostRecordingChecklistState(
  meetingId: string,
  patch: Partial<Pick<PostRecordingChecklistState, 'completedItemIds' | 'skipped'>>,
) {
  const states = readAllStates();
  const current = states[meetingId] ?? emptyState(meetingId);
  const next: PostRecordingChecklistState = {
    ...current,
    ...patch,
    meetingId,
    completedItemIds: Array.from(new Set(patch.completedItemIds ?? current.completedItemIds)),
    updatedAt: new Date().toISOString(),
  };
  states[meetingId] = next;
  writeAllStates(states);
  return next;
}

export function completePostRecordingChecklistItem(
  meetingId: string,
  itemId: PostRecordingChecklistItemId,
) {
  const current = getPostRecordingChecklistState(meetingId);
  return savePostRecordingChecklistState(meetingId, {
    completedItemIds: [...current.completedItemIds, itemId],
  });
}

export function skipPostRecordingChecklist(meetingId: string) {
  return savePostRecordingChecklistState(meetingId, { skipped: true });
}
