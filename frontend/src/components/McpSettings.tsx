"use client"

import { useCallback, useEffect, useState } from "react"
import { Bot, CheckCircle2, CircleAlert, CircleDashed, Play, Power, RefreshCw, ShieldCheck, Wrench } from "lucide-react"
import { Switch } from "./ui/switch"
import { mcpService, AgentKind, AgentSetupStatus, McpAuditEvent, McpClient, McpStatus } from "@/services/mcpService"

function statusText(status: McpStatus | null): string {
  if (!status) return "Loading"
  if (status.state === "running") return "Running"
  if (status.state === "starting") return "Starting"
  if (status.state === "error") return "Needs attention"
  return "Stopped"
}

function agentIcon(agent: AgentSetupStatus) {
  if (agent.working) return <CheckCircle2 className="h-4 w-4 text-emerald-600" />
  if (agent.configured) return <CircleDashed className="h-4 w-4 text-blue-600" />
  return <CircleAlert className="h-4 w-4 text-amber-600" />
}

function friendlyError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "The MCP setting could not be updated. Check the app permissions and try again."
}

export function McpSettings() {
  const [status, setStatus] = useState<McpStatus | null>(null)
  const [clients, setClients] = useState<McpClient[]>([])
  const [auditEvents, setAuditEvents] = useState<McpAuditEvent[]>([])
  const [agentStatuses, setAgentStatuses] = useState<AgentSetupStatus[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)
  const [message, setMessage] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    setIsLoading(true)
    setMessage(null)
    try {
      const [nextStatus, nextClients, nextAuditEvents, nextAgentStatuses] = await Promise.all([
        mcpService.getStatus(),
        mcpService.listClients(),
        mcpService.listAuditEvents(),
        mcpService.getAgentStatuses(),
      ])
      setStatus(nextStatus)
      setClients(nextClients)
      setAuditEvents(nextAuditEvents)
      setAgentStatuses(nextAgentStatuses)
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  const updateStatus = async (updater: (current: McpStatus) => McpStatus["settings"]) => {
    if (!status) return
    setIsSaving(true)
    setMessage(null)
    try {
      const nextStatus = await mcpService.updateSettings(updater(status))
      setStatus(nextStatus)
      const [nextAuditEvents, nextAgentStatuses] = await Promise.all([
        mcpService.listAuditEvents(),
        mcpService.getAgentStatuses(),
      ])
      setAuditEvents(nextAuditEvents)
      setAgentStatuses(nextAgentStatuses)
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsSaving(false)
    }
  }

  const handleEnabledChange = (enabled: boolean) => {
    updateStatus((current) => ({ ...current.settings, enabled }))
  }

  const handleAutoStartChange = (autoStart: boolean) => {
    updateStatus((current) => ({ ...current.settings, autoStart }))
  }

  const handlePortChange = (port: number) => {
    if (!Number.isFinite(port) || port < 1 || port > 65535) {
      setMessage("Choose a port between 1 and 65535.")
      return
    }
    updateStatus((current) => ({ ...current.settings, port }))
  }

  const handleSetupAgent = async (agent: AgentKind) => {
    setIsSaving(true)
    setMessage(null)
    try {
      await mcpService.setupAgent(agent)
      await refresh()
      setMessage("Agent configuration updated.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsSaving(false)
    }
  }

  const handleSetupAll = async () => {
    setIsSaving(true)
    setMessage(null)
    try {
      await mcpService.setupAllAgents()
      await refresh()
      setMessage("Agent configurations updated.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsSaving(false)
    }
  }

  const handleRevokeClient = async (clientId: string) => {
    setIsSaving(true)
    setMessage(null)
    try {
      const nextClients = await mcpService.revokeClient(clientId)
      setClients(nextClients)
      setMessage("Client access revoked.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsSaving(false)
    }
  }

  const currentStatus = statusText(status)
  const serverUrl = status?.url ?? `http://127.0.0.1:${status?.port ?? 43118}/mcp`

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className="flex items-center gap-2">
              <Bot className="h-5 w-5 text-blue-600" />
              <h3 className="text-lg font-semibold text-gray-900">Meetily MCP</h3>
            </div>
            <p className="mt-2 max-w-2xl text-sm text-gray-600">
              Local agent access is off by default. Enable it only for clients you trust on this machine.
            </p>
          </div>
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-gray-700">{currentStatus}</span>
            <Switch
              checked={Boolean(status?.settings.enabled)}
              disabled={!status || isSaving}
              onCheckedChange={handleEnabledChange}
            />
          </div>
        </div>

        <div className="mt-6 grid gap-4 md:grid-cols-3">
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <div className="flex items-center gap-2 text-sm font-medium text-gray-900">
              <Power className="h-4 w-4" />
              Server URL
            </div>
            <p className="mt-2 break-all font-mono text-xs text-gray-600">{serverUrl}</p>
          </div>
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <label className="text-sm font-medium text-gray-900" htmlFor="mcp-port">Port</label>
            <input
              id="mcp-port"
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              type="number"
              min={1}
              max={65535}
              value={status?.settings.port ?? 43118}
              disabled={!status || isSaving || status.settings.enabled}
              onChange={(event) => handlePortChange(Number(event.target.value))}
            />
          </div>
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="text-sm font-medium text-gray-900">Auto-start</div>
                <p className="mt-1 text-xs text-gray-600">Start MCP when Meetily opens.</p>
              </div>
              <Switch
                checked={Boolean(status?.settings.autoStart)}
                disabled={!status || isSaving}
                onCheckedChange={handleAutoStartChange}
              />
            </div>
          </div>
        </div>

        {status?.lastError && (
          <div className="mt-4 rounded-md border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
            {status.lastError}
          </div>
        )}
        {message && (
          <div className="mt-4 rounded-md border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-900">
            {message}
          </div>
        )}
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-900">Agent setup</h3>
            <p className="mt-1 text-sm text-gray-600">Configure Claude, Codex, and Cursor to use the local Meetily MCP endpoint.</p>
          </div>
          <div className="flex gap-2">
            <button
              className="inline-flex items-center gap-2 rounded-md border border-gray-300 px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
              onClick={refresh}
              disabled={isLoading || isSaving}
            >
              <RefreshCw className="h-4 w-4" />
              Refresh
            </button>
            <button
              className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              onClick={handleSetupAll}
              disabled={isLoading || isSaving}
            >
              <Wrench className="h-4 w-4" />
              Setup all
            </button>
          </div>
        </div>

        <div className="mt-5 grid gap-4 lg:grid-cols-3">
          {agentStatuses.map((agent) => (
            <div key={agent.agent} className="rounded-lg border border-gray-200 bg-gray-50 p-4">
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                  {agentIcon(agent)}
                  <div className="font-medium text-gray-900">{agent.label}</div>
                </div>
                <button
                  className="rounded-md border border-gray-300 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-white disabled:opacity-50"
                  onClick={() => handleSetupAgent(agent.agent)}
                  disabled={isSaving}
                >
                  {agent.configured ? "Repair" : "Setup"}
                </button>
              </div>
              <p className="mt-3 text-sm text-gray-600">{agent.message}</p>
              {agent.configPath && (
                <p className="mt-3 break-all font-mono text-xs text-gray-500">{agent.configPath}</p>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-5 w-5 text-blue-600" />
            <h3 className="text-lg font-semibold text-gray-900">Authorized clients</h3>
          </div>
          <div className="mt-4 space-y-3">
            {clients.length === 0 ? (
              <p className="text-sm text-gray-600">No clients have been authorized yet.</p>
            ) : (
              clients.map((client) => (
                <div key={client.id} className="rounded-lg border border-gray-200 bg-gray-50 p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="font-medium text-gray-900">{client.name}</div>
                      <p className="mt-1 text-xs text-gray-600">{client.scopes.join(", ")}</p>
                      <p className="mt-1 text-xs text-gray-500">{client.tokenFingerprint}</p>
                    </div>
                    <button
                      className="rounded-md border border-red-200 px-3 py-1.5 text-xs font-medium text-red-700 hover:bg-red-50"
                      onClick={() => handleRevokeClient(client.id)}
                    >
                      Revoke
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
          <h3 className="text-lg font-semibold text-gray-900">Audit log</h3>
          <div className="mt-4 space-y-3">
            {auditEvents.length === 0 ? (
              <p className="text-sm text-gray-600">No MCP access events yet.</p>
            ) : (
              auditEvents.slice(0, 8).map((event) => (
                <div key={event.id} className="rounded-lg border border-gray-200 bg-gray-50 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div className="font-medium text-gray-900">{event.toolName}</div>
                    <span className="rounded-full bg-gray-200 px-2 py-1 text-xs text-gray-700">{event.result}</span>
                  </div>
                  <p className="mt-2 text-xs text-gray-600">{new Date(event.timestamp).toLocaleString()}</p>
                  <p className="mt-1 text-xs text-gray-500">Client: {event.clientId}</p>
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      {isLoading && (
        <div className="flex items-center gap-2 text-sm text-gray-500">
          <Play className="h-4 w-4 animate-pulse" />
          Loading MCP settings...
        </div>
      )}
    </div>
  )
}
