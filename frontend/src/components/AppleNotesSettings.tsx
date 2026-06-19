"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { CheckCircle2, CircleAlert, FileText, Loader2, PlugZap, Save, Trash2 } from "lucide-react"
import { appleNotesService, AppleNotesExportRecord, AppleNotesProviderAccount, AppleNotesSettingsState } from "@/services/appleNotesService"

function friendlyError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "Apple Notes settings could not be updated. Check app permissions and try again."
}

function formatDateTime(value?: string | null): string {
  if (!value) return "Never"
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) return "Never"
  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    hour: "numeric",
    minute: "2-digit",
    month: "short",
    day: "numeric",
  }).format(date)
}

function statusLabel(account?: AppleNotesProviderAccount): string {
  if (!account) return "Not connected"
  if (account.status === "connected") return "Connected"
  if (account.status === "permission_needed") return "Needs permission"
  if (account.status === "revoked") return "Disconnected"
  return "Needs attention"
}

function statusClass(account?: AppleNotesProviderAccount): string {
  if (account?.status === "connected") return "bg-emerald-100 text-emerald-800"
  if (account?.status === "permission_needed") return "bg-amber-100 text-amber-800"
  if (account?.status === "revoked") return "bg-gray-100 text-gray-700"
  return "bg-red-100 text-red-800"
}

function exportStatusClass(status: string): string {
  if (status === "exported" || status === "updated") return "bg-emerald-100 text-emerald-800"
  if (status === "failed" || status === "missing") return "bg-amber-100 text-amber-800"
  if (status === "revoked") return "bg-gray-100 text-gray-700"
  return "bg-blue-100 text-blue-800"
}

export function AppleNotesSettings() {
  const [settings, setSettings] = useState<AppleNotesSettingsState | null>(null)
  const [recentExports, setRecentExports] = useState<AppleNotesExportRecord[]>([])
  const [rootFolderName, setRootFolderName] = useState("Meetily")
  const [autoExportEnabled, setAutoExportEnabled] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<"connect" | "save" | "disconnect" | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  const appleAccount = useMemo(
    () => settings?.accounts.find((account) => account.provider === "apple_notes"),
    [settings],
  )

  const appleProvider = useMemo(
    () => settings?.providers.find((provider) => provider.provider === "apple_notes"),
    [settings],
  )

  const refresh = useCallback(async () => {
    setIsLoading(true)
    setMessage(null)
    try {
      const nextSettings = await appleNotesService.getSettings()
      setSettings(nextSettings)
      setRecentExports(nextSettings.recentExports)
      const account = nextSettings.accounts.find((item) => item.provider === "apple_notes")
      setRootFolderName(account?.rootFolderName ?? "Meetily")
      setAutoExportEnabled(account?.autoExportEnabled ?? false)
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
      await appleNotesService.connectProvider()
      await refresh()
      setMessage("Apple Notes connection prepared. The first export will request macOS Automation permission.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const handleSave = async () => {
    setActionLoading("save")
    setMessage(null)
    try {
      const account = await appleNotesService.updateSettings({
        rootFolderName,
        autoExportEnabled,
      })
      setSettings(await appleNotesService.getSettings())
      setRootFolderName(account.rootFolderName)
      setAutoExportEnabled(account.autoExportEnabled)
      setMessage("Apple Notes settings saved.")
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
      await appleNotesService.disconnectProvider()
      await refresh()
      setMessage("Apple Notes disconnected. Existing notes were not modified.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const isSaving = actionLoading !== null
  const canUseProvider = appleProvider?.available !== false

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="flex items-center gap-3">
              <div className="rounded-md bg-blue-50 p-2 text-blue-700">
                <FileText className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-gray-950">Apple Notes</h3>
                <p className="text-sm text-gray-600">Export completed meeting summaries to app-managed Notes.</p>
              </div>
            </div>
            <div className="mt-4 flex flex-wrap items-center gap-2 text-sm">
              <span className={`rounded-full px-2.5 py-1 text-xs font-medium ${statusClass(appleAccount)}`}>
                {statusLabel(appleAccount)}
              </span>
              <span className="text-gray-500">Last export: {formatDateTime(appleAccount?.lastExportAt)}</span>
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
        <div className="flex flex-col gap-2">
          <h3 className="text-lg font-semibold text-gray-950">Destination and automation</h3>
          <p className="text-sm text-gray-600">Manual export always asks for confirmation. Auto-export only runs after this destination has been confirmed.</p>
        </div>

        <div className="mt-5 grid gap-4 lg:grid-cols-[1fr_auto] lg:items-end">
          <label className="block">
            <span className="text-sm font-medium text-gray-700">Root folder</span>
            <input
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-950 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-100"
              value={rootFolderName}
              disabled={isSaving}
              onChange={(event) => setRootFolderName(event.target.value)}
              placeholder="Meetily"
            />
          </label>
          <button
            type="button"
            className="inline-flex items-center justify-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            onClick={handleSave}
            disabled={isLoading || isSaving || !canUseProvider}
          >
            {actionLoading === "save" ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
            Save
          </button>
        </div>

        <label className="mt-5 flex items-start gap-3 rounded-lg border border-gray-200 bg-gray-50 p-4">
          <input
            type="checkbox"
            className="mt-1 h-4 w-4 rounded border-gray-300 text-blue-600"
            checked={autoExportEnabled}
            disabled={isSaving}
            onChange={(event) => setAutoExportEnabled(event.target.checked)}
          />
          <span>
            <span className="block text-sm font-medium text-gray-950">Auto-export after summary completion</span>
            <span className="mt-1 block text-xs text-gray-500">Off by default. The first export for each destination still requires confirmation from meeting details.</span>
          </span>
        </label>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-950">Export history</h3>
            <p className="text-sm text-gray-600">Recent Apple Notes exports created by Meetily.</p>
          </div>
          {isLoading && <Loader2 className="h-5 w-5 animate-spin text-gray-400" />}
        </div>
        <div className="mt-5 space-y-3">
          {!isLoading && recentExports.length === 0 && (
            <div className="rounded-md border border-dashed border-gray-300 p-6 text-center text-sm text-gray-500">
              Apple Notes exports will appear here.
            </div>
          )}
          {recentExports.map((record) => (
            <div key={record.id} className="rounded-lg border border-gray-200 bg-white p-4">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium text-gray-950">{record.noteTitle}</p>
                  <p className="mt-1 truncate text-xs text-gray-500">{record.folderName ?? "Apple Notes"} · {formatDateTime(record.exportedAt ?? record.updatedAt)}</p>
                </div>
                <span className={`shrink-0 rounded-full px-2 py-0.5 text-xs font-medium ${exportStatusClass(record.status)}`}>
                  {record.status}
                </span>
              </div>
              {record.lastError && (
                <p className="mt-2 rounded-md bg-amber-50 px-2 py-1 text-xs text-amber-800">{record.lastError}</p>
              )}
              {(record.status === "exported" || record.status === "updated") && (
                <div className="mt-3 flex items-center gap-2 text-xs text-emerald-700">
                  <CheckCircle2 className="h-3.5 w-3.5" />
                  <span>{record.accountName ?? "Apple Notes"}</span>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
