"use client"

import { useCallback, useEffect, useState } from "react"
import { Bot, CheckCircle2, CircleAlert, CircleDashed, Play, Power, RefreshCw, ShieldCheck, Trash2, Wrench } from "lucide-react"
import { Switch } from "./ui/switch"
import { mcpService, AgentKind, AgentSetupStatus, McpAuditEvent, McpClient, McpStatus } from "@/services/mcpService"
import {
  AGENT_SUPPORT_MATRIX,
  AGENT_WORKFLOW_ACTIONS,
  AgentWorkflowRule,
  AgentTarget,
  AgentWorkflowRun,
  AgentWorkflowSettings,
  WorkflowActionId,
  WorkflowMode,
  getAgentWorkflowSettings,
  installMeetilySkillPack,
  listAgentWorkflowRuns,
  MEETILY_SKILL_PACK_VERSION,
  removeMeetilySkillPack,
  resolveAgentWorkflowRule,
  saveAgentWorkflowSettings,
} from "@/services/agentWorkflowService"
import { AgentContextBudgetPreset, AgentContextConsent } from "@/services/agentContextPackage"

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
  if (!agent.installed) return <CircleAlert className="h-4 w-4 text-gray-500" />
  return <CircleAlert className="h-4 w-4 text-amber-600" />
}

function agentStatusLabel(agent: AgentSetupStatus): string {
  if (agent.working) return "Working"
  if (agent.configured) return "Configured"
  if (agent.installed) return "Needs setup"
  return "Unavailable"
}

function agentStatusClass(agent: AgentSetupStatus): string {
  if (agent.working) return "bg-emerald-100 text-emerald-800"
  if (agent.configured) return "bg-blue-100 text-blue-800"
  if (agent.installed) return "bg-amber-100 text-amber-800"
  return "bg-gray-200 text-gray-700"
}

function friendlyError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "The MCP setting could not be updated. Check the app permissions and try again."
}

function resultClass(result: string): string {
  if (result === "allowed") return "bg-emerald-100 text-emerald-800"
  if (result === "revoked" || result === "denied") return "bg-red-100 text-red-800"
  if (result === "failed") return "bg-amber-100 text-amber-800"
  return "bg-gray-200 text-gray-700"
}

function clientStateLabel(client: McpClient): string {
  if (client.revoked) return "Revoked"
  const expiresAt = new Date(client.expiresAt)
  if (Number.isFinite(expiresAt.getTime()) && expiresAt.getTime() < Date.now()) return "Expired"
  return "Active"
}

function keywordText(keywords: string[]): string {
  return keywords.join(", ")
}

function parseKeywords(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)
}

