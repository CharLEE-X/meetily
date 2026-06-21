"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { CalendarPlus, Loader2 } from "lucide-react"
import { toast } from "sonner"
import { calendarService, CalendarEventCreationResult, CalendarSettingsState } from "@/services/calendarService"

interface CalendarEventPanelProps {
  meetingId: string
  hasSummary: boolean
}

function friendlyError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "Apple Calendar event could not be created."
}

export function CalendarEventPanel({ meetingId, hasSummary }: CalendarEventPanelProps) {
  const [settings, setSettings] = useState<CalendarSettingsState | null>(null)
  const [result, setResult] = useState<CalendarEventCreationResult | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [isCreating, setIsCreating] = useState(false)

  const account = useMemo(
    () => settings?.accounts.find((item) => item.provider === "apple"),
    [settings],
  )
  const canCreate = account?.autoCreateEvents && (account.status === "connected" || account.status === "permission_needed")

  const refresh = useCallback(async () => {
    setIsLoading(true)
    try {
      setSettings(await calendarService.getSettings())
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  const createEvent = async () => {
    setIsCreating(true)
    setMessage(null)
    try {
      const nextResult = await calendarService.createOrUpdateMeetingEvent({ meetingId })
      setResult(nextResult)
      toast.success(nextResult.status === "updated" ? "Calendar event updated" : "Calendar event created")
      setMessage(`${nextResult.status === "updated" ? "Updated" : "Created"} in ${nextResult.calendarName}.`)
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setIsCreating(false)
    }
  }

  if (!hasSummary) return null

  return (
    <div className="mx-6 mb-4 mt-2 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <div className="flex items-center gap-3">
            <div className="rounded-md bg-emerald-50 p-2 text-emerald-700">
              <CalendarPlus className="h-4 w-4" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-slate-950">Apple Calendar event</h3>
              <p className="text-sm text-slate-600">Create or update a Meetily-owned calendar record.</p>
            </div>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
            <span className={`rounded-full px-2 py-0.5 font-medium ${canCreate ? "bg-emerald-100 text-emerald-800" : "bg-gray-100 text-gray-700"}`}>
              {canCreate ? "enabled" : "disabled"}
            </span>
            <span className="text-slate-500">{account?.targetCalendarName ?? "RecallX"}</span>
            {result && <span className="text-slate-500">{result.status}</span>}
          </div>
          {message && (
            <div className="mt-3 rounded-lg border border-slate-200 bg-slate-50 p-3 text-sm text-slate-700">
              {message}
            </div>
          )}
        </div>
        <button
          type="button"
          className="inline-flex items-center justify-center gap-2 rounded-md bg-emerald-600 px-3 py-2 text-sm font-medium text-white hover:bg-emerald-700 disabled:opacity-50"
          onClick={createEvent}
          disabled={isLoading || isCreating || !canCreate}
        >
          {isCreating ? <Loader2 className="h-4 w-4 animate-spin" /> : <CalendarPlus className="h-4 w-4" />}
          Create event
        </button>
      </div>
    </div>
  )
}
