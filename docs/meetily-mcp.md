# Meetily Local MCP Contract

Meetily exposes meeting data to local AI agents through an embedded MCP server. The
server is a local-only, opt-in bridge for approved clients; it is not a public API
and must not bind to non-loopback interfaces.

## Defaults

* MCP is disabled by default.
* When enabled, the server binds to `127.0.0.1` only.
* Initial meeting tools are read-only.
* Write, delete, mutation, and external export tools are out of scope for the first
  MCP release.
* Meeting content tools require a trusted client token and matching scopes.

## Client Authorization

Trusted clients are represented by local records with:

* client id;
* display name;
* allowed scopes;
* token fingerprint;
* token hash;
* expiry timestamp;
* revoked flag.

Meetily stores token hashes for validation and shows only fingerprints in the UI.
Raw tokens must not be written to Meetily logs, audit logs, docs, or visible
Settings text. Agent setup may write a local client configuration that passes the
token to that agent, but the app UI should expose only the fingerprint afterward.

Requests for meeting-content tools must include `Authorization: Bearer <token>`.
Unauthorized, expired, revoked, or insufficient-scope requests must return a
structured MCP error and must not include meeting content.

## Scopes

| Scope | Allows |
| --- | --- |
| `mcp:read_status` | health, initialization, and status tool calls |
| `meetings:list` | list meeting IDs and basic metadata |
| `meetings:read` | read one meeting, summary, transcript, action items, and artifact metadata |
| `meetings:search` | search transcript snippets across meetings |

The default generated Meetily client receives the read-only scopes above. Future
write scopes need a separate threat model and release gate.

## Tools

### `meetily_status`

Returns server name, version, transport, and read-only policy status. This tool
does not expose meeting content.

Input schema:

```json
{ "type": "object", "properties": {} }
```

### `meetily_list_meetings`

Lists meeting metadata available to the authorized client.

Scope: `meetings:list`

Input schema:

```json
{
  "type": "object",
  "properties": {
    "limit": { "type": "integer", "minimum": 1, "maximum": 100 },
    "query": { "type": "string" }
  }
}
```

### `meetily_search_transcripts`

Searches transcript text and returns bounded snippets, not full transcripts.

Scope: `meetings:search`

Input schema:

```json
{
  "type": "object",
  "required": ["query"],
  "properties": {
    "query": { "type": "string", "minLength": 1 },
    "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
  }
}
```

### `meetily_get_meeting`

Returns meeting metadata plus transcript/summary availability.

Scope: `meetings:read`

Input schema:

```json
{
  "type": "object",
  "required": ["meetingId"],
  "properties": {
    "meetingId": { "type": "string" }
  }
}
```

### `meetily_get_summary`

Returns the stored summary payload for a meeting when available.

Scope: `meetings:read`

Input schema:

```json
{
  "type": "object",
  "required": ["meetingId"],
  "properties": {
    "meetingId": { "type": "string" }
  }
}
```

### `meetily_get_transcript`

Returns transcript segments for a meeting, bounded by limit and offset.

Scope: `meetings:read`

Input schema:

```json
{
  "type": "object",
  "required": ["meetingId"],
  "properties": {
    "meetingId": { "type": "string" },
    "limit": { "type": "integer", "minimum": 1, "maximum": 500 },
    "offset": { "type": "integer", "minimum": 0 }
  }
}
```

### `meetily_get_action_items`

Returns action item fields stored with transcript rows and summary payloads.

Scope: `meetings:read`

Input schema:

```json
{
  "type": "object",
  "required": ["meetingId"],
  "properties": {
    "meetingId": { "type": "string" }
  }
}
```

### `meetily_get_artifacts`

Returns safe artifact metadata for a meeting. It must not return raw local file
paths unless a future explicit file-access scope is added.

Scope: `meetings:read`

Input schema:

```json
{
  "type": "object",
  "required": ["meetingId"],
  "properties": {
    "meetingId": { "type": "string" }
  }
}
```

### Read-only workflow tools

The MCP server also exposes read-only workflow tools for common agent use cases.
All require `meetings:read`, return bounded context, and include guidance for
source-backed answers:

* `meetily_get_latest_meeting`: latest meeting context.
* `meetily_find_meetings`: topic, person, title, or date-range lookup.
* `meetily_ask_meetings`: answer-ready context for natural questions such as
  "what did we say on the last call with X?"
* `meetily_get_recent_action_items`: recent follow-ups across meetings.
* `meetily_get_decisions`: decision-like summaries and excerpts.
* `meetily_get_followups_for_person`: commitments and follow-ups involving a
  person.
* `meetily_get_meeting_brief`: compact meeting brief for one meeting or the
  latest meeting.
* `meetily_compare_meetings`: compare related meetings.
* `meetily_get_project_context`: timeline for a project/topic.
* `meetily_get_daily_digest`: personal daily meeting digest.
* `meetily_get_weekly_digest`: weekly digest across commitments, risks,
  decisions, and themes.
* `meetily_get_open_loops`: unresolved questions, ownerless actions, risks, and
  confirmations.
* `meetily_prepare_next_meeting`: prep brief from previous related meetings.
* `meetily_prepare_role_brief`: role-specific product, engineering, sales,
  hiring, manager, founder, or customer-success brief.
* `meetily_prepare_handoff`: Codex, Claude, Cursor, Linear, or manual handoff
  prompt from meetings or a topic.

## Audit Events

Every meeting-content tool call records:

* timestamp;
* client id;
* tool name;
* scopes used;
* opaque meeting ids accessed;
* result: `allowed`, `denied`, `revoked`, or `failed`;
* denial/failure reason when applicable.

Audit events must not include meeting titles, transcript text, summary text,
prompts, screenshots, embeddings, local file paths, or raw tokens.

