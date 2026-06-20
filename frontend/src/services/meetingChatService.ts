import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';

export type MeetingChatRole = 'user' | 'assistant';
export type MeetingChatStatus = 'pending' | 'completed' | 'failed' | 'canceled';

export interface MeetingChatCitation {
  id: string;
  sourceType: 'transcript' | 'summary' | 'action_item' | 'key_point' | 'note' | 'screenshot' | string;
  sourceId: string;
  sourceLabel: string;
  transcriptId: string | null;
  timestamp: string;
  audioStartTime: number | null;
  audioEndTime: number | null;
  title: string | null;
  filePath: string | null;
  excerpt: string;
}

export interface MeetingChatMessage {
  id: string;
  meetingId: string;
  role: MeetingChatRole;
  content: string;
  status: MeetingChatStatus;
  provider: string | null;
  model: string | null;
  citations: MeetingChatCitation[];
  error: string | null;
  createdAt: string;
}

export interface AskMeetingChatResponse {
  userMessage: MeetingChatMessage;
  assistantMessage: MeetingChatMessage;
}

export interface MeetingChatIndexStatus {
  meetingId: string;
  itemCount: number;
  rebuilt: boolean;
}

export const meetingChatService = {
  async listMessages(meetingId: string): Promise<MeetingChatMessage[]> {
    if (!isTauriRuntime()) return [];
    return invoke<MeetingChatMessage[]>('meeting_chat_list_messages', { meetingId });
  },

  async ask(meetingId: string, question: string): Promise<AskMeetingChatResponse> {
    if (!isTauriRuntime()) {
      throw new Error('Meeting chat is available in the desktop app.');
    }
    return invoke<AskMeetingChatResponse>('meeting_chat_ask', {
      request: { meetingId, question },
    });
  },

  async cancel(meetingId?: string): Promise<void> {
    if (!isTauriRuntime()) return;
    await invoke('meeting_chat_cancel', meetingId ? { meetingId } : {});
  },

  async rebuildIndex(meetingId: string): Promise<MeetingChatIndexStatus | null> {
    if (!isTauriRuntime()) return null;
    return invoke<MeetingChatIndexStatus>('meeting_chat_rebuild_index', { meetingId });
  },
};
