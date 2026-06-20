"use client";

import { useEffect, useMemo, useRef, useState } from 'react';
import { AlertCircle, Bot, Clock3, FileText, Loader2, MessageCircle, RefreshCw, Send, Square, User } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { ModelConfig } from '@/components/ModelSettingsModal';
import {
  MeetingChatCitation,
  MeetingChatMessage,
  meetingChatService,
} from '@/services/meetingChatService';

const SUGGESTED_QUESTIONS = [
  'What follow-up actions came out of this meeting?',
  'What decisions did we make?',
  'What risks or blockers were mentioned?',
  'What did we say about the timeline?',
];

interface MeetingChatPanelProps {
  meetingId: string;
  meetingTitle: string;
  modelConfig: ModelConfig;
  transcriptCount: number;
}

type ChatDisplayMessage = MeetingChatMessage & {
  retryQuestion?: string;
};

function makeTempMessage(
  meetingId: string,
  role: 'user' | 'assistant',
  content: string,
  status: MeetingChatMessage['status'] = 'completed',
): ChatDisplayMessage {
  return {
    id: `temp-${role}-${crypto.randomUUID()}`,
    meetingId,
    role,
    content,
    status,
    provider: null,
    model: null,
    citations: [],
    error: null,
    createdAt: new Date().toISOString(),
  };
}

function withRetryQuestions(messages: MeetingChatMessage[]): ChatDisplayMessage[] {
  let lastUserQuestion: string | null = null;
  return messages.map((message) => {
    if (message.role === 'user') {
      lastUserQuestion = message.content;
      return message;
    }

    if (message.status === 'failed' && lastUserQuestion) {
      return {
        ...message,
        retryQuestion: lastUserQuestion,
      };
    }

    return message;
  });
}

function sourceLabel(citation: MeetingChatCitation) {
  switch (citation.sourceType) {
    case 'transcript':
      return citation.timestamp || citation.sourceLabel || 'Transcript';
    case 'summary':
      return citation.title || 'Summary';
    case 'action_item':
      return 'Action item';
    case 'key_point':
      return 'Key point';
    case 'note':
      return 'Note';
    case 'screenshot':
      return citation.sourceLabel || 'Screenshot';
    default:
      return citation.sourceLabel || citation.sourceType;
  }
}

function CitationChip({
  citation,
  selected,
  onSelect,
}: {
  citation: MeetingChatCitation;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`inline-flex max-w-full items-center gap-1 rounded-md border px-2 py-1 text-xs font-medium transition ${selected
        ? 'border-emerald-500 bg-emerald-50 text-emerald-800'
        : 'border-slate-200 bg-white text-slate-600 hover:border-slate-300 hover:bg-slate-50'
        }`}
      title={citation.excerpt}
    >
      {citation.sourceType === 'screenshot' ? <FileText size={13} /> : <Clock3 size={13} />}
      <span className="font-semibold">[{citation.id}]</span>
      <span className="truncate">{sourceLabel(citation)}</span>
    </button>
  );
}

function MessageBubble({
  message,
  selectedCitationKey,
  onCitationSelect,
  onRetry,
}: {
  message: ChatDisplayMessage;
  selectedCitationKey: string | null;
  onCitationSelect: (citationKey: string) => void;
  onRetry?: () => void;
}) {
  const isUser = message.role === 'user';
  const selectedCitation = message.citations.find((citation) => {
    const citationKey = `${message.id}:${citation.id}:${citation.sourceId}`;
    return citationKey === selectedCitationKey;
  });

  return (
    <div className={`flex gap-3 ${isUser ? 'justify-end' : 'justify-start'}`}>
      {!isUser && (
        <div className="mt-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-emerald-50 text-emerald-700">
          <Bot size={17} />
        </div>
      )}
      <div className={`min-w-0 max-w-[82%] rounded-xl border px-4 py-3 shadow-sm ${isUser
        ? 'border-slate-900 bg-slate-900 text-white'
        : message.status === 'failed'
          ? 'border-red-200 bg-red-50 text-slate-900'
          : 'border-slate-200 bg-white text-slate-900'
        }`}>
        <div className="whitespace-pre-wrap break-words text-sm leading-6">
          {message.content}
        </div>
        {message.status === 'pending' && (
          <div className="mt-3 flex items-center gap-2 text-xs text-slate-500">
            <Loader2 size={14} className="animate-spin" />
            Thinking through the meeting context
          </div>
        )}
        {message.error && (
          <div className="mt-3 flex items-start gap-2 rounded-md border border-red-200 bg-white px-3 py-2 text-xs text-red-700">
            <AlertCircle size={14} className="mt-0.5 shrink-0" />
            <span className="break-words">{message.error}</span>
          </div>
        )}
        {message.citations.length > 0 && (
          <div className="mt-3 flex flex-wrap gap-2">
            {message.citations.map((citation) => {
              const citationKey = `${message.id}:${citation.id}:${citation.sourceId}`;
              return (
                <CitationChip
                  key={citationKey}
                  citation={citation}
                  selected={selectedCitationKey === citationKey}
                  onSelect={() => onCitationSelect(citationKey)}
                />
              );
            })}
          </div>
        )}
        {selectedCitation && (
          <div className="mt-3 rounded-lg border border-slate-200 bg-slate-50 p-3 text-xs text-slate-700">
            <div className="mb-1 font-semibold text-slate-900">
              [{selectedCitation.id}] {sourceLabel(selectedCitation)}
            </div>
            <p className="whitespace-pre-wrap break-words leading-5">{selectedCitation.excerpt}</p>
            {selectedCitation.filePath && (
              <p className="mt-2 truncate text-slate-500">{selectedCitation.filePath}</p>
            )}
          </div>
        )}
        {message.status === 'failed' && onRetry && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="mt-3"
            onClick={onRetry}
          >
            <RefreshCw size={14} />
            Retry
          </Button>
        )}
      </div>
      {isUser && (
        <div className="mt-1 flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-slate-900 text-white">
          <User size={16} />
        </div>
      )}
    </div>
  );
}

