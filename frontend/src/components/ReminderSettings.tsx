"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { CheckCircle2, CircleAlert, ListTodo, Loader2, PlugZap, RefreshCw, Trash2 } from "lucide-react"
import { reminderService, ReminderList, ReminderProviderAccount, ReminderSettingsState, ReminderListSyncResult } from "@/services/reminderService"

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

export function ReminderSettings() {
  const [settings, setSettings] = useState<ReminderSettingsState | null>(null)
  const [syncResult, setSyncResult] = useState<ReminderListSyncResult | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<"connect" | "sync" | "disconnect" | "default" | null>(null)
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
      setSettings(await reminderService.getSettings())
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

  const lastSyncAt = appleAccount?.lastSyncAt ?? syncResult?.completedAt
  const isSaving = actionLoading !== null
  const canUseProvider = appleProvider?.available !== false

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
                <p className="text-sm text-gray-600">Prepare local follow-up destinations for action items after meetings.</p>
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
            <p className="text-sm text-gray-600">Choose where meeting action reminders will be offered in the next workflow slice.</p>
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
    </div>
  )
}
