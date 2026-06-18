---
name: meetily-agent-workflows
description: Use when a Meetily task mentions repo commands, .agents workflows, review workflows, wrap-up gates, Tauri context routing, audio/transcription routing, or codebase agent instructions.
---

# Meetily Agent Workflows

This is a Codex bridge. The authoritative workflow text stays in the repo's `.agents/` tree and nearest `AGENTS.md` files; read the matching file before acting.

## Command Routing

- `/review`: read `.agents/commands/review.md`, then `.agents/agents/review.md`.
- `/review-and-fix`: read `.agents/commands/review-and-fix.md`, then `.agents/agents/review-and-fix.md`, then `.agents/agents/review.md`.
- `/fix-check-commit-close`: read `.agents/commands/fix-check-commit-close.md`, then `.agents/agents/fix-check-commit-close.md`, then `.agents/agents/review.md`.

## Context Routing

- Always read root `AGENTS.md`.
- For frontend work, also read `frontend/AGENTS.md`.
- For Tauri/Rust work, also read `frontend/src-tauri/AGENTS.md`.
- For archived backend work, also read `backend/AGENTS.md`.
- For local summary sidecar work, also read `llama-helper/AGENTS.md`.

## Codex Adaptation

- Use Codex tools that match the workflow intent.
- Keep `.agents/` files canonical for Meetily. Do not create `.cursor/` workflow files.
- Keep findings first for review output and avoid edits unless the user asks for fixes.
