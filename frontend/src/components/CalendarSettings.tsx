"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { CalendarCheck2, CalendarClock, CheckCircle2, CircleAlert, Loader2, PlugZap, RefreshCw, Save, Trash2 } from "lucide-react"
import { toast } from "sonner"
import { calendarService, CalendarEvent, CalendarProviderAccount, CalendarSettingsState, CalendarSyncResult } from "@/services/calendarService"
import {
  calendarEventToApprovedEvent,
  clearSelectedCalendarEventForRecording,
  getApprovedCalendarEvents,
  getSelectedCalendarEventForRecording,
  saveApprovedCalendarEvents,
  saveSyncedCalendarEvents,
  selectCalendarEventForRecording,
  syncApprovedCalendarEventsFromProvider,
} from "@/services/meetingDetectionService"

function friendlyError(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return "Calendar settings could not be updated. Check app permissions and try again."
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

function formatTimeRange(event: CalendarEvent): string {
  const start = new Date(event.startsAt)
  const end = new Date(event.endsAt)
  if (!Number.isFinite(start.getTime()) || !Number.isFinite(end.getTime())) return "Time unavailable"
  const day = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric" }).format(start)
  const startTime = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" }).format(start)
  const endTime = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" }).format(end)
  return `${day}, ${startTime} - ${endTime}`
}

function statusLabel(account?: CalendarProviderAccount): string {
  if (!account) return "Not connected"
  if (account.status === "connected") return "Connected"
  if (account.status === "permission_needed") return "Needs permission"
  if (account.status === "revoked") return "Disconnected"
  return "Needs attention"
}

function statusClass(account?: CalendarProviderAccount): string {
  if (account?.status === "connected") return "bg-emerald-100 text-emerald-800"
  if (account?.status === "permission_needed") return "bg-amber-100 text-amber-800"
  if (account?.status === "revoked") return "bg-gray-100 text-gray-700"
  return "bg-red-100 text-red-800"
}

export function CalendarSettings() {
  const [settings, setSettings] = useState<CalendarSettingsState | null>(null)
  const [events, setEvents] = useState<CalendarEvent[]>([])
  const [syncResult, setSyncResult] = useState<CalendarSyncResult | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<"connect" | "sync" | "disconnect" | null>(null)
  const [writeLoading, setWriteLoading] = useState(false)
  const [targetCalendarName, setTargetCalendarName] = useState("Meetily")
  const [autoCreateEvents, setAutoCreateEvents] = useState(false)
  const [selectedRecordingEventId, setSelectedRecordingEventId] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  const appleAccount = useMemo(
    () => settings?.accounts.find((account) => account.provider === "apple"),
    [settings],
  )

  const refresh = useCallback(async () => {
    setIsLoading(true)
    setMessage(null)
    try {
      const [nextSettings, nextEvents] = await Promise.all([
        calendarService.getSettings(),
        calendarService.listUpcomingEvents(25),
      ])
      setSettings(nextSettings)
      setEvents(nextEvents)
      const account = nextSettings.accounts.find((account) => account.provider === "apple")
      setTargetCalendarName(account?.targetCalendarName ?? "Meetily")
      setAutoCreateEvents(account?.autoCreateEvents ?? false)
      if (nextEvents.length > 0) {
        saveSyncedCalendarEvents(nextEvents.map(calendarEventToApprovedEvent))
      }
      setSelectedRecordingEventId(getSelectedCalendarEventForRecording()?.id ?? null)
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
      await calendarService.connectProvider("apple")
      await refresh()
      setMessage("Apple Calendar connection prepared. Run sync to request permission and cache events.")
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
      const { result, events: approvedEvents } = await syncApprovedCalendarEventsFromProvider({
        provider: "apple",
        lookbackDays: 1,
        lookaheadDays: 14,
      }, 25)
      setSyncResult(result)
      setEvents(await calendarService.listUpcomingEvents(25))
      setSettings(await calendarService.getSettings())
      if (result.error) {
        setMessage(result.error)
      } else {
        setMessage(`Synced ${approvedEvents.length} upcoming calendar event${approvedEvents.length === 1 ? "" : "s"}.`)
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
      await calendarService.disconnectProvider("apple")
      saveApprovedCalendarEvents(getApprovedCalendarEvents().filter((event) => event.source !== "calendar"))
      clearSelectedCalendarEventForRecording()
      setSelectedRecordingEventId(null)
      await refresh()
      setEvents([])
      setMessage("Apple Calendar disconnected. Cached upcoming prompts were cleared.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setActionLoading(null)
    }
  }

  const handleSaveWriteSettings = async () => {
    setWriteLoading(true)
    setMessage(null)
    try {
      const account = await calendarService.updateWriteSettings({
        provider: "apple",
        targetCalendarName,
        autoCreateEvents,
      })
      setSettings(await calendarService.getSettings())
      setTargetCalendarName(account.targetCalendarName)
      setAutoCreateEvents(account.autoCreateEvents)
      setMessage("Apple Calendar event creation settings saved.")
    } catch (error) {
      setMessage(friendlyError(error))
    } finally {
      setWriteLoading(false)
    }
  }

  const handleUseForRecording = (event: CalendarEvent) => {
    const approvedEvent = calendarEventToApprovedEvent(event)
    selectCalendarEventForRecording(approvedEvent)
    setSelectedRecordingEventId(approvedEvent.id)
    saveApprovedCalendarEvents([approvedEvent, ...getApprovedCalendarEvents().filter((item) => item.id !== approvedEvent.id)].slice(0, 50))
    toast.success("Calendar event selected", {
      description: "The next recording will use this event title and metadata.",
    })
  }

  const handleClearRecordingSelection = () => {
    clearSelectedCalendarEventForRecording()
    setSelectedRecordingEventId(null)
    toast.info("Calendar event selection cleared")
  }

  const lastSyncAt = appleAccount?.lastSyncAt ?? syncResult?.completedAt
  const isSaving = actionLoading !== null

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="flex items-center gap-3">
              <div className="rounded-md bg-blue-50 p-2 text-blue-700">
                <CalendarCheck2 className="h-5 w-5" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-gray-950">Apple Calendar</h3>
                <p className="text-sm text-gray-600">Read upcoming event metadata locally for meeting prompts and recording titles.</p>
              </div>
            </div>
            <div className="mt-4 flex flex-wrap items-center gap-2 text-sm">
              <span className={`rounded-full px-2.5 py-1 text-xs font-medium ${statusClass(appleAccount)}`}>
                {statusLabel(appleAccount)}
              </span>
              <span className="text-gray-500">Last sync: {formatDateTime(lastSyncAt)}</span>
            </div>
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
              disabled={isLoading || isSaving || appleAccount?.status === "connected"}
            >
              {actionLoading === "connect" ? <Loader2 className="h-4 w-4 animate-spin" /> : <PlugZap className="h-4 w-4" />}
              Connect
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              onClick={handleSync}
              disabled={isLoading || isSaving}
            >
              {actionLoading === "sync" ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
              Sync now
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
        <div>
          <h3 className="text-lg font-semibold text-gray-950">Event creation</h3>
          <p className="text-sm text-gray-600">Create or update Meetily-owned Apple Calendar events for completed recordings.</p>
        </div>
        <div className="mt-5 grid gap-4 lg:grid-cols-[1fr_auto] lg:items-end">
          <label className="block">
            <span className="text-sm font-medium text-gray-700">Target calendar</span>
            <input
              className="mt-2 w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-950 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-100"
              value={targetCalendarName}
              disabled={writeLoading}
              onChange={(event) => setTargetCalendarName(event.target.value)}
              placeholder="Meetily"
            />
          </label>
          <button
            type="button"
            className="inline-flex items-center justify-center gap-2 rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            onClick={handleSaveWriteSettings}
            disabled={isLoading || writeLoading}
          >
            {writeLoading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
            Save
          </button>
        </div>
        <label className="mt-5 flex items-start gap-3 rounded-lg border border-gray-200 bg-gray-50 p-4">
          <input
            type="checkbox"
            className="mt-1 h-4 w-4 rounded border-gray-300 text-blue-600"
            checked={autoCreateEvents}
            disabled={writeLoading}
            onChange={(event) => setAutoCreateEvents(event.target.checked)}
          />
          <span>
            <span className="block text-sm font-medium text-gray-950">Allow Meetily to create calendar events</span>
            <span className="mt-1 block text-xs text-gray-500">Off by default. Meetily only updates events it created or linked.</span>
          </span>
        </label>
      </div>

      <div className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-950">Upcoming meetings</h3>
            <p className="text-sm text-gray-600">Select an event to populate the next recording title and meeting metadata.</p>
          </div>
          {isLoading && <Loader2 className="h-5 w-5 animate-spin text-gray-400" />}
        </div>

        <div className="mt-5 space-y-3">
          {!isLoading && events.length === 0 && (
            <div className="rounded-md border border-dashed border-gray-300 p-6 text-center text-sm text-gray-500">
              Sync Apple Calendar to show upcoming meetings here.
            </div>
          )}
          {events.map((event) => {
            const selected = selectedRecordingEventId === event.id
            return (
              <div key={event.id} className={`rounded-lg border p-4 ${selected ? "border-blue-300 bg-blue-50/40" : "border-gray-200"}`}>
                <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h4 className="font-medium text-gray-950">{event.title}</h4>
                      {event.meetingProvider && (
                        <span className="rounded-full bg-blue-50 px-2 py-0.5 text-xs font-medium text-blue-700">
                          {event.meetingProvider.replace("_", " ")}
                        </span>
                      )}
                      {event.meetingUrl && <CheckCircle2 className="h-4 w-4 text-emerald-600" />}
                      {selected && (
                        <span className="rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-800">
                          Selected
                        </span>
                      )}
                    </div>
                    <div className="mt-1 flex flex-wrap items-center gap-2 text-sm text-gray-600">
                      <CalendarClock className="h-4 w-4" />
                      {formatTimeRange(event)}
                    </div>
                    {event.descriptionExcerpt && (
                      <p className="mt-2 max-w-3xl text-sm text-gray-500">{event.descriptionExcerpt}</p>
                    )}
                  </div>
                  <div className="flex shrink-0 flex-wrap gap-2">
                    {selected && (
                      <button
                        type="button"
                        className="inline-flex items-center justify-center rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50"
                        onClick={handleClearRecordingSelection}
                      >
                        Unselect
                      </button>
                    )}
                    <button
                      type="button"
                      className="inline-flex items-center justify-center rounded-md bg-gray-900 px-3 py-2 text-sm font-medium text-white hover:bg-gray-800"
                      onClick={() => handleUseForRecording(event)}
                    >
                      {selected ? "Selected for recording" : "Use for next recording"}
                    </button>
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
