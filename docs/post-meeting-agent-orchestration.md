# Post-Meeting Agent Orchestration

Meetily can prepare and hand off meeting context to local agents such as Codex
and Claude after a summary completes. Meetily owns capture, local context,
consent, packaging, triggering, and auditability. The agent owns downstream
tool work such as code changes, GitHub/GitLab, Linear/Jira, docs, research, and
repo workflow execution.

This contract exists to prevent Meetily from rebuilding agent capabilities. If
a workflow requires repository reasoning, external issue creation, pull
requests, commits, messages, or multi-step tool execution, Meetily should
prepare the smallest useful context package and delegate the work to the
selected agent.

## Architecture

The orchestration surface has five local parts:

| Layer | Meetily responsibility | Agent responsibility |
| --- | --- | --- |
| Settings | Preferred agent, trigger mode, context budget, consent toggles, template/rule selection. | None. |
| Context package | Build bounded, source-cited meeting context from local summary, action items, approved excerpts, meeting metadata, and selected artifact links. | Read package and request more context through authorized MCP tools when needed. |
| Prompt template | Render the selected template with clear boundaries, source references, and expected output format. | Follow the prompt and use available agent tools for implementation or planning work. |
| Trigger adapter | Use a supported local launch/handoff path, or fall back to a copyable prompt. | Execute in Codex, Claude, or another authorized client. |
| Run history | Store status, ids, hashes, agent, template, budget, trigger mode, timestamps, and user-visible outcome links. | Return or expose outcomes such as branch, PR, issue, document, or needs-input notes. |

Meetily must not create issues, PRs, commits, external messages, or repo changes
as a side effect of this orchestration. Those actions remain agent-owned and
subject to the agent's own confirmations, permissions, and tool policies.

## Trigger Modes

| Mode | Behavior |
| --- | --- |
| Off | No context package is prepared after summary completion. Manual handoff actions may still be available from meeting details. |
| Ask before triggering | Meetily prepares a context package and prompt, then waits for explicit user approval before launching, copying, or handing off to an agent. |
| Auto-trigger | Meetily prepares the package and attempts the configured supported adapter automatically after summary completion. If the adapter is unavailable or unsafe, it records `waiting_for_approval` and shows a copyable prompt instead. |

Auto-trigger is still not consent for external writes. It only starts or hands
off the agent run. The agent remains responsible for any separate approval
needed before writing to GitHub, GitLab, Linear, Jira, local files, docs, or
other external systems.

## Consent Rules

All post-meeting agent automation is off by default. Consent is layered by
content type because a useful engineering handoff may not need the full meeting.

| Content | Default | Consent rule |
| --- | --- | --- |
| Summary | Excluded until a post-meeting workflow is enabled or manually prepared. | Allowed for enabled workflows because summaries are the primary handoff input. |
| Action items | Excluded until a workflow is enabled or manually prepared. | Allowed when summary context is allowed; each item should keep source section or timestamp where available. |
| Transcript excerpts | Excluded. | Include only bounded excerpts selected by template, budget, or explicit user setting. Full transcript export to agents requires a separate explicit toggle. |
| Screenshots and OCR | Excluded. | Never include by default. Requires screenshot capture consent plus a separate agent-context/OCR consent gate. |
| Speaker labels | Excluded unless already confirmed by the user. | Include confirmed labels only; unconfirmed labels must be marked as inferred or omitted. |
| Calendar metadata | Excluded. | Include title/time/provider/artifact links only when Calendar integration is enabled and the workflow setting allows artifact links. |
| Apple Notes, Reminders, exports, MCP links | Excluded. | Include only metadata and links for integrations the user enabled. Do not copy external content back into the package unless separately approved. |
| Repository paths, project names, customer names | Excluded unless present in approved meeting context or rule settings. | Include when the user selects a rule/template that needs repo or project targeting. |

Context packages should prefer concise source references over raw content:
meeting id, summary section, action item id, timestamp, transcript segment id,
calendar event id, export id, or MCP query hint. Raw private content should be
bounded by the selected context budget and omitted from run history.

