"use client"

import { useCallback, useEffect, useMemo, useState } from "react";
import { CalendarClock, ExternalLink, Mic, X } from "lucide-react";
import { toast } from "sonner";
import {
  MeetingDetectionSettings,
  MeetingJoinCandidate,
  MEETING_DETECTION_SETTINGS_EVENT,
  dismissMeetingCandidate,
  getApprovedCalendarEvents,
  getMeetingDetectionSettings,
  getUpcomingMeetingCandidates,
  markMeetingCandidateAutoOpened,
  openMeetingCandidate,
  saveMeetingDetectionSettings,
  wasAutoOpened,
} from "@/services/meetingDetectionService";

interface MeetingDetectionPromptProps {
  sidebarCollapsed: boolean;
  onStartRecording: (candidate: MeetingJoinCandidate) => Promise<void>;
  isRecording: boolean;
}

function providerLabel(provider: MeetingJoinCandidate["provider"]): string {
  switch (provider) {
    case "google-meet":
      return "Google Meet";
    case "zoom":
      return "Zoom";
    case "teams":
      return "Microsoft Teams";
    default:
      return "Meeting";
  }
}

function timeLabel(candidate: MeetingJoinCandidate): string {
  if (candidate.isActive) return "Happening now";
  if (candidate.minutesUntilStart <= 0) return "Starting now";
  if (candidate.minutesUntilStart === 1) return "Starts in 1 minute";
  return `Starts in ${candidate.minutesUntilStart} minutes`;
}

export function MeetingDetectionPrompt({ sidebarCollapsed, onStartRecording, isRecording }: MeetingDetectionPromptProps) {
  const [settings, setSettings] = useState<MeetingDetectionSettings>(() => getMeetingDetectionSettings());
  const [candidates, setCandidates] = useState<MeetingJoinCandidate[]>([]);
  const [isOpening, setIsOpening] = useState(false);
  const [isStarting, setIsStarting] = useState(false);

  const refreshCandidates = useCallback(() => {
    const nextSettings = getMeetingDetectionSettings();
    setSettings(nextSettings);
    setCandidates(getUpcomingMeetingCandidates(getApprovedCalendarEvents(), nextSettings));
  }, []);

  useEffect(() => {
    refreshCandidates();
    const interval = window.setInterval(refreshCandidates, 30000);
    const onStorage = () => refreshCandidates();
    window.addEventListener("storage", onStorage);
    window.addEventListener(MEETING_DETECTION_SETTINGS_EVENT, onStorage);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("storage", onStorage);
      window.removeEventListener(MEETING_DETECTION_SETTINGS_EVENT, onStorage);
    };
  }, [refreshCandidates]);

  const candidate = candidates[0];

  useEffect(() => {
    if (!candidate || settings.mode !== "autoOpen" || wasAutoOpened(candidate)) return;
    openMeetingCandidate(candidate)
      .then(() => {
        markMeetingCandidateAutoOpened(candidate);
        toast.info("Meeting link opened", {
          description: "Meetily opened the link because auto-open is enabled. Recording still requires your action.",
        });
      })
      .catch((error) => {
        console.error("Failed to auto-open meeting:", error);
        toast.error("Unable to open meeting link");
      });
  }, [candidate, settings.mode]);

  const attendeePreview = useMemo(() => {
    if (!candidate?.attendees.length) return null;
    const visible = candidate.attendees.slice(0, 3).join(", ");
    const remaining = candidate.attendees.length - 3;
    return remaining > 0 ? `${visible} +${remaining}` : visible;
  }, [candidate]);

  if (!candidate || settings.mode === "disabled" || isRecording) return null;

  const handleOpen = async () => {
    setIsOpening(true);
    try {
      await openMeetingCandidate(candidate);
      toast.success("Meeting link opened");
    } catch (error) {
      console.error("Failed to open meeting:", error);
      toast.error("Unable to open meeting link");
    } finally {
      setIsOpening(false);
    }
  };

  const handleStartRecording = async () => {
    setIsStarting(true);
    try {
      await onStartRecording(candidate);
    } finally {
      setIsStarting(false);
    }
  };

  const handleDismiss = () => {
    dismissMeetingCandidate(candidate);
    refreshCandidates();
  };

  const handleDisable = () => {
    const nextSettings = saveMeetingDetectionSettings({ ...settings, mode: "disabled" });
    setSettings(nextSettings);
    setCandidates([]);
    window.dispatchEvent(new Event(MEETING_DETECTION_SETTINGS_EVENT));
    toast.info("Meeting detection disabled");
  };

  return (
    <div
      className="fixed left-0 right-0 top-6 z-20 flex justify-center px-6 transition-[margin] duration-300 ease-out"
      style={{ marginLeft: sidebarCollapsed ? "4.5rem" : "18rem" }}
    >
      <div className="w-full max-w-3xl rounded-lg border border-blue-200 bg-white/95 p-4 shadow-[0_18px_50px_rgba(15,23,42,0.16)] backdrop-blur">
        <div className="flex items-start gap-3">
          <div className="mt-0.5 rounded-md bg-blue-50 p-2 text-blue-700">
            <CalendarClock className="h-5 w-5" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="font-semibold text-gray-950">{candidate.title}</h3>
              <span className="rounded-full bg-blue-50 px-2 py-0.5 text-xs font-medium text-blue-700">
                {timeLabel(candidate)}
              </span>
              <span className="rounded-full bg-gray-100 px-2 py-0.5 text-xs text-gray-700">
                {providerLabel(candidate.provider)}
              </span>
            </div>
            <p className="mt-1 text-sm text-gray-600">
              {candidate.calendarName ? `${candidate.calendarName} calendar` : "Approved calendar event"}
              {attendeePreview ? ` · ${attendeePreview}` : ""}
            </p>
            <p className="mt-1 text-xs text-gray-500">
              Meetily can open the meeting link or start recording with this title. It will not join or record silently.
            </p>
          </div>
          <button
            type="button"
            className="rounded-md p-1 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            onClick={handleDismiss}
            aria-label="Dismiss meeting prompt"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="mt-4 flex flex-wrap items-center justify-between gap-3">
          <button
            type="button"
            className="text-xs font-medium text-gray-500 hover:text-gray-800"
            onClick={handleDisable}
          >
            Disable detection
          </button>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
              onClick={handleOpen}
              disabled={isOpening}
            >
              <ExternalLink className="h-4 w-4" />
              Open meeting
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
              onClick={handleStartRecording}
              disabled={isStarting}
            >
              <Mic className="h-4 w-4" />
              Start recording
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
