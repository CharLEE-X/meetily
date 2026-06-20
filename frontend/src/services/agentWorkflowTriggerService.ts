import { toast } from 'sonner';
import { Summary } from '@/types';
import { mcpService } from '@/services/mcpService';
import {
  getAgentWorkflowSettings,
  prepareAgentWorkflow,
} from '@/services/agentWorkflowService';
import {
  markAgentWorkflowPromptCopied,
  triggerPreparedAgentWorkflow,
} from '@/services/agentInvocationService';

interface TriggerMeetingAgentWorkflowOptions {
  meetingId: string;
  meetingTitle: string;
  templateId?: string | null;
  summary?: Summary | { markdown?: string } | null;
  source?: 'automatic-summary' | 'manual-summary' | 'manual-chat';
}

function copyPreparedPrompt(runId: string, prompt: string) {
  if (!navigator.clipboard) {
    toast.error('Clipboard is not available');
    return;
  }

  navigator.clipboard.writeText(prompt).then(
    () => {
      markAgentWorkflowPromptCopied(runId);
      toast.success('Agent handoff copied');
    },
    () => toast.error('Unable to copy agent handoff'),
  );
}

export async function triggerMeetingAgentWorkflow({
  meetingId,
  meetingTitle,
  templateId,
  summary = null,
  source = 'manual-summary',
}: TriggerMeetingAgentWorkflowOptions): Promise<boolean> {
  const settings = getAgentWorkflowSettings();
  const manual = source !== 'automatic-summary';

  if (settings.mode === 'off' || !settings.skillPackInstalled) {
    if (manual) {
      toast.info('Agent automation is not enabled', {
        description: 'Install the Meetily agent skill pack and choose a post-meeting mode in MCP settings.',
      });
    }
    return false;
  }

  try {
    const [mcpStatus, agentStatuses] = await Promise.all([
      mcpService.getStatus(),
      mcpService.getAgentStatuses(),
    ]);

    const prepared = prepareAgentWorkflow(
      {
        meetingId,
        meetingTitle: meetingTitle || 'Untitled meeting',
        templateId,
        summary,
        mcpUrl: mcpStatus.url,
      },
      mcpStatus,
      agentStatuses,
      settings,
    );

    if (!prepared.canRun) {
      toast.info(manual ? 'Agent automation could not run' : 'Post-meeting workflow skipped', {
        description: prepared.reason ?? 'Review agent workflow settings.',
      });
      return false;
    }

    const copyPrompt = () => copyPreparedPrompt(prepared.run.id, prepared.prompt);

    if (prepared.run.mode === 'auto') {
      const result = await triggerPreparedAgentWorkflow(prepared);
      toast.info(
        result.status === 'fallbackReady' ? 'Agent handoff fallback ready' : 'Agent automation triggered',
        {
          description: result.message,
          duration: 12000,
          action: result.prompt
            ? {
              label: 'Copy prompt',
              onClick: copyPrompt,
            }
            : undefined,
        },
      );
      return true;
    }

    toast.info(
      manual ? 'Agent automation ready' : 'Post-meeting workflow ready',
      {
        description: 'Review and copy the prepared agent prompt before running it.',
        duration: 12000,
        action: {
          label: 'Copy prompt',
          onClick: copyPrompt,
        },
      },
    );
    return true;
  } catch (error) {
    console.error('Failed to trigger agent automation:', error);
    toast.error('Agent automation failed', {
      description: error instanceof Error ? error.message : 'Review agent workflow settings.',
    });
    return false;
  }
}