## Codex And Claude Responsibilities

| Agent | Recommended use | Why |
| --- | --- | --- |
| Codex | Code implementation follow-ups, repo investigation, tests, PR preparation, local build failures, GitHub/GitLab development work, Linear/Jira issue execution when the repo is involved. | Codex has strong local-codebase and developer workflow affordances. |
| Claude | Planning, research, product/spec synthesis, long-form docs, issue grooming, risk analysis, meeting follow-up drafts, cross-functional summaries. | Claude is useful for broad synthesis and writing-heavy work, especially when direct repo edits are not required. |
| Manual handoff | Sensitive meetings, unclear tasks, unsupported local agent setup, or workflows where the user wants to inspect the package before using any agent. | Keeps the user in control and avoids pretending automation ran when no reliable adapter exists. |

Meetily can recommend a default agent by template, but user selection wins.
Templates must not assume the agent has access to a specific third-party tool;
they should instruct the agent to verify available tools before acting.

## Supported Invocation And Fallback

Supported invocation is adapter-specific and must be conservative.

| Target | Supported path | Fallback |
| --- | --- | --- |
| Codex | Local Codex MCP setup plus handoff prompt. A future adapter may open a Codex task only if the local client exposes a stable supported mechanism. | Copyable prompt with MCP endpoint and source ids. |
| Claude Desktop | Local Claude Desktop MCP setup plus handoff prompt. Direct task launch is unsupported unless Claude exposes a stable local adapter. | Copyable prompt for Claude Desktop. |
| Cursor | MCP setup and manual prompt handoff only. Cursor automation is not a primary target for this epic. | Copyable prompt. |
| Manual | Copy prompt and package references. | Same path. |

If a configured adapter cannot prove that it launched the agent with the
intended prompt, Meetily must not mark the run completed. It should record
`waiting_for_approval` or `failed` with a user-safe reason and keep the prompt
available.

## Run Status Lifecycle

| Status | Meaning |
| --- | --- |
| `prepared` | Context package and prompt were generated locally. |
| `waiting_for_approval` | User approval is required before handoff or trigger. |
| `triggered` | Meetily handed the package to a supported adapter or opened a handoff target. |
| `running_unknown` | The agent was triggered but Meetily cannot observe progress. |
| `needs_input` | The run requires user clarification, missing setup, or unavailable agent capability. |
| `failed` | Preparation or trigger failed before a useful agent handoff. |
| `completed` | User or supported adapter marked the run complete. |
| `linked` | The run has one or more outcome links such as PR, branch, issue, doc, or local file reference. |

`running_unknown` is expected for most local handoff paths. The UI should be
honest about limited observability rather than implying Meetily controls the
agent.

## Audit And Retention

Run history stores metadata, not meeting bodies:

* run id;
* meeting id;
* agent target;
* trigger mode;
* template id and version;
* context budget;
* content scope flags;
* context package hash;
* prompt hash;
* status and timestamps;
* user approval timestamp when applicable;
* setup/readiness snapshot;
* user-safe error code/message;
* outcome links and link labels.

Run history must not store raw transcript text, raw screenshots/OCR, full
prompts, raw MCP tokens, API keys, repository credentials, or third-party auth
responses. If the user deletes a meeting, associated run history should retain
only minimal audit metadata unless the user explicitly keeps outcome links.

## Implementation References

Initial implementation should live in:

* `frontend/src/services/agentWorkflowService.ts` for frontend settings,
  templates, local run preparation, and fallback prompt behavior.
* `frontend/src/components/McpSettings.tsx` for global settings, readiness, and
  agent support status.
* `frontend/src/components/MeetingDetails/` for meeting-level preparation,
  approval, trigger, and run-history UI.
* Future native persistence may use `frontend/src-tauri/src/agent_workflows/`
  and a SQLite migration when localStorage is no longer sufficient.

This design depends on the local MCP contract in [Meetily Local MCP
Contract](meetily-mcp.md) and the release gate in [Privacy, Consent, and Access
Controls](privacy-consent-access-controls.md).
