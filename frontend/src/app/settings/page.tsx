'use client';

import React, { useState, useEffect, useLayoutEffect, useRef } from 'react';
import { ArrowLeft, Settings2, Mic, Database as DatabaseIcon, SparkleIcon, FlaskConical, Bot, CalendarDays, ListTodo, FileText, LayoutTemplate } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { TranscriptSettings } from '@/components/TranscriptSettings';
import { RecordingSettings } from '@/components/RecordingSettings';
import { PreferenceSettings } from '@/components/PreferenceSettings';
import { SummaryModelSettings } from '@/components/SummaryModelSettings';
import { BetaSettings } from '@/components/BetaSettings';
import { McpSettings } from '@/components/McpSettings';
import { CalendarSettings } from '@/components/CalendarSettings';
import { ReminderSettings } from '@/components/ReminderSettings';
import { AppleNotesSettings } from '@/components/AppleNotesSettings';
import { SummaryTemplateSettings } from '@/components/SummaryTemplateSettings';
import { useConfig } from '@/contexts/ConfigContext';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';

// Tabs configuration (constant)
const TABS = [
  { value: 'general', label: 'General', icon: Settings2 },
  { value: 'recording', label: 'Recordings', icon: Mic },
  { value: 'Transcriptionmodels', label: 'Transcription', icon: DatabaseIcon },
  { value: 'summaryModels', label: 'Summary', icon: SparkleIcon },
  { value: 'summaryTemplates', label: 'Templates', icon: LayoutTemplate },
  { value: 'mcp', label: 'MCP', icon: Bot },
  { value: 'calendar', label: 'Calendar', icon: CalendarDays },
  { value: 'notes', label: 'Notes', icon: FileText },
  { value: 'reminders', label: 'Reminders', icon: ListTodo },
  { value: 'beta', label: 'Beta', icon: FlaskConical }
] as const;

const TAB_EXPLANATIONS: Record<string, { title: string; description: string; points: string[] }> = {
  general: {
    title: 'General app behavior',
    description: 'Use this page for app-wide preferences that affect prompts, notifications, storage visibility, meeting detection, and analytics consent.',
    points: ['Notifications are local reminders around recording state.', 'Meeting detection uses approved local signals and never starts recording silently.', 'Storage and analytics controls help you understand where app data lives and what is shared.'],
  },
  recording: {
    title: 'Recording retention and capture inputs',
    description: 'Use this page to decide what Meetily keeps after a meeting and which audio devices it should prefer when recording starts.',
    points: ['Audio files and screenshots can contain sensitive meeting content.', 'Device defaults reduce setup time but can be changed before recording.', 'Screenshots support timeline context and speaker identification when enabled.'],
  },
  Transcriptionmodels: {
    title: 'Speech-to-text engine',
    description: 'Use this page to choose the model that turns meeting audio into transcript text and manage the local model files required for that engine.',
    points: ['Local models keep transcription on this Mac.', 'Parakeet is optimized for fast streaming transcripts.', 'Whisper remains available when you prefer its compatibility or accuracy profile.'],
  },
  summaryModels: {
    title: 'Summary and chat model behavior',
    description: 'Use this page to control automatic summaries, summary language, and the AI provider used for meeting summaries and meeting-aware chat.',
    points: ['Auto summary runs after recording stops when enabled.', 'Cloud providers may receive the selected meeting context.', 'Language settings affect generated summaries, not the recorded transcript.'],
  },
  summaryTemplates: {
    title: 'Summary templates',
    description: 'Use this page to create reusable summary structures for standups, project reviews, sales calls, retrospectives, or your own meeting rituals.',
    points: ['Built-in templates are protected, but can be duplicated into editable copies.', 'Custom templates can be imported and exported as JSON.', 'Per-meeting template selection is remembered from the summary toolbar.'],
  },
  mcp: {
    title: 'Local MCP access for agents',
    description: 'Use this page to expose approved Meetily meeting tools to local agents such as Codex, Claude, and Cursor, then configure post-meeting handoffs.',
    points: ['The MCP server listens locally on this machine.', 'Agent setup writes client configuration and verifies readiness.', 'Automation rules prepare reviewable handoffs for follow-up work.'],
  },
  calendar: {
    title: 'Apple Calendar integration',
    description: 'Use this page to sync upcoming calendar metadata, select the event for the next recording, and optionally create Meetily-owned calendar records.',
    points: ['Calendar sync reads local event metadata for better meeting titles and prompts.', 'Event creation is off by default.', 'Meetily only updates calendar events it created or linked.'],
  },
  notes: {
    title: 'Apple Notes exports',
    description: 'Use this page to choose where completed meeting summaries are written in Apple Notes and whether export should happen automatically.',
    points: ['Manual export remains available even when auto-export is off.', 'The first destination needs confirmation before automation runs.', 'Export history shows what Meetily created or updated.'],
  },
  reminders: {
    title: 'Apple Reminders follow-ups',
    description: 'Use this page to choose the default list for meeting action items and tune how developer-focused follow-up drafts are categorized.',
    points: ['Meetily proposes reminders from action items after meetings.', 'List and preset choices affect future drafts only.', 'Created reminders stay in Apple Reminders.'],
  },
  beta: {
    title: 'Experimental features',
    description: 'Use this page to opt into features that are useful enough to test but may still change before becoming part of the stable workflow.',
    points: ['Beta toggles are reversible.', 'Existing meetings are not deleted when a beta feature is disabled.', 'Enable these only when you are comfortable validating rougher flows.'],
  },
};

