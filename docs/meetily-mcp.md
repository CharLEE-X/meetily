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