export function MeetingChatPanel({
  meetingId,
  meetingTitle,
  modelConfig,
  transcriptCount,
}: MeetingChatPanelProps) {
  const [messages, setMessages] = useState<ChatDisplayMessage[]>([]);
  const [question, setQuestion] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isAsking, setIsAsking] = useState(false);
  const [selectedCitationKey, setSelectedCitationKey] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const mountedRef = useRef(true);
  const activeRequestRef = useRef<string | null>(null);
  const activeLoadRef = useRef<string | null>(null);

  const modelReady = Boolean(modelConfig.provider && modelConfig.model);
  const canAsk = modelReady && transcriptCount > 0 && question.trim().length > 0 && !isAsking;

  const modelLabel = useMemo(() => {
    if (!modelReady) return 'No AI model selected';
    return `${modelConfig.provider} / ${modelConfig.model}`;
  }, [modelConfig.model, modelConfig.provider, modelReady]);

  const loadMessages = async () => {
    const loadId = crypto.randomUUID();
    activeLoadRef.current = loadId;
    setIsLoading(true);
    try {
      const history = await meetingChatService.listMessages(meetingId);
      if (mountedRef.current && activeLoadRef.current === loadId) {
        setMessages(withRetryQuestions(history));
      }
    } catch (error) {
      console.error('Failed to load meeting chat:', error);
      if (mountedRef.current && activeLoadRef.current === loadId) {
        toast.error('Could not load meeting chat');
      }
    } finally {
      if (mountedRef.current && activeLoadRef.current === loadId) {
        setIsLoading(false);
      }
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    activeRequestRef.current = null;
    activeLoadRef.current = null;
    setMessages([]);
    setQuestion('');
    setIsAsking(false);
    setSelectedCitationKey(null);
    void loadMessages();

    return () => {
      mountedRef.current = false;
      activeRequestRef.current = null;
      activeLoadRef.current = null;
    };
  }, [meetingId]);

  useEffect(() => {
    scrollRef.current?.scrollTo({
      top: scrollRef.current.scrollHeight,
      behavior: 'smooth',
    });
  }, [messages.length, isAsking]);

  const askQuestion = async (value: string) => {
    const trimmed = value.trim();
    if (!trimmed || isAsking) return;
    if (!modelReady) {
      toast.error('Choose an AI model before using meeting chat');
      return;
    }
    if (transcriptCount === 0) {
      toast.error('This meeting needs transcript text before chat can answer');
      return;
    }

    const optimisticUser = makeTempMessage(meetingId, 'user', trimmed);
    const optimisticAssistant = makeTempMessage(meetingId, 'assistant', '', 'pending');
    optimisticAssistant.retryQuestion = trimmed;
    const requestId = crypto.randomUUID();
    activeRequestRef.current = requestId;
    setMessages((current) => [...current, optimisticUser, optimisticAssistant]);
    setQuestion('');
    setSelectedCitationKey(null);
    setIsAsking(true);

    try {
      const response = await meetingChatService.ask(meetingId, trimmed);
      if (!mountedRef.current || activeRequestRef.current !== requestId) {
        return;
      }
      setMessages((current) => [
        ...current.filter((message) => message.id !== optimisticUser.id && message.id !== optimisticAssistant.id),
        response.userMessage,
        {
          ...response.assistantMessage,
          retryQuestion: response.assistantMessage.status === 'failed' ? trimmed : undefined,
        },
      ]);
    } catch (error) {
      if (!mountedRef.current || activeRequestRef.current !== requestId) {
        return;
      }
      console.error('Meeting chat failed:', error);
      const message = error instanceof Error ? error.message : String(error);
      setMessages((current) => [
        ...current.filter((item) => item.id !== optimisticAssistant.id),
        {
          ...optimisticAssistant,
          content: 'I could not answer that question yet.',
          status: 'failed',
          error: message,
          retryQuestion: trimmed,
        },
      ]);
      toast.error('Meeting chat failed', { description: message });
    } finally {
      if (mountedRef.current && activeRequestRef.current === requestId) {
        activeRequestRef.current = null;
        setIsAsking(false);
      }
    }
  };

  const handleCancel = async () => {
    try {
      await meetingChatService.cancel(meetingId);
      activeRequestRef.current = null;
      setIsAsking(false);
      setMessages((current) => current.map((message) => (
        message.status === 'pending'
          ? {
            ...message,
            content: 'Meeting chat answer was canceled.',
            status: 'canceled',
            error: 'Canceled by user.',
          }
          : message
      )));
      toast.info('Stopping meeting chat answer');
    } catch (error) {
      console.error('Failed to cancel meeting chat:', error);
      toast.error('Could not cancel meeting chat');
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-slate-50/70">
      <div className="border-b border-slate-200 bg-white px-6 py-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <h2 className="flex items-center gap-2 text-base font-semibold text-slate-950">
              <MessageCircle size={18} className="text-emerald-700" />
              Meeting chat
            </h2>
            <p className="mt-1 truncate text-sm text-slate-500">{meetingTitle}</p>
          </div>
          <div className="rounded-md border border-slate-200 bg-white px-3 py-2 text-xs text-slate-600">
            {modelLabel}
          </div>
        </div>
      </div>

      <div ref={scrollRef} className="min-h-0 flex-1 space-y-5 overflow-y-auto overflow-x-hidden px-6 py-5">
        {isLoading ? (
          <div className="flex h-full items-center justify-center text-sm text-slate-500">
            <Loader2 size={18} className="mr-2 animate-spin" />
            Loading chat history
          </div>
        ) : messages.length === 0 ? (
          <div className="mx-auto flex h-full max-w-2xl flex-col justify-center">
            <div className="rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
              <div className="flex items-start gap-3">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-emerald-50 text-emerald-700">
                  <Bot size={20} />
                </div>
                <div>
                  <h3 className="text-sm font-semibold text-slate-950">Ask about this meeting</h3>
                  <p className="mt-1 text-sm leading-6 text-slate-600">
                    Chat uses this meeting&apos;s transcript, summary, actions, notes, and screenshots with source citations.
                  </p>
                </div>
              </div>
              <div className="mt-4 grid gap-2 sm:grid-cols-2">
                {SUGGESTED_QUESTIONS.map((suggestion) => (
                  <button
                    key={suggestion}
                    type="button"
                    className="rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-left text-sm text-slate-700 transition hover:border-emerald-300 hover:bg-emerald-50"
                    onClick={() => {
                      setQuestion(suggestion);
                      textareaRef.current?.focus();
                    }}
                  >
                    {suggestion}
                  </button>
                ))}
              </div>
            </div>
          </div>
        ) : (
          messages.map((message) => (
            <MessageBubble
              key={message.id}
              message={message}
              selectedCitationKey={selectedCitationKey}
              onCitationSelect={(citationKey) => setSelectedCitationKey((current) => current === citationKey ? null : citationKey)}
              onRetry={message.status === 'failed' && message.retryQuestion ? () => askQuestion(message.retryQuestion!) : undefined}
            />
          ))
        )}
      </div>

      <div className="border-t border-slate-200 bg-white p-4">
        {!modelReady && (
          <div className="mb-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800">
            Choose and save an AI model before asking meeting questions.
          </div>
        )}
        {transcriptCount === 0 && (
          <div className="mb-3 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 text-sm text-slate-600">
            This meeting does not have transcript text yet.
          </div>
        )}
        <div className="flex items-end gap-2">
          <Textarea
            ref={textareaRef}
            value={question}
            onChange={(event) => setQuestion(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                void askQuestion(question);
              }
            }}
            placeholder="Ask a follow-up question about this meeting..."
            aria-label="Ask a question about this meeting"
            className="min-h-[52px] max-h-40 resize-none rounded-lg border-slate-200 bg-white text-sm"
            disabled={isAsking}
          />
          {isAsking ? (
            <Button
              type="button"
              variant="outline"
              onClick={handleCancel}
              className="h-[52px] shrink-0"
              aria-label="Cancel meeting chat answer"
            >
              <Square size={16} />
            </Button>
          ) : (
            <Button
              type="button"
              onClick={() => void askQuestion(question)}
              disabled={!canAsk}
              className="h-[52px] shrink-0 bg-emerald-700 hover:bg-emerald-800"
              aria-label="Send meeting chat question"
            >
              <Send size={16} />
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