export default function SettingsPage() {
  const router = useRouter();
  const { transcriptModelConfig, setTranscriptModelConfig } = useConfig();

  // Animation state for tabs
  const [activeTab, setActiveTab] = useState('general');
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const [underlineStyle, setUnderlineStyle] = useState({ left: 0, width: 0 });
  const activeTabExplanation = TAB_EXPLANATIONS[activeTab] ?? TAB_EXPLANATIONS.general;

  // Load saved transcript configuration on mount
  useEffect(() => {
    const loadTranscriptConfig = async () => {
      try {
        const config = await invoke('api_get_transcript_config') as any;
        if (config) {
          console.log('Loaded saved transcript config:', config);
          setTranscriptModelConfig({
            provider: config.provider || 'localWhisper',
            model: config.model || 'large-v3',
            apiKey: config.apiKey || null
          });
        }
      } catch (error) {
        console.error('Failed to load transcript config:', error);
      }
    };
    loadTranscriptConfig();
  }, [setTranscriptModelConfig]);

  // Update underline position when active tab changes
  useLayoutEffect(() => {
    const activeIndex = TABS.findIndex(tab => tab.value === activeTab);
    const activeTabElement = tabRefs.current[activeIndex];

    if (activeTabElement) {
      const { offsetLeft, offsetWidth } = activeTabElement;
      setUnderlineStyle({ left: offsetLeft, width: offsetWidth });
      activeTabElement.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
  }, [activeTab]);

  return (
    <div className="h-screen bg-gray-50 flex flex-col">
      {/* Fixed Header */}
      <div className="sticky top-0 z-10 bg-gray-50 border-b border-gray-200">
        <div className="max-w-6xl mx-auto px-8 py-6">
          <div className="flex items-center gap-4">
            <button
              onClick={() => router.back()}
              className="flex items-center gap-2 text-gray-600 hover:text-gray-900 transition-colors"
            >
              <ArrowLeft className="w-5 h-5" />
              <span>Back</span>
            </button>
            <h1 className="text-3xl font-bold">Settings</h1>
          </div>
        </div>
      </div>

      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden">
        <div className="max-w-6xl mx-auto p-8 pt-6">
          {/* Tabs */}
          <Tabs value={activeTab} onValueChange={setActiveTab}>
            <div className="-mx-8 overflow-x-auto px-8">
              <TabsList className="relative flex h-auto min-w-full w-max justify-start rounded-none border-b border-gray-200 bg-transparent p-0">
                {TABS.map((tab, index) => {
                  const Icon = tab.icon;
                  return (
                    <TabsTrigger
                      key={tab.value}
                      value={tab.value}
                      ref={el => { tabRefs.current[index] = el }}
                      className="flex shrink-0 items-center gap-2 px-6 py-4 bg-transparent rounded-none border-0 data-[state=active]:bg-transparent data-[state=active]:text-blue-600 data-[state=active]:shadow-none text-gray-600 hover:text-gray-900 relative z-10"
                    >
                      <Icon className="w-4 h-4" />
                      {tab.label}
                    </TabsTrigger>
                  );
                })}

                <motion.div
                  className="absolute bottom-0 z-20 h-0.5 bg-blue-600"
                  layoutId="underline"
                  style={{ left: underlineStyle.left, width: underlineStyle.width }}
                  transition={{ type: 'spring', stiffness: 400, damping: 40 }}
                />
              </TabsList>
            </div>

            <div className="mt-6 rounded-lg border border-blue-100 bg-blue-50/70 p-5 text-blue-950">
              <h2 className="text-base font-semibold">{activeTabExplanation.title}</h2>
              <p className="mt-2 max-w-4xl text-sm leading-6 text-blue-900">
                {activeTabExplanation.description}
              </p>
              <ul className="mt-4 grid gap-2 text-sm text-blue-900 md:grid-cols-3">
                {activeTabExplanation.points.map((point) => (
                  <li key={point} className="rounded-md bg-white/70 px-3 py-2 ring-1 ring-blue-100">
                    {point}
                  </li>
                ))}
              </ul>
            </div>

            <TabsContent value="general">
              <PreferenceSettings />
            </TabsContent>
            <TabsContent value="recording">
              <RecordingSettings />
            </TabsContent>
            <TabsContent value="Transcriptionmodels">
              <TranscriptSettings
                transcriptModelConfig={transcriptModelConfig}
                setTranscriptModelConfig={setTranscriptModelConfig}
              />
            </TabsContent>
            <TabsContent value="summaryModels">
              <SummaryModelSettings />
            </TabsContent>
            <TabsContent value="summaryTemplates" className="mt-6">
              <SummaryTemplateSettings />
            </TabsContent>
            <TabsContent value="mcp" className="mt-6">
              <McpSettings />
            </TabsContent>
            <TabsContent value="calendar" className="mt-6">
              <CalendarSettings />
            </TabsContent>
            <TabsContent value="notes" className="mt-6">
              <AppleNotesSettings />
            </TabsContent>
            <TabsContent value="reminders" className="mt-6">
              <ReminderSettings />
            </TabsContent>
            <TabsContent value="beta" className="mt-6">
              <BetaSettings />
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
};