## Agent Setup

Use `Settings -> MCP -> Agent setup` to configure supported local clients:

* Claude Desktop;
* Codex;
* Cursor.

The app writes a Meetily MCP entry for the selected client and creates a matching
trusted-client record. Re-running setup is idempotent while the client is already
configured with an active token; if the client is missing, expired, or revoked,
setup writes a fresh local token. After setup, Settings shows the client name,
scopes, token fingerprint, expiry, and revoke state.

Client records are stored in Meetily's local config directory in
`mcp_clients.json`. The registry stores token hashes and fingerprints, not raw
tokens. The agent configuration file receives the raw token because the agent
needs it to call the local server, but Meetily should never display that token in
Settings or audit logs.

## Agent Skill Pack and Post-Meeting Workflows

Meetily also provides a local agent skill pack from `Settings -> MCP`. The skill
pack is default-off, reversible, and stores only workflow templates plus MCP
endpoint references. It must not write meeting content, raw client tokens, API
keys, or secrets into agent configuration files.

The broader Codex/Claude orchestration contract is defined in
[Post-Meeting Agent Orchestration](post-meeting-agent-orchestration.md). That
document is the source of truth for trigger modes, consent boundaries,
Codex-vs-Claude responsibilities, invocation fallback, run status lifecycle, and
content-free audit metadata.

The first skill pack contains workflows for:

* meeting search and meeting lookup through the authorized MCP server;
* last-meeting recall, topic search, person search, and meeting comparison;
* summary review and missing-context checks;
* follow-up and action extraction;
* personal daily and weekly meeting digests;
* next-meeting preparation from previous related calls;
* open-loop review for unresolved questions, ownerless actions, risks, and
  confirmations;
* role-based briefs for product, engineering, sales, hiring, manager, founder,
  and customer-success workflows;
* project status updates and decision-log extraction;
* Linear follow-up issue proposals;
* follow-up message drafting;
* manual agent handoff prompts.

The packaged MCP workflows are read-only. They return bounded source context and
agent instructions, not external writes. Use cases that produce Linear issues,
status updates, customer follow-ups, decision logs, or docs must remain drafts
until the user explicitly approves the destination and content in a separate
write flow.

Post-meeting workflows have three modes:

| Mode | Behavior |
| --- | --- |
| Off | No post-meeting agent workflow is prepared or run. |
| Ask before running | Meetily prepares a bounded handoff after summary completion and asks the user to copy/run it. |
| Prepare handoff automatically | Meetily prepares the handoff automatically after summary completion, but external writes still require approval. |

Linear follow-up workflows are proposal-only by default. They ask the selected
agent to return reviewable issue drafts with title, description, owner if known,
priority suggestion, source meeting reference, and confidence. Meetily must not
create Linear issues unless the user explicitly reviews and approves the write in
a future authorized Linear write flow.

### Agent Invocation Support Matrix

| Agent target | MCP setup | Direct invocation from Meetily | Fallback |
| --- | --- | --- | --- |
| Codex | Meetily can write a `meetily` MCP server entry to `~/.codex/config.toml`. | Not launched directly in this release. | Copy the generated prompt into Codex; the prompt references the local MCP endpoint. |
| Claude Desktop | Meetily can write a `meetily` MCP server entry to Claude Desktop config. | Not launched directly in this release. | Open Claude and paste the generated prompt after setup. |
| Cursor | Meetily can write a `meetily` MCP server entry to `~/.cursor/mcp.json`. | Not launched directly in this release. | Open Cursor and paste the generated prompt after setup. |
| Manual MCP client | User configures the documented MCP endpoint and trusted client token flow. | Manual only. | Copy the generated prompt into any authorized local MCP client. |

Unsupported direct-invocation paths must degrade to manual handoff instead of
pretending the workflow has run. Local workflow logs may store meeting id, agent,
action template, mode, status, and timestamp, but must not store transcript text,
summary bodies, screenshots, prompts, raw tokens, or external credentials.

## Connection

When enabled, the server URL is:

```text
http://127.0.0.1:<configured-port>/mcp
```

The default port is `43118`. The health endpoint is:

```text
http://127.0.0.1:<configured-port>/health
```

External MCP clients must send:

```text
Authorization: Bearer <client-token>
```

`initialize`, `tools/list`, and `meetily_status` are safe discovery/status paths.
Meeting-content tools require the bearer token and a matching scope.

## Settings UX

The MCP Settings page must show:

* server enabled/running/error state;
* loopback URL and configured port;
* auto-start preference;
* supported agent setup status;
* trusted clients with active, expired, or revoked state;
* token fingerprints, not raw tokens;
* recent audit events with client id, tool, scopes, meeting ids, result, and
  reason when applicable.

Revoking a client marks the client record as revoked. Subsequent meeting-content
tool calls with that token must fail with a revoked authorization error and log a
revoked audit event.

## QA Checklist

Before shipping MCP meeting access:

* Confirm MCP is disabled by default.
* Enable MCP and verify `/health` responds only on `127.0.0.1`.
* Verify `tools/list` includes only read-only tools.
* Call a meeting-content tool without `Authorization`; expect no meeting content
  and a denied audit event.
* Configure at least one agent from Settings; verify a trusted client appears
  with a fingerprint and scopes.
* Call a read-only meeting tool with that agent token; expect scoped meeting data
  and an allowed audit event.
* Revoke that client and retry the same token; expect no meeting content and a
  revoked audit event.
* Restart the app with auto-start enabled and verify the server starts on the
  configured loopback port.
* Disable MCP and verify the server stops.
* Inspect audit logs and app logs for absence of raw tokens, transcript text,
  summary text, prompts, screenshots, embeddings, and local file paths.
