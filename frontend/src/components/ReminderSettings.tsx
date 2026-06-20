"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { CheckCircle2, CircleAlert, ListTodo, Loader2, PlugZap, RefreshCw, SlidersHorizontal, Trash2 } from "lucide-react"
import { reminderService, CreatedReminderLink, ReminderList, ReminderProviderAccount, ReminderSettingsState, ReminderListSyncResult, ReminderWorkflowPreset } from "@/services/reminderService"

function friendlyError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "Apple Reminders settings could not be updated. Check app permissions and try again."
}

function formatDateTime(value?: string | null): string {
  if (!value) return "Not synced yet"
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) return "Not synced yet"
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    hour: "numeric",
    minute: "2-digit",
    month: "short",
    day: "numeric",
  }).format(date)
}

function statusLabel(account?: ReminderProviderAccount): string {
  if (!account) return "Not connected"
  if (account.status === "connected") return "Connected"
  if (account.status === "permission_needed") return "Needs permission"
  if (account.status === "revoked") return "Disconnected"
  return "Needs attention"
}

function statusClass(account?: ReminderProviderAccount): string {
  if (account?.status === "connected") return "bg-emerald-100 text-emerald-800"
  if (account?.status === "permission_needed") return "bg-amber-100 text-amber-800"
  if (account?.status === "revoked") return "bg-gray-100 text-gray-700"
  return "bg-red-100 text-red-800"
}

const CATEGORY_LABELS: Record<string, string> = {
  pr_review: "PR review",
  linear_follow_up: "Linear follow-up",
  deploy_alert_check: "Deploy or alert check",
  docs_update: "Docs update",
  implementation_task: "Implementation task",
  experiment_revisit: "Experiment revisit",
  clarification_follow_up: "Clarification follow-up",
}

const DUE_PRESET_LABELS: Record<string, string> = {
  none: "No automatic due date",
  in_2_hours: "In 2 hours",
  tomorrow_morning: "Tomorrow morning",
  in_2_days: "In 2 days",
  next_week: "Next week",
}

function priorityLabel(priority?: number | null): string {
  if (!priority) return "Global"
  if (priority <= 3) return "High"
  if (priority <= 6) return "Medium"
  return "Low"
}

function formatDate(value?: string | null): string {
  if (!value) return "No due date"
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) return "No due date"
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date)
}

function statusClassName(status: string): string {
  if (status === "completed") return "bg-emerald-100 text-emerald-800"
  if (status === "open") return "bg-blue-100 text-blue-800"
  if (status === "missing") return "bg-amber-100 text-amber-800"
  return "bg-gray-100 text-gray-700"
}

