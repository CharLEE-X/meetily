# RecallX Rebrand Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans, superpowers:subagent-driven-development, or superpowers:agent-teams-development to implement this plan. Ask the user which approach they prefer.

**Goal:** Rebrand the Meetily fork into RecallX and apply the Obsidian Archive product shell without breaking existing meeting, transcription, export, or automation behavior.

**Architecture:** Treat the work as a phased rebrand. First change user-facing brand metadata and docs while preserving MIT attribution, then introduce shared RecallX visual tokens and apply them to shell-level components, then redesign deeper workflows in later passes.

**Tech Stack:** Tauri 2, Rust, Next.js 14, React 18, TypeScript, Tailwind CSS, pnpm, GitHub Releases.

---

## Task 1: Inventory Current Brand Surfaces

**Files:**
- Read: `README.md`
- Read: `LICENSE.md`
- Read: `frontend/package.json`
- Read: `frontend/src-tauri/tauri.conf.json`
- Read: `frontend/src-tauri/Cargo.toml`
- Read: `frontend/src/components/`
- Read: `frontend/src/app/`
- Read: `docs/`

**Step 1: Search for current brand strings**

Run:

```bash
rg -n "Meetily|meetily|Zackriya|zackriya|meeting-minutes|com\\.meetily\\.ai|meetily\\.ai|CharLEE-X/meetily" README.md docs frontend Cargo.toml package.json .github
```

Expected: a list of files containing visible product names, upstream attribution, repo URLs, bundle IDs, and product copy.

**Step 2: Classify each match**

Create a temporary checklist with these buckets:

- Replace now: visible app product name, UI copy, docs copy, app title.
- Preserve: MIT license attribution and upstream acknowledgments.
- Review before changing: updater endpoints, GitHub repo URLs, release workflow names, package/crate names.

**Step 3: Commit no changes**

This task is inventory only.

## Task 2: Update Product Metadata to RecallX

**Files:**
- Modify: `frontend/package.json`
- Modify: `frontend/src-tauri/tauri.conf.json`
- Modify: `frontend/src-tauri/Cargo.toml`

**Step 1: Change package/app display names**

Set:

```json
"name": "recallx"
```

in `frontend/package.json`.

In `frontend/src-tauri/tauri.conf.json`, set:

```json
"productName": "RecallX"
```

and window title:

```json
"title": "RecallX"
```

Do not change the updater endpoint in this task unless a new release repository exists.

**Step 2: Review bundle identifier**

If the commercial bundle ID is ready, change:

```json
"identifier": "com.recallx.app"
```

If not ready, leave `com.meetily.ai` temporarily and add it to the follow-up list. Changing bundle ID affects app storage, updater identity, and signing.

**Step 3: Update Rust package metadata conservatively**

In `frontend/src-tauri/Cargo.toml`, update:

```toml
description = "Private meeting memory with AI"
repository = "https://github.com/CharLEE-X/meetily"
```

Keep `license = "MIT"`.

Rename package only if the crate is not referenced by generated config or build scripts. If safe:

```toml
name = "recallx"
```

**Step 4: Validate config**

Run:

```bash
cd frontend
pnpm run lint
```

Expected: lint passes or only existing unrelated warnings appear.

**Step 5: Commit**

```bash
git add frontend/package.json frontend/src-tauri/tauri.conf.json frontend/src-tauri/Cargo.toml
git commit -m "chore: rename product metadata to RecallX"
```

## Task 3: Update Public Docs and Attribution

**Files:**
- Modify: `README.md`
- Modify: `LICENSE.md` only if adding a separate notice is legally reviewed; otherwise do not change.
- Create: `NOTICE.md`
- Modify: selected files under `docs/`

**Step 1: Add attribution notice**

Create `NOTICE.md`:

```markdown
# RecallX Notices

RecallX is based on Meetily, originally created by Zackriya Solutions and licensed under the MIT License.

The original MIT copyright and permission notice is preserved in `LICENSE.md`.
```

