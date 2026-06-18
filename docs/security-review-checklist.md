# Security Review Checklist

This checklist is the release gate for sensitive Meetily automation. The detailed checklist is maintained by CHA-1728 and must be completed or explicitly waived before any feature covered by [Privacy, Consent, and Access Controls](privacy-consent-access-controls.md) ships.

Release status must remain blocked until every applicable item is complete or explicitly waived by the release owner with a reason, risk, expiration, and follow-up issue.

## Required Issue Links

Every sensitive feature epic, implementation issue, or QA child must link to:

* This checklist.
* [Privacy, Consent, and Access Controls](privacy-consent-access-controls.md).
* Any feature-specific policy section, such as MCP access, external data boundaries, or screenshot controls.

## Consent and Permission Gate

Verify that the feature:

* Is disabled by default.
* Has an explicit opt-in action and clear consent scope.
* Shows runtime state for active capture, export, sync, MCP, or provider use.
* Provides pause, stop, disable, disconnect, revoke, or delete controls as applicable.
* Handles denied, missing, and revoked OS permissions without data loss or misleading success states.
* Shows user-friendly errors without raw provider, Rust, shell, or diagnostic strings.

Required areas to check when applicable:

* Microphone and system audio permissions.
* macOS screen recording permissions.
* Local file read/write permissions and user-selected export destinations.
* Apple Events permissions for Apple Calendar and Apple Notes.
* Calendar provider OAuth scopes and selected calendar lists.
* Local MCP server enablement, client authorization, token storage, and revocation.
* Cloud model/provider configuration and provider disclosure.

## Local Storage and File Access Gate

Verify that the feature:

* Uses Tauri path APIs or app-managed storage instead of hardcoded user paths.
* Writes app-managed artifacts only under approved app-data locations unless the user chooses a destination.
* Shows destination previews before external writes.
* Does not delete user-managed external files without explicit confirmation.
* Deletes or invalidates derived artifacts when a meeting is deleted.
* Handles missing local files gracefully when metadata remains.
* Keeps logs free of transcript text, screenshots, prompts, summaries, raw tokens, and credentials.

## Local Server and MCP Gate

Verify that local server behavior:

* Binds only to loopback (`127.0.0.1` or `::1`) by default.
* Refuses non-loopback binding unless a future enterprise release gate explicitly approves it.
* Starts disabled and stops when the user disables the feature or exits the app.
* Requires client authorization before meeting metadata or content is exposed.
* Enforces per-tool scopes and denies unauthorized transcript, summary, search, export, and mutation calls.
* Records content-free audit logs for allowed and denied sensitive calls.
* Terminates in-flight sessions after client revocation or global disable.

## External Integration Gate

Verify that:

* Calendar sync reads only selected calendars and stores minimal event metadata.
* Assisted join requires confirmation or an enabled automation scope.
* Manual exports preview destination, file name, format, and included content.
* Auto-export has an explicit destination, format, and template before it can run.
* Apple Notes export previews account/folder/note destination and included content.
* External write failures are visible, retryable, and not marked complete.
* Cloud summaries or chat disclose that selected meeting content is sent to the configured provider.
* Clearing local history does not imply deletion from external files, Notes, calendars, or provider-side logs.

## Screenshot Gate

Verify that:

* Screenshots never start from a global setting alone.
* Per-meeting confirmation appears before the first capture.
* The active meeting surface shows screenshot status, next-capture time, pause, resume, stop, and delete controls.
* Permission denial or revocation disables screenshots without breaking recording.
* Meeting deletion removes screenshot files and timeline references.
* Screenshot-derived speaker labels are shown as detected labels unless the user confirms them.

## Model, Provider, and Dependency Gate

Review new or changed dependencies for:

* License compatibility with the repository license and distribution model.
* Transcription engines, diarization packages, PDF/DOCX libraries, vector indexes, MCP packages, calendar packages, and Apple automation helpers.
* Native binary downloads, model weights, sidecars, and GPU backend requirements.
* Network behavior, telemetry, update checks, and cloud-provider defaults.
* Stored secrets, API keys, tokens, provider request metadata, and model cache deletion.

Dependency review output must name the package, version, license, source, runtime network behavior, and whether it ships in the desktop bundle.

## Manual QA Scenarios

Run the applicable manual QA before release:

* Fresh install or reset settings: sensitive automation remains off.
* Permission denied: feature remains unavailable with clear recovery path.
* Permission revoked mid-flow: active automation stops and state is visible.
* Delete meeting with derived artifacts: transcripts, summaries, screenshots, indexes, labels, export records, and meeting references are removed or invalidated.
* Failed export or Notes write: failure is visible and retryable.
* Calendar disconnect: sync stops and stale automation prompts disappear.
* MCP unauthorized client: no meeting content is returned.
* MCP client revocation: old credentials cannot be reused.
* Cloud provider opt-in: disclosure appears before sending meeting content.
* Screenshot capture: active indicator and next-capture timing remain visible until stopped.

## Automated Checks

Prefer automated coverage for:

* Default-off settings and migration defaults.
* Permission-denied and permission-revoked command paths.
* Meeting deletion cleanup for database records and app-managed files.
* Export destination validation and failed-write states.
* MCP unauthorized, revoked, and insufficient-scope requests.
* Log redaction for prompts, transcripts, screenshots, summaries, and raw tokens.

Minimum command expectations:

* Frontend-only changes: `pnpm run lint` from `frontend/`.
* Rust/Tauri command or storage changes: `cargo check` from `frontend/src-tauri/`.
* Package or release behavior changes: `pnpm run tauri:build` or the relevant platform helper script when feasible.
* macOS feature changes touching permissions, Apple Events, tray, local files, screenshots, or packaging: run `pnpm run tauri:dev` or `./clean_run.sh` for development verification, and document whether `pnpm run tauri:build` or `./clean_build.sh` was run.

## Release-Blocking Severity

Block release for:

* Sensitive automation enabled by default.
* Silent recording, screenshot capture, external export, Apple Notes write, calendar sync, MCP exposure, or cloud-provider request.
* Meeting content exposed to unauthorized clients or logs.
* Revocation that does not stop future access.
* Meeting deletion that leaves app-managed transcripts, summaries, screenshots, indexes, labels, or content-bearing logs accessible.
* External writes marked successful when they failed or partially completed.
* Missing README or in-app privacy disclosure for a sensitive feature.
* Unreviewed dependency that handles meeting content, credentials, native binaries, or network access.

Severity tiers:

* Critical: One of the release-blocking conditions above is present, or a feature can expose meeting content, credentials, screenshots, exports, prompts, or tokens without consent. Fix before merge and before any internal or external release build.
* Important: A required control exists but is incomplete, ambiguous, unaudited, untested, or unavailable on a supported platform. Fix before the feature release candidate, or waive with an owner, follow-up issue, and expiration no later than the next release candidate.
* Minor: Documentation, wording, or non-sensitive UX polish that does not weaken consent, revocation, deletion, auditability, or local-first defaults. Track before release and resolve by the next planned maintenance release unless the release owner accepts the residual risk.

Important findings must be fixed before release unless explicitly waived. Minor findings may ship only with a tracked follow-up and release-owner approval.

## Documentation Requirements

Before release, update user-readable documentation when behavior changes:

* README privacy notes for sensitive automation, external providers, and external destinations.
* Feature docs or settings copy for consent scope, revocation, audit history, and retention.
* Build docs when new environment requirements, native dependencies, sidecars, or packaging steps are added.
* Implementation issues must include completed checklist status or a link to the QA child that owns it.
