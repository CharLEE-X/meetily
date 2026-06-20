"use client"

import { useEffect, useState } from "react"
import {
  addAgentWorkflowOutcomeLink,
  AGENT_WORKFLOW_RUNS_EVENT,
  AgentWorkflowOutcomeLink,
  AgentWorkflowRun,
  listAgentWorkflowRuns,
  removeAgentWorkflowOutcomeLink,
} from "@/services/agentWorkflowService"

interface AgentWorkflowRunsPanelProps {
  meetingId?: string
  limit?: number
}

function statusClass(status: AgentWorkflowRun["status"]): string {
  if (status === "completed" || status === "running") return "bg-emerald-100 text-emerald-800"
  if (status === "failed") return "bg-red-100 text-red-800"
  if (status === "waitingForApproval" || status === "fallbackReady") return "bg-amber-100 text-amber-800"
  return "bg-gray-200 text-gray-700"
}

function defaultLabel(type: AgentWorkflowOutcomeLink["type"]): string {
  if (type === "pullRequest") return "Pull request"
  if (type === "linear") return "Linear issue"
  if (type === "jira") return "Jira issue"
  if (type === "branch") return "Branch"
  if (type === "draft") return "Draft"
  return "Outcome"
}

export function AgentWorkflowRunsPanel({ meetingId, limit = 8 }: AgentWorkflowRunsPanelProps) {
  const [runs, setRuns] = useState(() => listAgentWorkflowRuns())
  const [linkInputs, setLinkInputs] = useState<Record<string, { type: AgentWorkflowOutcomeLink["type"]; label: string; url: string }>>({})
  const visibleRuns = runs
    .filter((run) => !meetingId || run.meetingId === meetingId)
    .slice(0, limit)

  useEffect(() => {
    const refreshRuns = () => setRuns(listAgentWorkflowRuns())
    window.addEventListener(AGENT_WORKFLOW_RUNS_EVENT, refreshRuns)
    return () => window.removeEventListener(AGENT_WORKFLOW_RUNS_EVENT, refreshRuns)
  }, [])

  const updateLinkInput = (runId: string, patch: Partial<{ type: AgentWorkflowOutcomeLink["type"]; label: string; url: string }>) => {
    setLinkInputs((current) => ({
      ...current,
      [runId]: {
        ...(current[runId] ?? { type: "pullRequest" as const, label: "", url: "" }),
        ...patch,
      },
    }))
  }

  const addLink = (runId: string) => {
    const input = linkInputs[runId]
    if (!input?.url.trim()) return
    const updated = addAgentWorkflowOutcomeLink(runId, {
      type: input.type,
      label: input.label.trim() || defaultLabel(input.type),
      url: input.url.trim(),
    })
    if (updated) {
      setRuns(listAgentWorkflowRuns())
      setLinkInputs((current) => ({ ...current, [runId]: { type: "pullRequest", label: "", url: "" } }))
    }
  }

  const removeLink = (runId: string, linkId: string) => {
    const updated = removeAgentWorkflowOutcomeLink(runId, linkId)
    if (updated) setRuns(listAgentWorkflowRuns())
  }

  return (
    <div className="space-y-3">
      {visibleRuns.length === 0 ? (
        <p className="text-sm text-gray-600">No agent workflow runs yet.</p>
      ) : visibleRuns.map((run) => {
        const input = linkInputs[run.id] ?? { type: "pullRequest", label: "", url: "" }
        return (
          <div key={run.id} className="rounded-lg border border-gray-200 bg-gray-50 p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <div className="font-medium text-gray-900">{run.meetingTitle}</div>
                <p className="mt-1 text-xs text-gray-500">
                  Created {new Date(run.createdAt).toLocaleString()} · Updated {new Date(run.updatedAt).toLocaleString()}
                </p>
              </div>
              <span className={`rounded-full px-2 py-1 text-xs ${statusClass(run.status)}`}>{run.status}</span>
            </div>
            <p className="mt-2 text-sm text-gray-600">{run.message}</p>
            <p className="mt-2 text-xs text-gray-500">
              Agent: {run.agent}; mode: {run.mode}; template: {run.promptTemplateId}; budget: {run.contextPackage.budgetPreset}
            </p>
            <p className="mt-1 text-xs text-gray-500">
              Sources: {run.contextPackage.includedSources.join(", ") || "none"}
            </p>
            {run.outcomeLinks.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-2">
                {run.outcomeLinks.map((link) => (
                  <span key={link.id} className="inline-flex items-center gap-2 rounded-full bg-white px-2 py-1 text-xs text-gray-700 ring-1 ring-gray-200">
                    <a className="text-blue-700 hover:underline" href={link.url} target="_blank" rel="noreferrer">{link.label}</a>
                    <button className="text-gray-400 hover:text-red-600" onClick={() => removeLink(run.id, link.id)}>remove</button>
                  </span>
                ))}
              </div>
            )}
            <div className="mt-3 grid gap-2 md:grid-cols-[140px_1fr_1fr_auto]">
              <select
                className="rounded-md border border-gray-300 bg-white px-2 py-2 text-xs"
                value={input.type}
                onChange={(event) => updateLinkInput(run.id, { type: event.target.value as AgentWorkflowOutcomeLink["type"] })}
              >
                <option value="pullRequest">PR</option>
                <option value="branch">Branch</option>
                <option value="linear">Linear</option>
                <option value="jira">Jira</option>
                <option value="draft">Draft</option>
                <option value="other">Other</option>
              </select>
              <input
                className="rounded-md border border-gray-300 px-2 py-2 text-xs"
                value={input.label}
                onChange={(event) => updateLinkInput(run.id, { label: event.target.value })}
                placeholder="Label"
              />
              <input
                className="rounded-md border border-gray-300 px-2 py-2 text-xs"
                value={input.url}
                onChange={(event) => updateLinkInput(run.id, { url: event.target.value })}
                placeholder="https://..."
              />
              <button className="rounded-md border border-gray-300 px-3 py-2 text-xs font-medium text-gray-700 hover:bg-white" onClick={() => addLink(run.id)}>
                Add
              </button>
            </div>
            {run.auditEvents.length > 0 && (
              <div className="mt-3 text-xs text-gray-500">
                Last audit: {run.auditEvents[run.auditEvents.length - 1].type} · {run.auditEvents[run.auditEvents.length - 1].message}
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}
