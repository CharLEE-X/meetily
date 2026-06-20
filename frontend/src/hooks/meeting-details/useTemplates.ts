import { useState, useEffect, useCallback } from 'react';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import Analytics from '@/lib/analytics';
import { SUMMARY_TEMPLATES_CHANGED_EVENT } from '@/services/summaryTemplateService';

function selectedTemplateStorageKey(meetingId?: string | null) {
  return meetingId ? `meetily:selected-summary-template:${meetingId}` : 'meetily:selected-summary-template';
}

export function useTemplates(meetingId?: string | null) {
  const [availableTemplates, setAvailableTemplates] = useState<Array<{
    id: string;
    name: string;
    description: string;
  }>>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string>(() => {
    if (typeof window === 'undefined') return 'standard_meeting';
    return window.localStorage.getItem(selectedTemplateStorageKey(meetingId)) || 'standard_meeting';
  });

  // Fetch available templates on mount
  useEffect(() => {
    const fetchTemplates = async () => {
      try {
        const templates = await invokeTauri('api_list_templates') as Array<{
          id: string;
          name: string;
          description: string;
        }>;
        console.log('Available templates:', templates);
        setAvailableTemplates(templates);
        setSelectedTemplate((current) => {
          const stored = window.localStorage.getItem(selectedTemplateStorageKey(meetingId));
          const next = stored || current || 'standard_meeting';
          return templates.some((template) => template.id === next)
            ? next
            : templates[0]?.id ?? 'standard_meeting';
        });
      } catch (error) {
        console.error('Failed to fetch templates:', error);
      }
    };
    fetchTemplates();
    window.addEventListener(SUMMARY_TEMPLATES_CHANGED_EVENT, fetchTemplates);
    return () => window.removeEventListener(SUMMARY_TEMPLATES_CHANGED_EVENT, fetchTemplates);
  }, [meetingId]);

  // Handle template selection
  const handleTemplateSelection = useCallback((templateId: string, templateName: string) => {
    setSelectedTemplate(templateId);
    window.localStorage.setItem(selectedTemplateStorageKey(meetingId), templateId);
    toast.success('Template selected', {
      description: `Using "${templateName}" template for summary generation`,
    });
    Analytics.trackFeatureUsed('template_selected');
  }, [meetingId]);

  return {
    availableTemplates,
    selectedTemplate,
    handleTemplateSelection,
  };
}
