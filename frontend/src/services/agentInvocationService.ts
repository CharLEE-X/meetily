import {
  AgentTarget,
  PreparedAgentWorkflow,
  updateAgentWorkflowRun,
} from './agentWorkflowService';

export type AgentInvocationStatus = 'triggered' | 'fallbackReady' | 'failed';

export interface AgentInvocationResult {
  status: AgentInvocationStatus;
  message: string;
  prompt: string;
  directInvocationSupported: boolean;
}

interface AgentInvocationAdapter {
  agent: AgentTarget;
  canInvokeDirectly: boolean;
  invoke(prepared: PreparedAgentWorkflow): Promise<AgentInvocationResult>;
}

function fallbackResult(prepared: PreparedAgentWorkflow, message: string): AgentInvocationResult {
  const auditEvents = [
    ...prepared.run.auditEvents,
    {
      id: `audit-${Date.now()}`,
      timestamp: new Date().toISOString(),
      type: 'fallbackReady' as const,
      message,
    },
  ];
  updateAgentWorkflowRun(prepared.run.id, {
    status: 'fallbackReady',
    message,
    auditEvents,
  });
  return {
    status: 'fallbackReady',
    message,
    prompt: prepared.prompt,
    directInvocationSupported: false,
  };
}

const copyPromptAdapter = (agent: AgentTarget): AgentInvocationAdapter => ({
  agent,
  canInvokeDirectly: false,
  async invoke(prepared) {
    const label = agent === 'manual' ? 'Manual MCP client' : agent;
    return fallbackResult(
      prepared,
      `${label} direct invocation is not available in this release. Copy the prepared prompt into the agent.`
    );
  },
});

const adapters: Record<AgentTarget, AgentInvocationAdapter> = {
  codex: copyPromptAdapter('codex'),
  claude: copyPromptAdapter('claude'),
  cursor: copyPromptAdapter('cursor'),
  manual: copyPromptAdapter('manual'),
};

export async function triggerPreparedAgentWorkflow(
  prepared: PreparedAgentWorkflow
): Promise<AgentInvocationResult> {
  if (!prepared.canRun) {
    const message = prepared.reason ?? 'Agent workflow was not runnable.';
    updateAgentWorkflowRun(prepared.run.id, {
      status: 'failed',
      message,
      auditEvents: [
        ...prepared.run.auditEvents,
        {
          id: `audit-${Date.now()}`,
          timestamp: new Date().toISOString(),
          type: 'failed',
          message,
        },
      ],
    });
    return {
      status: 'failed',
      message: prepared.reason ?? 'Agent workflow was not runnable.',
      prompt: prepared.prompt,
      directInvocationSupported: false,
    };
  }

  const adapter = adapters[prepared.run.agent] ?? adapters.manual;
  if (!adapter.canInvokeDirectly) {
    return adapter.invoke(prepared);
  }

  try {
    const result = await adapter.invoke(prepared);
    updateAgentWorkflowRun(prepared.run.id, {
      status: result.status === 'triggered' ? 'running' : result.status,
      message: result.message,
    });
    return result;
  } catch (error) {
    const message = error instanceof Error
      ? `Direct invocation failed: ${error.message}. Copy the prepared prompt instead.`
      : 'Direct invocation failed. Copy the prepared prompt instead.';
    return fallbackResult(prepared, message);
  }
}

export function markAgentWorkflowPromptCopied(runId: string) {
  const updatedRun = updateAgentWorkflowRun(runId, {
    status: 'fallbackReady',
    message: 'Prepared prompt copied for manual agent handoff.',
  });
  if (!updatedRun) return;
  updateAgentWorkflowRun(runId, {
    auditEvents: [
      ...updatedRun.auditEvents,
      {
        id: `audit-${Date.now()}`,
        timestamp: new Date().toISOString(),
        type: 'copied',
        message: 'Prepared prompt copied for manual agent handoff.',
      },
    ],
  });
}
