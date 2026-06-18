# Frontend Agent Instructions

This tree contains the supported Meetily desktop UI: Next.js app routes, React components, contexts, hooks, services, and Tauri API wrappers.

## Routing

| Task | Start here |
| --- | --- |
| Main recording workspace | `src/app/page.tsx`, `src/components/RecordingControls.tsx`, `src/components/RecordingStatusBar.tsx`, `src/contexts/RecordingStateContext.tsx` |
| App shell, drag/drop import, global providers | `src/app/layout.tsx`, `src/components/MainNav/`, `src/components/MainContent/`, `src/contexts/ImportDialogContext.tsx` |
| Meeting detail page | `src/app/meeting-details/`, `src/components/MeetingDetails/`, `src/hooks/meeting-details/` |
| Sidebar, meeting list, search, summary previews | `src/components/Sidebar/` |
| Recording start/stop/pause/resume flows | `src/hooks/useRecordingStart.ts`, `src/hooks/useRecordingStop.ts`, `src/services/recordingService.ts` |
| Transcripts and streaming updates | `src/contexts/TranscriptContext.tsx`, `src/services/transcriptService.ts`, `src/hooks/useTranscriptStreaming.ts`, `src/components/TranscriptView.tsx`, `src/components/VirtualizedTranscriptView.tsx` |
| Audio import | `src/components/ImportAudio/`, `src/hooks/useImportAudio.ts` |
| Retranscription and recovery | `src/components/MeetingDetails/RetranscribeDialog.tsx`, `src/components/TranscriptRecovery/`, `src/hooks/useTranscriptRecovery.ts` |
| Onboarding and first-run permissions/models | `src/components/onboarding/`, `src/contexts/OnboardingContext.tsx`, `src/lib/onboarding-summary-model.ts` |
| Model settings and providers | `src/components/ModelSettingsModal.tsx`, `src/components/SummaryModelSettings.tsx`, `src/components/WhisperModelManager.tsx`, `src/components/ParakeetModelManager.tsx`, `src/components/BuiltInModelManager.tsx` |
| Notifications, updates, analytics, privacy UI | `src/components/Update*`, `src/services/updateService.ts`, `src/components/Analytics*`, `src/lib/analytics.ts` |
| Tauri wrappers and safe platform access | `src/lib/tauri.ts`, service files under `src/services/` |
| Shared UI primitives | `src/components/ui/` |

## Conventions

- Keep Tauri IPC calls behind existing services or focused hooks when possible.
- Use `src/lib/tauri.ts` for safe Tauri imports and platform checks.
- Keep user-facing errors actionable and friendly; avoid raw provider, Rust, or diagnostic strings.
- Match the existing Tailwind/Radix/lucide component style and the existing context/hook split.
- Do not add browser-only assumptions to code that runs in the Tauri desktop shell.
- When adding a new Tauri command call, verify it is registered in `src-tauri/src/lib.rs`.

## Verification

- UI-only changes: run `pnpm run lint` from `frontend/`.
- Tauri IPC changes: also run `cargo check` from `frontend/src-tauri`.
- Import, recording, permissions, or model download changes need manual app verification when feasible; compile checks do not cover native device behavior.
