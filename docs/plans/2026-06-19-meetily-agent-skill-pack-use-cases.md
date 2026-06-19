# Meetily Agent Skill Pack Use Cases Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans, superpowers:subagent-driven-development, or superpowers:agent-teams-development to implement this plan. Ask the user which approach they prefer.

**Goal:** Expand the Meetily agent skill pack with practical personal and team workflows built on read-only MCP meeting context.

**Architecture:** Add read-only MCP tools for daily/weekly digest, open loops, next-meeting preparation, and role-based briefs. Extend the frontend workflow action catalog and prompt builder so the installed skill pack exposes these use cases without adding external writes or new scopes.

**Tech Stack:** Tauri/Rust MCP server in `frontend/src-tauri/src/mcp/mod.rs`, React/TypeScript workflow settings in `frontend/src/services/agentWorkflowService.ts` and `frontend/src/components/McpSettings.tsx`, documentation in `docs/meetily-mcp.md`.

---

### Task 1: Add Read-Only MCP Workflow Tools

**Files:**
- Modify: `frontend/src-tauri/src/mcp/mod.rs`

**Steps:**
1. Add tool schema entries for `meetily_get_daily_digest`, `meetily_get_weekly_digest`, `meetily_get_open_loops`, `meetily_prepare_next_meeting`, and `meetily_prepare_role_brief`.
2. Route these tools through the existing `meetings:read` scope.
3. Implement each tool by composing existing `find_meeting_ids`, `meeting_card`, `get_action_context_json`, and `matching_transcript_excerpts` helpers.
4. Keep outputs bounded and citation-friendly.
5. Run `cargo test --manifest-path frontend/src-tauri/Cargo.toml mcp --lib`.

### Task 2: Extend Skill Pack Actions and Prompts

**Files:**
- Modify: `frontend/src/services/agentWorkflowService.ts`
- Modify: `frontend/src/components/McpSettings.tsx`

**Steps:**
1. Add workflow actions for daily digest, weekly digest, next-meeting prep, open loops, project status, role brief, and decision log.
2. Update the generated handoff prompt with action-specific instructions.
3. Update skill pack copy so users see the broader workflow set.
4. Run `pnpm build`.

### Task 3: Update MCP Documentation

**Files:**
- Modify: `docs/meetily-mcp.md`

**Steps:**
1. Document the expanded personal and team workflows.
2. Clarify that all new workflows are read-only and proposal-only for external writes.
3. Run a targeted docs review and the relevant build checks.
