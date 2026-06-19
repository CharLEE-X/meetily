# Post-Meeting Agent Orchestration Release Notes

This release adds local post-meeting handoff workflows for Codex, Claude
Desktop, Cursor, and manual MCP clients.

## Included

* Local settings for preferred agent, trigger mode, context budget, content
  sources, prompt template, and automation rules.
* Source-cited context packages built from approved meeting summary data and
  optional sensitive sources.
* Prompt templates for implementation handoff, repo investigation, PR review,
  docs update, Linear/Jira grooming, incident follow-up, product planning, and
  open-loop review.
* Per-agent readiness checks for MCP endpoint configuration and local fallback
  capability.
* Ask and auto modes after summary completion.
* Copyable prompt fallback for unsupported or failed direct invocation.
* Local run history with status, timestamps, template, budget/source metadata,
  audit event type, and manually editable outcome links.

## Recommended Use

Use Codex when the follow-up needs repository inspection, implementation, test
work, PR review, local build debugging, or developer workflow execution.

Use Claude when the follow-up is primarily planning, synthesis, issue grooming,
product analysis, incident follow-up, documentation, or stakeholder messaging.

Use manual handoff for sensitive meetings, unclear setup, or when you want to
inspect the context package before involving any agent.

## Limitations

* Meetily does not directly launch Codex or Claude tasks in this release. The
  stable supported path is a prepared prompt plus MCP references.
* Direct invocation adapters are intentionally conservative and fall back to a
  copyable prompt unless a future local client exposes a stable supported
  launch mechanism.
* Meetily cannot observe all downstream agent progress. Users can add outcome
  links manually after the agent finishes work.
* Run history stores metadata only. It does not store raw prompts, transcript
  text, screenshot OCR, MCP tokens, API keys, or third-party auth responses.

## Out Of Scope

Meetily does not create GitHub/GitLab branches, commits, pull requests, Linear
issues, Jira issues, docs changes, local code edits, or external messages as
part of this orchestration. Codex, Claude, Cursor, or another authorized agent
must own those actions through their own tools and approval flows.

## QA Checklist

* Off mode creates no post-meeting run.
* Ask mode prepares a run and requires the user to copy or trigger manually.
* Auto mode attempts the adapter only after settings, readiness, and source
  gates pass.
* Missing MCP setup or unavailable agent readiness blocks auto mode.
* Copy prompt fallback preserves context and records a `fallbackReady` run.
* Direct invocation failure falls back without losing the prompt.
* Settings history shows cross-meeting runs.
* Meeting details shows runs for the selected meeting.
* Outcome links can be added and removed.
* No raw prompt, transcript, screenshot OCR, token, or secret appears in run
  history, MCP audit logs, or user-safe errors.
