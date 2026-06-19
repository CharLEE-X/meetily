"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { CircleAlert, FileText, Loader2, RefreshCw } from "lucide-react"
import { toast } from "sonner"
import { appleNotesService, AppleNotesExportPreview, AppleNotesExportRecord, AppleNotesSettingsState } from "@/services/appleNotesService"

interface AppleNotesExportPanelProps {
  meetingId: string
  hasSummary: boolean
  summaryStatus: string
}

function friendlyError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "Apple Notes export could not be completed."
}

function statusClass(status?: string | null): string {
  if (status === "exported" || status === "updated") return "bg-emerald-100 text-emerald-800"
  if (status === "failed" || status === "missing") return "bg-amber-100 text-amber-800"
  if (status === "revoked") return "bg-gray-100 text-gray-700"
  return "bg-blue-100 text-blue-800"
}

function formatDateTime(value?: string | null): string {
  if (!value) return "Never"
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) return "Never"
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date)
}

export function AppleNotesExportPanel({ meetingId, hasSummary, summaryStatus }: AppleNotesExportPanelProps) {
  const [settings, setSettings] = useState<AppleNotesSettingsState | null>(null)
  const [preview, setPreview] = useState<AppleNotesExportPreview | null>(null)
  const [record, setRecord] = useState<AppleNotesExportRecord | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<"preview" | "export" | null>(null)
  const autoExportKeyRef = useRef<string | null>(null)

  const account = useMemo(
    () => settings?.accounts.find((item) => item.provider === "apple_notes"),
    [settings],
  )
  const isConnected = account?.status === "connected" || account?.status === "permission_needed"
  const autoExportEnabled = account?.autoExportEnabled ?? false

  const refresh = useCallback(async () => {
    setIsLoading(true)
    setMessage(null)
    try {
      const [nextSettings, nextRecord] = await Promise.all([
        appleNotesService.getSettings(),
        appleNotesService.getMeetingExport(meetingId),
      ])
      setSettings(nextSettings)
      setRecord(nextRecord)
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsLoading(false)
    }
  }, [meetingId])

  useEffect(() => {
    refresh()
  }, [refresh])

  const loadPreview = useCallback(async () => {
    setActionLoading("preview")
    setMessage(null)
    try {
      const nextPreview = await appleNotesService.previewExport(meetingId)
      setPreview(nextPreview)
      setMessage(nextPreview.requiresDestinationConfirmation ? "Review the destination before exporting." : "Destination already confirmed.")
      return nextPreview
    } catch (error) {
      setMessage(friendlyError(error))
      return null
    } finally {
      setActionLoading(null)
    }
  }, [meetingId])

  const exportWithPreview = useCallback(async (activePreview: AppleNotesExportPreview) => {
    setActionLoading("export")
    setMessage(null)
    try {
      const nextRecord = await appleNotesService.exportMeeting({
        meetingId,
        confirmDestinationHash: activePreview.destinationHash,
      })
      setRecord(nextRecord)
      setPreview(null)
      setSettings(await appleNotesService.getSettings())
      toast.success("Exported to Apple Notes")
      setMessage(`${nextRecord.status === "updated" ? "Updated" : "Created"} ${nextRecord.folderName ?? activePreview.folderName}.`)
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }, [meetingId])

  const handlePrimaryAction = async () => {
    if (!preview) {
      const nextPreview = await loadPreview()
      if (nextPreview && !nextPreview.requiresDestinationConfirmation) {
        await exportWithPreview(nextPreview)
      }
      return
    }
    await exportWithPreview(preview)
  }

  useEffect(() => {
    if (!hasSummary || summaryStatus !== "completed" || !autoExportEnabled || !isConnected) return
    let cancelled = false
    const run = async () => {
      const nextPreview = await appleNotesService.previewExport(meetingId)
      if (cancelled) return
      const key = `${meetingId}:${nextPreview.destinationHash}:${nextPreview.contentHash}`
      if (autoExportKeyRef.current === key) return
      if (nextPreview.requiresDestinationConfirmation) {
        autoExportKeyRef.current = key
        setPreview(nextPreview)
        setMessage("Auto-export is enabled. Confirm this Apple Notes destination once to allow future automatic exports.")
        return
      }
      autoExportKeyRef.current = key
      await exportWithPreview(nextPreview)
    }
    run().catch((error) => {
      if (!cancelled) setMessage(friendlyError(error))
    })
    return () => {
      cancelled = true
    }
  }, [autoExportEnabled, exportWithPreview, hasSummary, isConnected, meetingId, summaryStatus])

  if (!hasSummary) return null

  return (
    <div className="mx-6 mb-4 mt-2 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="min-w-0">
          <div className="flex items-center gap-3">
            <div className="rounded-md bg-blue-50 p-2 text-blue-700">
              <FileText className="h-4 w-4" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-slate-950">Apple Notes export</h3>
              <p className="text-sm text-slate-600">Manual export is separate from auto-export.</p>
            </div>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
            <span className={`rounded-full px-2 py-0.5 font-medium ${statusClass(record?.status)}`}>
              {record?.status ?? (isConnected ? "ready" : "not connected")}
            </span>
            <span className="text-slate-500">Last export: {formatDateTime(record?.exportedAt)}</span>
            {autoExportEnabled && <span className="rounded-full bg-blue-50 px-2 py-0.5 font-medium text-blue-700">Auto-export on</span>}
          </div>
          {preview && (
            <div className="mt-3 rounded-lg border border-blue-100 bg-blue-50 p-3 text-sm text-blue-950">
              <p className="font-medium">{preview.noteTitle}</p>
              <p className="mt-1 text-blue-800">{preview.accountLabel} · {preview.folderName}</p>
              {preview.iCloudSyncDisclosure && (
                <p className="mt-2 flex items-start gap-2 text-amber-800">
                  <CircleAlert className="mt-0.5 h-4 w-4 flex-shrink-0" />
                  <span>{preview.iCloudSyncDisclosure}</span>
                </p>
              )}
            </div>
          )}
          {(message || record?.lastError) && (
            <div className="mt-3 rounded-lg border border-slate-200 bg-slate-50 p-3 text-sm text-slate-700">
              {message ?? record?.lastError}
            </div>
          )}
        </div>

        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="inline-flex items-center gap-2 rounded-md border border-slate-300 bg-white px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50"
            onClick={loadPreview}
            disabled={isLoading || actionLoading !== null}
          >
            {actionLoading === "preview" ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
            Preview
          </button>
          <button
            type="button"
            className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            onClick={handlePrimaryAction}
            disabled={isLoading || actionLoading !== null || !isConnected}
          >
            {actionLoading === "export" ? <Loader2 className="h-4 w-4 animate-spin" /> : <FileText className="h-4 w-4" />}
            {preview ? "Export to Notes" : "Export"}
          </button>
        </div>
      </div>
    </div>
  )
}