export function ReminderSettings() {
  const [settings, setSettings] = useState<ReminderSettingsState | null>(null)
  const [syncResult, setSyncResult] = useState<ReminderListSyncResult | null>(null)
  const [recentReminders, setRecentReminders] = useState<CreatedReminderLink[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<"connect" | "sync" | "disconnect" | "default" | "workflow" | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  const appleAccount = useMemo(
    () => settings?.accounts.find((account) => account.provider === "apple_reminders"),
    [settings],
  )

  const appleProvider = useMemo(
    () => settings?.providers.find((provider) => provider.provider === "apple_reminders"),
    [settings],
  )

  const appleLists = useMemo(
    () => settings?.lists.filter((list) => list.providerAccountId === appleAccount?.id && list.selected) ?? [],
    [appleAccount?.id, settings],
  )

  const refresh = useCallback(async () => {
    setIsLoading(true)
    setMessage(null)
    try {
      const [nextSettings, nextRecentReminders] = await Promise.all([
        reminderService.getSettings(),
        reminderService.listRecentCreated(10),
      ])
      setSettings(nextSettings)
      setRecentReminders(nextRecentReminders)
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  const handleConnect = async () => {
    setActionLoading("connect")
    setMessage(null)
    try {
      await reminderService.connectProvider("apple_reminders")
      await refresh()
      setMessage("Apple Reminders connection prepared. Refresh lists to request permission and choose a destination.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const handleSync = async () => {
    setActionLoading("sync")
    setMessage(null)
    try {
      const result = await reminderService.syncLists({ provider: "apple_reminders" })
      setSyncResult(result)
      setSettings(await reminderService.getSettings())
      if (result.error) {
        setMessage(result.error)
      } else {
        setMessage(`Found ${result.syncedListCount} Apple Reminders list${result.syncedListCount === 1 ? "" : "s"}.`)
      }
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const handleDisconnect = async () => {
    setActionLoading("disconnect")
    setMessage(null)
    try {
      await reminderService.disconnectProvider("apple_reminders")
      await refresh()
      setSyncResult(null)
      setMessage("Apple Reminders disconnected. Existing reminders were not modified.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const handleDefaultList = async (list: ReminderList) => {
    setActionLoading("default")
    setMessage(null)
    try {
      await reminderService.updateDefaultList({ provider: "apple_reminders", listId: list.id })
      setSettings(await reminderService.getSettings())
      setMessage(`${list.name} is now the default follow-up destination.`)
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const updateWorkflowPreset = async (preset: ReminderWorkflowPreset, patch: Partial<ReminderWorkflowPreset> & { useGlobalList?: true; useGlobalPriority?: true }) => {
    setActionLoading("workflow")
    setMessage(null)
    try {
      const nextSettings = await reminderService.updateWorkflowPreset({
        category: preset.category,
        enabled: patch.enabled,
        defaultListId: patch.defaultListId,
        useGlobalList: patch.useGlobalList,
        defaultPriority: patch.defaultPriority,
        useGlobalPriority: patch.useGlobalPriority,
        duePreset: patch.duePreset,
      })
      setSettings(nextSettings)
      setMessage("Reminder workflow defaults saved.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const updateGlobalPriority = async (value: number) => {
    setActionLoading("workflow")
    setMessage(null)
    try {
      setSettings(await reminderService.updateWorkflowPreset({ globalPriority: value }))
      setMessage("Global reminder priority saved.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const lastSyncAt = appleAccount?.lastSyncAt ?? syncResult?.completedAt
  const isSaving = actionLoading !== null
  const canUseProvider = appleProvider?.available !== false
  const workflowSettings = settings?.workflowSettings
  const workflowPresets = settings?.workflowPresets ?? []
  const availableListIds = useMemo(() => new Set(appleLists.map((list) => list.id)), [appleLists])
  const presetListValue = (preset: ReminderWorkflowPreset) => (
    preset.defaultListId && availableListIds.has(preset.defaultListId) ? preset.defaultListId : "global"
  )

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="flex items-center gap-3">
              <div className="rounded-md bg-blue-50 p-2 text-blue-700">
                <ListTodo className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-gray-950">Apple Reminders</h3>
                <p className="max-w-3xl text-sm leading-6 text-gray-600">
                  Prepare Apple Reminders destinations for action items discovered after meetings. Meetily uses this to offer reviewable follow-up drafts, so implementation tasks, PR reviews, Linear follow-ups, and clarification loops can become reminders without copying them manually.
                </p>
              </div>
            </div>
            <div className="mt-4 flex flex-wrap items-center gap-2 text-sm">
              <span className={`rounded-full px-2.5 py-1 text-xs font-medium ${statusClass(appleAccount)}`}>
                {statusLabel(appleAccount)}
              </span>
              <span className="text-gray-500">Last list refresh: {formatDateTime(lastSyncAt)}</span>
            </div>
            {appleProvider?.notes && (
              <div className="mt-4 rounded-md border border-blue-100 bg-blue-50 p-3 text-sm text-blue-900">
                {appleProvider.notes}
              </div>
            )}
            {appleAccount?.lastError && (
              <div className="mt-4 flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
                <CircleAlert className="mt-0.5 h-4 w-4 flex-shrink-0" />
                <span>{appleAccount.lastError}</span>
              </div>
            )}
            {message && (
              <div className="mt-4 rounded-md border border-gray-200 bg-gray-50 p-3 text-sm text-gray-700">
                {message}
              </div>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
              onClick={handleConnect}
              disabled={isLoading || isSaving || !canUseProvider || appleAccount?.status === "connected"}
            >
              {actionLoading === "connect" ? <Loader2 className="h-4 w-4 animate-spin" /> : <PlugZap className="h-4 w-4" />}
              Connect
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              onClick={handleSync}
              disabled={isLoading || isSaving || !canUseProvider}
            >
              {actionLoading === "sync" ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
              Refresh lists
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md border border-red-200 bg-white px-3 py-2 text-sm font-medium text-red-700 hover:bg-red-50 disabled:opacity-50"
              onClick={handleDisconnect}
              disabled={isLoading || isSaving || !appleAccount || appleAccount.status === "revoked"}
            >
              {actionLoading === "disconnect" ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
              Disconnect
            </button>
          </div>
        </div>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-950">Default follow-up list</h3>
            <p className="max-w-3xl text-sm leading-6 text-gray-600">
              Choose the default Apple Reminders list used for new meeting follow-up drafts. This is only a default; workflow presets below can route specific categories to another list when needed.
            </p>
          </div>
          {isLoading && <Loader2 className="h-5 w-5 animate-spin text-gray-400" />}
        </div>

        <div className="mt-5 space-y-3">
          {!isLoading && appleLists.length === 0 && (
            <div className="rounded-md border border-dashed border-gray-300 p-6 text-center text-sm text-gray-500">
              Refresh Apple Reminders to show available lists here.
            </div>
          )}
          {appleLists.map((list) => (
            <button
              key={list.id}
              type="button"
              className={`flex w-full items-center justify-between gap-4 rounded-lg border p-4 text-left transition-colors hover:bg-gray-50 disabled:opacity-50 ${
                list.isDefault ? "border-blue-300 bg-blue-50/50" : "border-gray-200 bg-white"
              }`}
              onClick={() => handleDefaultList(list)}
              disabled={isSaving}
            >
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium text-gray-950">{list.name}</span>
                  {list.isDefault && (
                    <span className="rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-800">
                      Default
                    </span>
                  )}
                </div>
                <p className="mt-1 text-xs text-gray-500">List metadata only. Reminder contents stay in Apple Reminders.</p>
              </div>
              {list.isDefault ? (
                <CheckCircle2 className="h-5 w-5 flex-shrink-0 text-emerald-600" />
              ) : (
                <span className="text-sm font-medium text-blue-700">Set default</span>
              )}
            </button>
          ))}
        </div>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className="flex items-center gap-3">
              <div className="rounded-md bg-emerald-50 p-2 text-emerald-700">
                <SlidersHorizontal className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-gray-950">Developer workflow presets</h3>
                <p className="max-w-3xl text-sm leading-6 text-gray-600">
                  Tune which developer-focused follow-ups are suggested and how reminder drafts are prefilled. Presets let you set default due dates, priority, and destination by category without changing existing reminders.
                </p>
              </div>
            </div>
          </div>
          {workflowSettings && (
            <label className="flex items-center gap-2 text-sm text-gray-700">
              Global priority
              <select
                className="rounded-md border border-gray-300 bg-white px-2 py-1 text-sm"
                value={workflowSettings.globalPriority}
                disabled={isSaving}
                onChange={(event) => updateGlobalPriority(Number(event.target.value))}
              >
                <option value={1}>High</option>
                <option value={5}>Medium</option>
                <option value={9}>Low</option>
              </select>
            </label>
          )}
        </div>

        <div className="mt-5 overflow-hidden rounded-lg border border-gray-200">
          <div className="hidden grid-cols-[1.3fr_1fr_1fr_1fr] gap-3 border-b border-gray-200 bg-gray-50 px-4 py-3 text-xs font-semibold uppercase text-gray-500 lg:grid">
            <span>Category</span>
            <span>Due default</span>
            <span>Priority</span>
            <span>List</span>
          </div>
          <div className="divide-y divide-gray-200">
            {workflowPresets.map((preset) => (
              <div key={preset.category} className="grid grid-cols-1 gap-3 px-4 py-4 lg:grid-cols-[1.3fr_1fr_1fr_1fr] lg:items-center">
                <label className="flex min-w-0 items-center gap-3">
                  <input
                    type="checkbox"
                    className="h-4 w-4 rounded border-gray-300 text-blue-600"
                    checked={preset.enabled}
                    disabled={isSaving}
                    onChange={(event) => updateWorkflowPreset(preset, { enabled: event.target.checked })}
                  />
                  <div className="min-w-0">
                    <div className="font-medium text-gray-950">{CATEGORY_LABELS[preset.category] ?? preset.category}</div>
                    <div className="text-xs text-gray-500">{preset.enabled ? "New matching drafts can be suggested." : "Hidden from new draft generation."}</div>
                  </div>
                </label>

                <select
                  className="w-full rounded-md border border-gray-300 bg-white px-2 py-2 text-sm disabled:opacity-50"
                  value={preset.duePreset}
                  disabled={isSaving || !preset.enabled}
                  onChange={(event) => updateWorkflowPreset(preset, { duePreset: event.target.value })}
                >
                  {Object.entries(DUE_PRESET_LABELS).map(([value, label]) => (
                    <option key={value} value={value}>{label}</option>
                  ))}
                </select>

                <select
                  className="w-full rounded-md border border-gray-300 bg-white px-2 py-2 text-sm disabled:opacity-50"
                  value={preset.defaultPriority ?? "global"}
                  disabled={isSaving || !preset.enabled}
                  onChange={(event) => updateWorkflowPreset(preset, event.target.value === "global" ? { useGlobalPriority: true } : { defaultPriority: Number(event.target.value) })}
                >
                  <option value="global">Global ({priorityLabel(workflowSettings?.globalPriority)})</option>
                  <option value={1}>High</option>
                  <option value={5}>Medium</option>
                  <option value={9}>Low</option>
                </select>

                <select
                  className="w-full rounded-md border border-gray-300 bg-white px-2 py-2 text-sm disabled:opacity-50"
                  value={presetListValue(preset)}
                  disabled={isSaving || !preset.enabled || appleLists.length === 0}
                  onChange={(event) => updateWorkflowPreset(preset, event.target.value === "global" ? { useGlobalList: true } : { defaultListId: event.target.value })}
                >
                  <option value="global">Global default</option>
                  {appleLists.map((list) => (
                    <option key={list.id} value={list.id}>{list.name}</option>
                  ))}
                </select>
              </div>
            ))}
            {!isLoading && workflowPresets.length === 0 && (
              <div className="px-4 py-6 text-sm text-gray-500">Workflow presets will appear after the desktop database is initialized.</div>
            )}
          </div>
        </div>

        <p className="mt-3 text-xs text-gray-500">
          Presets only affect future local reminder drafts. Created Apple Reminders stay unchanged, and Meetily will still show drafts for review before creating new follow-ups.
        </p>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-950">Follow-up history</h3>
            <p className="max-w-3xl text-sm leading-6 text-gray-600">
              Recent Apple Reminders created from Meetily meetings. Use this history to confirm what was created, where it was placed, and whether any creation attempt failed.
            </p>
          </div>
          {isLoading && <Loader2 className="h-5 w-5 animate-spin text-gray-400" />}
        </div>
        <div className="mt-5 space-y-3">
          {!isLoading && recentReminders.length === 0 && (
            <div className="rounded-md border border-dashed border-gray-300 p-6 text-center text-sm text-gray-500">
              Created meeting follow-ups will appear here.
            </div>
          )}
          {recentReminders.map((reminder) => (
            <div key={reminder.id} className="rounded-lg border border-gray-200 bg-white p-4">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium text-gray-950">{reminder.title}</p>
                  <p className="mt-1 truncate text-xs text-gray-500">{reminder.meetingTitle ?? "Meetily meeting"}</p>
                </div>
                <span className={`shrink-0 rounded-full px-2 py-0.5 text-xs font-medium ${statusClassName(reminder.status)}`}>
                  {reminder.status}
                </span>
              </div>
              <div className="mt-3 flex flex-wrap gap-x-3 gap-y-1 text-xs text-gray-500">
                <span>{reminder.listName ?? "Apple Reminders"}</span>
                <span>Due {formatDate(reminder.dueAt)}</span>
                <span>Created {formatDate(reminder.createdAt)}</span>
              </div>
              {reminder.lastError && (
                <p className="mt-2 rounded-md bg-amber-50 px-2 py-1 text-xs text-amber-800">{reminder.lastError}</p>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