export function McpSettings() {
  const [status, setStatus] = useState<McpStatus | null>(null)
  const [clients, setClients] = useState<McpClient[]>([])
  const [auditEvents, setAuditEvents] = useState<McpAuditEvent[]>([])
  const [agentStatuses, setAgentStatuses] = useState<AgentSetupStatus[]>([])
  const [workflowSettings, setWorkflowSettings] = useState<AgentWorkflowSettings>(() => getAgentWorkflowSettings())
  const [workflowRuns, setWorkflowRuns] = useState<AgentWorkflowRun[]>([])
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
      setWorkflowSettings(getAgentWorkflowSettings())
      setWorkflowRuns(listAgentWorkflowRuns())
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

  const updateWorkflowSettings = (updater: (current: AgentWorkflowSettings) => AgentWorkflowSettings) => {
    const nextSettings = saveAgentWorkflowSettings(updater(workflowSettings))
    setWorkflowSettings(nextSettings)
    setMessage("Agent workflow settings saved.")
  }

  const handleSkillPackInstall = () => {
    setWorkflowSettings(installMeetilySkillPack(workflowSettings))
    setMessage("Meetily agent skill pack installed.")
  }

  const handleSkillPackRemove = () => {
    setWorkflowSettings(removeMeetilySkillPack(workflowSettings))
    setMessage("Meetily agent skill pack removed and post-meeting workflows disabled.")
  }

  const handleDefaultAgentChange = (defaultAgent: AgentTarget) => {
    updateWorkflowSettings((current) => ({ ...current, defaultAgent }))
  }

  const handleWorkflowModeChange = (mode: WorkflowMode) => {
    updateWorkflowSettings((current) => ({ ...current, mode }))
  }

  const handleActionToggle = (actionId: WorkflowActionId, enabled: boolean) => {
    updateWorkflowSettings((current) => {
      const nextActions = enabled
        ? Array.from(new Set([...current.enabledActions, actionId]))
        : current.enabledActions.filter((id) => id !== actionId)
      return {
        ...current,
        enabledActions: nextActions.length ? nextActions : current.enabledActions,
      }
    })
  }

  const handleBudgetChange = (budgetPreset: AgentContextBudgetPreset) => {
    updateWorkflowSettings((current) => ({ ...current, budgetPreset }))
  }

  const handleConsentChange = (key: keyof AgentContextConsent, enabled: boolean) => {
    updateWorkflowSettings((current) => ({
      ...current,
      consent: {
        ...current.consent,
        [key]: enabled,
      },
    }))
  }

  const handleAddRule = () => {
    updateWorkflowSettings((current) => ({
      ...current,
      rules: [
        ...current.rules,
        {
          id: `rule-${Date.now()}`,
          name: "New automation rule",
          enabled: true,
          agent: current.defaultAgent,
          mode: current.mode === "off" ? "ask" : current.mode,
          enabledActions: current.enabledActions,
          budgetPreset: current.budgetPreset,
          templateId: "",
          match: {
            titleKeywords: [],
            calendarKeywords: [],
            projectKeywords: [],
            templateIds: [],
          },
          consent: current.consent,
        },
      ],
    }))
  }

  const updateRule = (ruleId: string, updater: (rule: AgentWorkflowRule) => AgentWorkflowRule) => {
    updateWorkflowSettings((current) => ({
      ...current,
      rules: current.rules.map((rule) => rule.id === ruleId ? updater(rule) : rule),
    }))
  }

  const removeRule = (ruleId: string) => {
    updateWorkflowSettings((current) => ({
      ...current,
      rules: current.rules.filter((rule) => rule.id !== ruleId),
    }))
  }

  const handleRevokeClient = async (clientId: string) => {
    setIsSaving(true)
    setMessage(null)
    try {
      const nextClients = await mcpService.revokeClient(clientId)
      setClients(nextClients)
      setAuditEvents(await mcpService.listAuditEvents())
      setMessage("Client access revoked.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsSaving(false)
    }
  }

  const currentStatus = statusText(status)
  const serverUrl = status?.url ?? `http://127.0.0.1:${status?.port ?? 43118}/mcp`
  const selectedAgentStatus = agentStatuses.find((agent) => agent.agent === workflowSettings.defaultAgent)
  const selectedAgentNeedsSetup = workflowSettings.defaultAgent !== "manual" && !selectedAgentStatus?.configured
  const selectedAgentNotReadyForAuto = workflowSettings.mode === "auto" && workflowSettings.defaultAgent !== "manual" && !selectedAgentStatus?.working
  const selectedAgentLabel = AGENT_SUPPORT_MATRIX.find((agent) => agent.agent === workflowSettings.defaultAgent)?.label ?? workflowSettings.defaultAgent
  const workflowPreview = resolveAgentWorkflowRule({
    meetingId: "preview",
    meetingTitle: "Project sync",
    templateId: "project-status",
    calendarText: "Engineering project sync",
    projectText: "Connected Mobility",
    summary: { markdown: "Preview summary" },
    mcpUrl: status?.url ?? null,
  }, status, agentStatuses, workflowSettings)

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
                  <div>
                    <div className="font-medium text-gray-900">{agent.label}</div>
                    <div className="mt-1 text-xs text-gray-500">Checked {new Date(agent.lastCheckedAt).toLocaleString()}</div>
                  </div>
                </div>
                <span className={`rounded-full px-2 py-0.5 text-xs ${agentStatusClass(agent)}`}>
                  {agentStatusLabel(agent)}
                </span>
              </div>
              <div className="mt-4 flex items-center justify-between gap-3">
                <div className="text-xs text-gray-600">
                  Invocation: {agent.invocationMode === "copyPrompt" ? "copy prompt fallback" : agent.invocationMode}
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
              {!agent.configured && (
                <p className="mt-2 text-xs text-gray-500">{agent.setupHint}</p>
              )}
              <div className="mt-3 flex flex-wrap gap-2">
                {agent.capabilities.map((capability) => (
                  <span key={capability} className="rounded-full bg-white px-2 py-1 text-xs text-gray-600 ring-1 ring-gray-200">
                    {capability}
                  </span>
                ))}
              </div>
              <p className="mt-3 text-xs text-gray-600">{agent.fallback}</p>
              {agent.configPath && (
                <p className="mt-3 break-all font-mono text-xs text-gray-500">{agent.configPath}</p>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-900">Meetily agent skill pack</h3>
            <p className="mt-1 max-w-2xl text-sm text-gray-600">
              Installs local workflow templates for meeting recall, daily and weekly digests, next-meeting prep, open-loop review, role briefs, Linear issue drafting, and manual agent handoff. The pack stores MCP references only; it does not embed meeting content or secrets.
            </p>
          </div>
          <div className="flex gap-2">
            <button
              className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              onClick={handleSkillPackInstall}
              disabled={isSaving}
            >
              <Wrench className="h-4 w-4" />
              {workflowSettings.skillPackInstalled ? "Update pack" : "Install pack"}
            </button>
            <button
              className="inline-flex items-center gap-2 rounded-md border border-red-200 px-3 py-2 text-sm font-medium text-red-700 hover:bg-red-50 disabled:opacity-50"
              onClick={handleSkillPackRemove}
              disabled={!workflowSettings.skillPackInstalled || isSaving}
            >
              <Trash2 className="h-4 w-4" />
              Remove
            </button>
          </div>
        </div>
        <div className="mt-4 grid gap-4 md:grid-cols-3">
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <div className="text-sm font-medium text-gray-900">Status</div>
            <p className="mt-2 text-sm text-gray-600">
              {workflowSettings.skillPackInstalled
                ? `Installed (${workflowSettings.skillPackVersion ?? MEETILY_SKILL_PACK_VERSION})`
                : "Not installed"}
            </p>
          </div>
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <label className="text-sm font-medium text-gray-900" htmlFor="default-agent">Default agent</label>
            <select
              id="default-agent"
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              value={workflowSettings.defaultAgent}
              onChange={(event) => handleDefaultAgentChange(event.target.value as AgentTarget)}
            >
              <option value="manual">Manual MCP handoff</option>
              <option value="codex">Codex</option>
              <option value="claude">Claude Desktop</option>
              <option value="cursor">Cursor</option>
            </select>
          </div>
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <label className="text-sm font-medium text-gray-900" htmlFor="workflow-mode">Post-meeting mode</label>
            <select
              id="workflow-mode"
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              value={workflowSettings.mode}
              onChange={(event) => handleWorkflowModeChange(event.target.value as WorkflowMode)}
              disabled={!workflowSettings.skillPackInstalled}
            >
              <option value="off">Off</option>
              <option value="ask">Ask before running</option>
              <option value="auto">Prepare handoff automatically</option>
            </select>
          </div>
        </div>

        <div className="mt-5 grid gap-3 md:grid-cols-2">
          {AGENT_WORKFLOW_ACTIONS.map((action) => {
            const checked = workflowSettings.enabledActions.includes(action.id)
            return (
              <label key={action.id} className="flex items-start gap-3 rounded-lg border border-gray-200 bg-gray-50 p-4">
                <input
                  type="checkbox"
                  className="mt-1 h-4 w-4 rounded border-gray-300"
                  checked={checked}
                  disabled={!workflowSettings.skillPackInstalled}
                  onChange={(event) => handleActionToggle(action.id, event.target.checked)}
                />
                <span>
                  <span className="flex flex-wrap items-center gap-2 text-sm font-medium text-gray-900">
                    {action.label}
                    {action.requiresApproval && (
                      <span className="rounded-full bg-amber-100 px-2 py-0.5 text-xs text-amber-800">approval required</span>
                    )}
                  </span>
                  <span className="mt-1 block text-sm text-gray-600">{action.description}</span>
                </span>
              </label>
            )
          })}
        </div>

        <div className="mt-5 grid gap-4 md:grid-cols-2">
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <label className="text-sm font-medium text-gray-900" htmlFor="context-budget">Context budget</label>
            <select
              id="context-budget"
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm"
              value={workflowSettings.budgetPreset}
              onChange={(event) => handleBudgetChange(event.target.value as AgentContextBudgetPreset)}
              disabled={!workflowSettings.skillPackInstalled}
            >
              <option value="minimal">Minimal</option>
              <option value="standard">Standard</option>
              <option value="detailed">Detailed</option>
            </select>
            <p className="mt-2 text-xs text-gray-600">Controls how much cited meeting context is sent to the agent handoff.</p>
          </div>
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <div className="text-sm font-medium text-gray-900">Rule preview</div>
            <p className="mt-2 text-sm text-gray-600">{workflowPreview.preview}</p>
            {workflowPreview.blockedReason && (
              <p className="mt-2 text-xs text-amber-800">{workflowPreview.blockedReason}</p>
            )}
          </div>
        </div>

        <div className="mt-5 rounded-lg border border-gray-200 bg-gray-50 p-4">
          <div className="text-sm font-medium text-gray-900">Allowed content sources</div>
          <div className="mt-3 grid gap-3 md:grid-cols-2">
            {([
              ["includeSummary", "Summary"],
              ["includeActionItems", "Action items"],
              ["includeDecisions", "Decisions"],
              ["includeRisks", "Risks"],
              ["includeTranscriptExcerpts", "Transcript excerpts"],
              ["includeScreenshotsOcr", "Screenshot OCR"],
              ["includeCalendarMetadata", "Calendar metadata"],
              ["includeArtifacts", "Artifact links"],
            ] as Array<[keyof AgentContextConsent, string]>).map(([key, label]) => (
              <label key={key} className="flex items-center gap-2 text-sm text-gray-700">
                <input
                  type="checkbox"
                  className="h-4 w-4 rounded border-gray-300"
                  checked={Boolean(workflowSettings.consent[key])}
                  disabled={!workflowSettings.skillPackInstalled}
                  onChange={(event) => handleConsentChange(key, event.target.checked)}
                />
                {label}
              </label>
            ))}
          </div>
        </div>

        <div className="mt-5 rounded-lg border border-gray-200 bg-gray-50 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="text-sm font-medium text-gray-900">Automation rules</div>
              <p className="mt-1 text-xs text-gray-600">First matching enabled rule overrides the global agent, mode, actions, budget, and sources.</p>
            </div>
            <button
              className="rounded-md border border-gray-300 px-3 py-1.5 text-xs font-medium text-gray-700 hover:bg-white disabled:opacity-50"
              onClick={handleAddRule}
              disabled={!workflowSettings.skillPackInstalled || isSaving}
            >
              Add rule
            </button>
          </div>
          <div className="mt-4 space-y-3">
            {workflowSettings.rules.length === 0 ? (
              <p className="text-sm text-gray-600">No custom rules yet. Global settings apply to every meeting.</p>
            ) : workflowSettings.rules.map((rule) => (
              <div key={rule.id} className="rounded-lg border border-gray-200 bg-white p-4">
                <div className="grid gap-3 md:grid-cols-4">
                  <label className="text-xs font-medium text-gray-700">
                    Name
                    <input
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={rule.name}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, name: event.target.value }))}
                    />
                  </label>
                  <label className="text-xs font-medium text-gray-700">
                    Agent
                    <select
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={rule.agent}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, agent: event.target.value as AgentTarget }))}
                    >
                      <option value="manual">Manual</option>
                      <option value="codex">Codex</option>
                      <option value="claude">Claude</option>
                    </select>
                  </label>
                  <label className="text-xs font-medium text-gray-700">
                    Mode
                    <select
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={rule.mode}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, mode: event.target.value as WorkflowMode }))}
                    >
                      <option value="off">Off</option>
                      <option value="ask">Ask</option>
                      <option value="auto">Auto</option>
                    </select>
                  </label>
                  <label className="text-xs font-medium text-gray-700">
                    Budget
                    <select
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={rule.budgetPreset}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, budgetPreset: event.target.value as AgentContextBudgetPreset }))}
                    >
                      <option value="minimal">Minimal</option>
                      <option value="standard">Standard</option>
                      <option value="detailed">Detailed</option>
                    </select>
                  </label>
                </div>
                <div className="mt-3 grid gap-3 md:grid-cols-4">
                  <label className="text-xs font-medium text-gray-700">
                    Title keywords
                    <input
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={keywordText(rule.match.titleKeywords)}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, match: { ...current.match, titleKeywords: parseKeywords(event.target.value) } }))}
                      placeholder="standup, sync"
                    />
                  </label>
                  <label className="text-xs font-medium text-gray-700">
                    Calendar keywords
                    <input
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={keywordText(rule.match.calendarKeywords)}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, match: { ...current.match, calendarKeywords: parseKeywords(event.target.value) } }))}
                      placeholder="engineering, customer"
                    />
                  </label>
                  <label className="text-xs font-medium text-gray-700">
                    Project keywords
                    <input
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={keywordText(rule.match.projectKeywords)}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, match: { ...current.match, projectKeywords: parseKeywords(event.target.value) } }))}
                      placeholder="mobility"
                    />
                  </label>
                  <label className="text-xs font-medium text-gray-700">
                    Template ids
                    <input
                      className="mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm"
                      value={keywordText(rule.match.templateIds)}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, match: { ...current.match, templateIds: parseKeywords(event.target.value) } }))}
                      placeholder="project-status"
                    />
                  </label>
                </div>
                <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
                  <label className="flex items-center gap-2 text-sm text-gray-700">
                    <input
                      type="checkbox"
                      className="h-4 w-4 rounded border-gray-300"
                      checked={rule.enabled}
                      onChange={(event) => updateRule(rule.id, (current) => ({ ...current, enabled: event.target.checked }))}
                    />
                    Enabled
                  </label>
                  <button
                    className="rounded-md border border-red-200 px-3 py-1.5 text-xs font-medium text-red-700 hover:bg-red-50"
                    onClick={() => removeRule(rule.id)}
                  >
                    Remove
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {selectedAgentNeedsSetup && (
          <div className="mt-5 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
            The selected default agent is not configured yet. Run Agent setup for {selectedAgentLabel} before enabling post-meeting handoffs for that target.
          </div>
        )}
        {selectedAgentNotReadyForAuto && (
          <div className="mt-5 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
            Auto mode is selected, but {selectedAgentLabel} is not working yet. Start Meetily MCP and confirm the agent readiness check before relying on automatic post-meeting handoffs.
          </div>
        )}

        <div className="mt-5 rounded-lg border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-900">
          External write workflows, including Linear issue creation, are proposal-only by default. Meetily prepares reviewable drafts and requires explicit approval before anything is created outside the app.
        </div>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="text-lg font-semibold text-gray-900">Agent support matrix</h3>
        <p className="mt-1 text-sm text-gray-600">
          Meetily can configure MCP for supported local agents, but direct task invocation depends on each client. Unsupported launch paths degrade to copyable handoff prompts.
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-gray-200">
          <table className="w-full table-fixed text-left text-sm">
            <thead className="bg-gray-50 text-xs uppercase text-gray-500">
              <tr>
                <th className="w-36 px-4 py-3">Agent</th>
                <th className="px-4 py-3">Setup</th>
                <th className="px-4 py-3">Invocation</th>
                <th className="px-4 py-3">Fallback</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200">
              {AGENT_SUPPORT_MATRIX.map((row) => (
                <tr key={row.agent} className="align-top">
                  <td className="px-4 py-3 font-medium text-gray-900">{row.label}</td>
                  <td className="px-4 py-3 text-gray-600">{row.setup}</td>
                  <td className="px-4 py-3 text-gray-600">{row.invocation}</td>
                  <td className="px-4 py-3 text-gray-600">{row.handoff}</td>
                </tr>
              ))}
            </tbody>
          </table>
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
              <p className="text-sm text-gray-600">No trusted clients yet. Use Agent setup to authorize Claude, Codex, or Cursor.</p>
            ) : (
              clients.map((client) => (
                <div key={client.id} className="rounded-lg border border-gray-200 bg-gray-50 p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="flex flex-wrap items-center gap-2">
                        <div className="font-medium text-gray-900">{client.name}</div>
                        <span className={`rounded-full px-2 py-0.5 text-xs ${client.revoked ? "bg-red-100 text-red-800" : "bg-emerald-100 text-emerald-800"}`}>
                          {clientStateLabel(client)}
                        </span>
                      </div>
                      <p className="mt-2 text-xs text-gray-600">Scopes: {client.scopes.join(", ")}</p>
                      <p className="mt-1 text-xs text-gray-500">Fingerprint: {client.tokenFingerprint}</p>
                      <p className="mt-1 text-xs text-gray-500">Expires: {new Date(client.expiresAt).toLocaleString()}</p>
                    </div>
                    <button
                      className="rounded-md border border-red-200 px-3 py-1.5 text-xs font-medium text-red-700 hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-50"
                      onClick={() => handleRevokeClient(client.id)}
                      disabled={client.revoked || isSaving}
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
                    <span className={`rounded-full px-2 py-1 text-xs ${resultClass(event.result)}`}>{event.result}</span>
                  </div>
                  <p className="mt-2 text-xs text-gray-600">{new Date(event.timestamp).toLocaleString()}</p>
                  <p className="mt-1 text-xs text-gray-500">Client: {event.clientId}</p>
                  {event.scopes.length > 0 && (
                    <p className="mt-1 text-xs text-gray-500">Scopes: {event.scopes.join(", ")}</p>
                  )}
                  {event.meetingIds.length > 0 && (
                    <p className="mt-1 break-all text-xs text-gray-500">Meetings: {event.meetingIds.join(", ")}</p>
                  )}
                  {event.reason && (
                    <p className="mt-1 text-xs text-gray-500">Reason: {event.reason}</p>
                  )}
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <h3 className="text-lg font-semibold text-gray-900">Post-meeting workflow log</h3>
        <div className="mt-4 space-y-3">
          {workflowRuns.length === 0 ? (
            <p className="text-sm text-gray-600">No agent workflow runs yet.</p>
          ) : (
            workflowRuns.slice(0, 6).map((run) => (
              <div key={run.id} className="rounded-lg border border-gray-200 bg-gray-50 p-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <div className="font-medium text-gray-900">{run.meetingTitle}</div>
                    <p className="mt-1 text-xs text-gray-500">{new Date(run.createdAt).toLocaleString()}</p>
                  </div>
                  <span className="rounded-full bg-gray-200 px-2 py-1 text-xs text-gray-700">{run.status}</span>
                </div>
                <p className="mt-2 text-sm text-gray-600">{run.message}</p>
                <p className="mt-1 text-xs text-gray-500">
                  Agent: {run.agent}; actions: {run.actions.join(", ")}
                </p>
              </div>
            ))
          )}
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