**Step 2: Update README title and intro**

Replace visible product intro with:

```markdown
# RecallX

Private meeting memory with AI.

RecallX captures, transcribes, summarizes, and recalls meetings on local-first infrastructure. It is built for people and teams who need searchable meeting memory, reviewable follow-ups, and AI-assisted context without silent cloud capture.
```

Keep a license/attribution paragraph:

```markdown
RecallX is a fork of Meetily and preserves the original MIT license attribution in `LICENSE.md`.
```

**Step 3: Replace docs brand mentions**

For docs that describe current supported product behavior, replace `Meetily` with `RecallX`.

Do not replace:

- `LICENSE.md`
- historical attribution text
- upstream links used for acknowledgement
- migration notes that explicitly discuss previous Meetily versions

**Step 4: Check the diff**

Run:

```bash
git diff --check
git diff -- README.md NOTICE.md docs
```

Expected: no whitespace errors; attribution preserved.

**Step 5: Commit**

```bash
git add README.md NOTICE.md docs
git commit -m "docs: rebrand public docs to RecallX"
```

## Task 4: Create RecallX Design Tokens

**Files:**
- Modify: `frontend/tailwind.config.js` or nearest Tailwind config
- Modify: `frontend/src/app/globals.css`
- Create or modify: shared UI token file if one already exists

**Step 1: Locate styling entry points**

Run:

```bash
cd frontend
rg -n "tailwind|globals.css|@tailwind|theme|colors|fontFamily" .
```

Expected: find Tailwind config and global CSS.

**Step 2: Add RecallX color tokens**

Add tokens equivalent to:

```js
recallx: {
  black: "#050505",
  graphite: "#0B0B0A",
  panel: "#10110F",
  line: "rgba(255,255,255,0.10)",
  text: "#F7F4EE",
  muted: "rgba(247,244,238,0.68)",
  acid: "#C8FF85"
}
```

**Step 3: Add motion token utilities**

Add CSS custom properties:

```css
:root {
  --recallx-motion: cubic-bezier(0.32, 0.72, 0, 1);
}
```

**Step 4: Do not globally force dark mode yet**

Keep tokens available without breaking every existing component.

**Step 5: Validate**

Run:

```bash
cd frontend
pnpm run lint
```

Expected: pass.

**Step 6: Commit**

```bash
git add frontend/tailwind.config.* frontend/src/app/globals.css
git commit -m "style: add RecallX design tokens"
```

## Task 5: Build Shared Obsidian Shell Components

**Files:**
- Create: `frontend/src/components/recallx/RecallXShell.tsx`
- Create: `frontend/src/components/recallx/RecallXCard.tsx`
- Create: `frontend/src/components/recallx/RecallXButton.tsx`
- Create: `frontend/src/components/recallx/index.ts`

**Step 1: Create double-bezel card component**

Implement:

```tsx
import { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function RecallXCard({
  children,
  className,
  innerClassName,
}: {
  children: ReactNode;
  className?: string;
  innerClassName?: string;
}) {
  return (
    <div className={cn("rounded-[2rem] bg-white/[0.06] p-1.5", className)}>
      <div
        className={cn(
          "rounded-[calc(2rem-0.375rem)] bg-[#0B0B0A] shadow-[inset_0_1px_1px_rgba(255,255,255,0.14)]",
          innerClassName
        )}
      >
        {children}
      </div>
    </div>
  );
}
```

If `@/lib/utils` or `cn` does not exist, use the nearest local className helper or create a tiny helper following existing patterns.

**Step 2: Create button-in-button CTA**

Implement a button with rounded pill shape, custom motion timing, active scale, and nested trailing icon circle.

**Step 3: Create shell wrapper**

`RecallXShell` should provide:

- OLED background.
- Fixed subtle noise/line overlay.
- Main content min height.
- Responsive padding.

**Step 4: Export components**

Add exports from `frontend/src/components/recallx/index.ts`.

