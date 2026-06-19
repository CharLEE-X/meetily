import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '@/lib/tauri';

export type MeetingChatRole = 'user' | 'assistant';
export type MeetingChatStatus = 'pending' | 'completed' | 'failed' | 'canceled';

export interface MeetingChatCitation {
  id: string;
  transcriptId: string;
  timestamp: string;
  audioStartTime: number | null;
  audioEndTime: number | null;
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

  async cancel(): Promise<void> {
    if (!isTauriRuntime()) return;
    await invoke('meeting_chat_cancel');
  },
};
