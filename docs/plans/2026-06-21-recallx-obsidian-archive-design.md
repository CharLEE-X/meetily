# RecallX Obsidian Archive Design

## Goal

Rebrand the Meetily fork into RecallX, a legally distinct private meeting memory product with a premium Obsidian Archive visual system.

## Positioning

RecallX is a private AI memory layer for every meeting. It turns conversations into local, searchable memory records with summaries, transcript citations, screenshots, follow-ups, Apple workflows, and reviewable agent handoffs.

The product should feel like a secured archive, not a generic meeting recorder. AI is present as a quiet intelligence layer that helps users recall, review, and act on meeting context without silently exporting or automating sensitive data.

## Selected Visual Direction

Use the Obsidian Archive direction from `brand-previews/recallx/obsidian-archive.html`.

Core attributes:

- OLED black and graphite surfaces.
- Pale typography with very high contrast.
- Acid-lime memory accent for active states, recall markers, and primary actions.
- Double-bezel panels that feel like machined glass or hardware.
- Calm, precise motion using custom cubic-bezier timing.
- Secure local archive metaphor rather than cloud assistant styling.

## Brand System

Name: RecallX

Tagline options:

- Private meeting memory with AI.
- Your conversations, sealed into local recall.
- A private AI memory layer for every meeting.

Primary tagline for app and README:

> Private meeting memory with AI.

Brand vocabulary:

- Memory
- Recall
- Archive
- Local context
- Review before write
- Private AI
- Meeting record
- Handoff
- Audit trail

Avoid:

- Meetily
- Zackriya
- "meeting minutes" as a product name
- Any copy implying official affiliation with upstream

## Legal Separation Requirements

The repo is MIT licensed, so the fork can be modified, renamed, published, and sold if the MIT notice is retained. RecallX must keep upstream MIT attribution in distributed copies.

Required separation work:

- Keep `LICENSE.md` with Zackriya Solutions copyright.
- Add a short attribution notice where appropriate, such as in README or an About view.
- Replace visible Meetily product branding in the app, docs, metadata, and release surfaces.
- Replace app bundle identifiers, updater endpoints, icons, screenshots, marketing links, and support links before commercial distribution.
- Do not imply RecallX is official Meetily or affiliated with Zackriya Solutions.

## Implementation Sequence

### Phase 1: Brand Foundation

Rename product metadata and visible brand references from Meetily to RecallX while preserving upstream license attribution.

Scope:

- App product name and window title.
- Package metadata where safe.
- README and public docs.
- Updater/release naming where it does not break existing published channels.
- About/settings labels.
- Browser title and install labels.

Do not rename deep Rust crate/module paths unless needed for distribution. Internal code identifiers can remain temporarily if changing them would add risk without user-facing benefit.

### Phase 2: App Shell Redesign

Apply the Obsidian Archive system to the shared UI shell before redesigning every feature workflow.

Scope:

- Main app surface.
- Sidebar/navigation.
- Recording preflight.
- Primary cards and panels.
- Settings shell.
- Meeting detail shell.
- Shared button/card/input tokens.

Goal:

The app should immediately read as RecallX even if some deep panels still use transitional layouts.

### Phase 3: Deep Workflow Redesign

Redesign feature surfaces after the shell is stable.

Scope:

- Meeting details.
- Transcript and speaker correction.
- Screenshot timeline.
- Meeting chat.
- Summary templates.
- Apple Notes and Reminders panels.
- MCP and agent workflow panels.
- Export flows.

Each deep workflow should preserve current behavior and consent gates.

## UI Principles

- No generic gray bordered cards. Major panels use double-bezel containers.
- No broad purple/blue gradient SaaS look.
- No oversized marketing landing page inside the desktop app.
- Use dense but calm operational UI for repeated workflows.
- Preserve explicit permission and review-before-write copy.
- Motion must use transform and opacity, not layout-triggering properties.
- Avoid hiding important consent state behind decorative visuals.

## Verification

Phase 1:

- Search for visible `Meetily`, `meetily`, upstream URLs, and old bundle IDs.
- Run frontend lint if UI copy changes touch TypeScript.
- Confirm app metadata changes do not break Tauri config parsing.

Phase 2:

- Run `pnpm run lint` from `frontend/`.
- Launch the app and manually check desktop widths for main shell, settings, and meeting detail.
- Capture screenshots for review.

Phase 3:

- Run targeted tests for changed services.
- Manually check recording preflight, meeting detail, export, Apple Notes, Reminders, chat, and agent panels.
- Confirm consent gates still appear before external writes or agent runs.