**Step 5: Validate**

Run:

```bash
cd frontend
pnpm run lint
```

Expected: pass.

**Step 6: Commit**

```bash
git add frontend/src/components/recallx
git commit -m "feat: add RecallX shell components"
```

## Task 6: Apply RecallX Shell to Main App Surface

**Files:**
- Modify: `frontend/src/app/page.tsx`
- Modify: `frontend/src/components/Sidebar/index.tsx`
- Modify: `frontend/src/components/RecordingControls.tsx`
- Modify: `frontend/src/components/RecordingAuditTrail.tsx`

**Step 1: Wrap main page in RecallX shell**

Use `RecallXShell` at the highest safe level in `frontend/src/app/page.tsx`.

**Step 2: Replace top-level panels with `RecallXCard`**

Start with:

- recording preflight panel
- current meeting status panel
- recording controls enclosure
- audit trail enclosure

**Step 3: Update sidebar branding**

Replace visible app name and version block with RecallX visual identity.

**Step 4: Preserve behavior**

Do not change hooks, service calls, recording command parameters, or consent logic.

**Step 5: Validate**

Run:

```bash
cd frontend
pnpm run lint
```

Expected: pass.

**Step 6: Manual check**

Run:

```bash
cd frontend
pnpm run dev
```

Open `http://localhost:3118` and verify:

- main surface loads
- no overlapping text at desktop width
- recording controls still render
- preflight and audit trail still visible

**Step 7: Commit**

```bash
git add frontend/src/app/page.tsx frontend/src/components/Sidebar/index.tsx frontend/src/components/RecordingControls.tsx frontend/src/components/RecordingAuditTrail.tsx
git commit -m "feat: apply RecallX app shell"
```

## Task 7: Apply RecallX Shell to Meeting Details and Settings

**Files:**
- Modify: `frontend/src/app/meeting-details/`
- Modify: `frontend/src/components/SettingTabs.tsx`
- Modify: `frontend/src/components/PreferenceSettings.tsx`
- Modify: `frontend/src/components/McpSettings.tsx`

**Step 1: Update meeting details container**

Apply dark RecallX background and double-bezel containers to the page shell. Preserve all child workflow state.

**Step 2: Update settings shell**

Apply RecallX panel treatment to settings navigation and preference panels without redesigning every nested setting.

**Step 3: Update MCP/agent surfaces**

Use memory/agent language:

- "Post-meeting agent workflows"
- "Recall context package"
- "Review before run"

Keep all current safety wording.

**Step 4: Validate**

Run:

```bash
cd frontend
pnpm run lint
```

Expected: pass.

**Step 5: Manual check**

Open meeting details and settings in the dev app. Verify no text overlap and no missing controls.

**Step 6: Commit**

```bash
git add frontend/src/app/meeting-details frontend/src/components/SettingTabs.tsx frontend/src/components/PreferenceSettings.tsx frontend/src/components/McpSettings.tsx
git commit -m "feat: apply RecallX detail and settings shell"
```

## Task 8: Run Final Brand Audit

**Files:**
- All project files

**Step 1: Search for old visible brand**

Run:

```bash
rg -n "Meetily|meetily|Zackriya|zackriya|meeting-minutes|com\\.meetily\\.ai|meetily\\.ai" README.md docs frontend Cargo.toml package.json .github
```

Expected:

- Only preserved MIT attribution, upstream acknowledgement, migration notes, and intentional repository/updater references remain.
- Any visible UI references are changed to RecallX.

**Step 2: Run checks**

Run:

```bash
cd frontend
pnpm run lint
```

If Rust metadata changed, run:

```bash
cd frontend/src-tauri
cargo check
```

**Step 3: Build smoke test if time allows**

Run:

```bash
cd frontend
pnpm run build
```

Expected: Next.js build succeeds.

**Step 4: Commit any final fixes**

```bash
git add .
git commit -m "chore: complete RecallX rebrand audit"
```

