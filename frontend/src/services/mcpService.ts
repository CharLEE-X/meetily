import { invoke } from '@tauri-apps/api/core';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export type McpServerState = 'stopped' | 'starting' | 'running' | 'error';
export type AgentKind = 'claude' | 'codex' | 'cursor';
export type AgentStatusValue = 'notInstalled' | 'notConfigured' | 'configured' | 'working';

export interface McpSettings {
  enabled: boolean;
  autoStart: boolean;
  port: number;
}

export interface McpStatus {
  settings: McpSettings;
  state: McpServerState;
  bindHost: string;
  port: number;
  url: string | null;
  lastError: string | null;
}

export interface McpClient {
  id: string;
  name: string;
  scopes: string[];
  tokenFingerprint: string;
  expiresAt: string;
  revoked: boolean;
}

export interface McpAuditEvent {
  id: string;
  timestamp: string;
  clientId: string;
  toolName: string;
  scopes: string[];
  meetingIds: string[];
  result: string;
  reason: string | null;
}

export interface AgentSetupStatus {
  agent: AgentKind;
  label: string;
  configPath: string;
  installed: boolean;
  configured: boolean;
  working: boolean;
  status: AgentStatusValue;
  lastCheckedAt: string;
  message: string;
}

const defaultStatus: McpStatus = {
  settings: {
    enabled: false,
    autoStart: false,
    port: 43118,
  },
  state: 'stopped',
  bindHost: '127.0.0.1',
  port: 43118,
  url: null,
  lastError: null,
};

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && Boolean(window.__TAURI_INTERNALS__);
}

export const mcpService = {
  async getStatus(): Promise<McpStatus> {
    if (!isTauriRuntime()) return defaultStatus;
    return invoke<McpStatus>('mcp_get_status');
  },

  async updateSettings(settings: McpSettings): Promise<McpStatus> {
    if (!isTauriRuntime()) return { ...defaultStatus, settings };
    return invoke<McpStatus>('mcp_update_settings', { settings });
  },

  async startServer(): Promise<McpStatus> {
    if (!isTauriRuntime()) return { ...defaultStatus, state: 'running', url: 'http://127.0.0.1:43118/mcp' };
    return invoke<McpStatus>('mcp_start_server');
  },

  async stopServer(): Promise<McpStatus> {
    if (!isTauriRuntime()) return defaultStatus;
    return invoke<McpStatus>('mcp_stop_server');
  },

  async listClients(): Promise<McpClient[]> {
    if (!isTauriRuntime()) return [];
    return invoke<McpClient[]>('mcp_list_clients');
  },

  async revokeClient(clientId: string): Promise<McpClient[]> {
    if (!isTauriRuntime()) return [];
    return invoke<McpClient[]>('mcp_revoke_client', { clientId });
  },

  async listAuditEvents(): Promise<McpAuditEvent[]> {
    if (!isTauriRuntime()) return [];
    return invoke<McpAuditEvent[]>('mcp_list_audit_events');
  },

  async getAgentStatuses(): Promise<AgentSetupStatus[]> {
    if (!isTauriRuntime()) {
      return ['claude', 'codex', 'cursor'].map((agent) => ({
        agent: agent as AgentKind,
        label: agent === 'claude' ? 'Claude' : agent === 'codex' ? 'Codex' : 'Cursor',
        configPath: '',
        installed: false,
        configured: false,
        working: false,
        status: 'notInstalled' as AgentStatusValue,
        lastCheckedAt: new Date().toISOString(),
        message: 'Open the desktop app to inspect local agent configuration.',
      }));
    }
    return invoke<AgentSetupStatus[]>('mcp_get_agent_statuses');
  },

  async setupAgent(agent: AgentKind): Promise<AgentSetupStatus> {
    if (!isTauriRuntime()) throw new Error('Agent setup is available in the desktop app.');
    return invoke<AgentSetupStatus>('mcp_setup_agent', { agent });
  },

  async setupAllAgents(): Promise<AgentSetupStatus[]> {
    if (!isTauriRuntime()) throw new Error('Agent setup is available in the desktop app.');
    return invoke<AgentSetupStatus[]>('mcp_setup_all_agents');
  },
};
