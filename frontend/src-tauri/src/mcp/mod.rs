use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, io, path::PathBuf, sync::Arc, sync::Mutex as StdMutex, time::Duration};
use tauri::{AppHandle, Manager, State};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{oneshot, Mutex},
    task::JoinHandle,
    time::timeout,
};
use uuid::Uuid;

use crate::{
    database::repositories::{
        meeting::MeetingsRepository, summary::SummaryProcessesRepository,
        transcript::TranscriptsRepository,
    },
    state::AppState,
};

const DEFAULT_PORT: u16 = 43118;
const CONFIG_DIR_NAME: &str = "meetily";
const SETTINGS_FILE_NAME: &str = "mcp_settings.json";
const AUDIT_FILE_NAME: &str = "mcp_audit_log.json";
const CLIENTS_FILE_NAME: &str = "mcp_clients.json";
const DEFAULT_CLIENT_TTL_DAYS: i64 = 30;

const SCOPE_READ_STATUS: &str = "mcp:read_status";
const SCOPE_LIST_MEETINGS: &str = "meetings:list";
const SCOPE_READ_MEETINGS: &str = "meetings:read";
const SCOPE_SEARCH_MEETINGS: &str = "meetings:search";

static AUDIT_FILE_LOCK: std::sync::LazyLock<StdMutex<()>> =
    std::sync::LazyLock::new(|| StdMutex::new(()));
static CLIENTS_FILE_LOCK: std::sync::LazyLock<StdMutex<()>> =
    std::sync::LazyLock::new(|| StdMutex::new(()));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    pub enabled: bool,
    pub auto_start: bool,
    pub port: u16,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_start: false,
            port: DEFAULT_PORT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum McpServerState {
    Stopped,
    Starting,
    Running,
    Error,
}

impl Default for McpServerState {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub settings: McpSettings,
    pub state: McpServerState,
    pub bind_host: String,
    pub port: u16,
    pub url: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpClient {
    pub id: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub token_fingerprint: String,
    pub expires_at: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrustedClientRecord {
    id: String,
    name: String,
    scopes: Vec<String>,
    token_hash: String,
    token_fingerprint: String,
    expires_at: String,
    revoked: bool,
    created_at: String,
    updated_at: String,
}

impl From<TrustedClientRecord> for McpClient {
    fn from(client: TrustedClientRecord) -> Self {
        Self {
            id: client.id,
            name: client.name,
            scopes: client.scopes,
            token_fingerprint: client.token_fingerprint,
            expires_at: client.expires_at,
            revoked: client.revoked,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAuditEvent {
    pub id: String,
    pub timestamp: String,
    pub client_id: String,
    pub tool_name: String,
    pub scopes: Vec<String>,
    pub meeting_ids: Vec<String>,
    pub result: String,
    pub reason: Option<String>,
}

#[derive(Debug, Default)]
struct McpRuntime {
    settings: McpSettings,
    state: McpServerState,
    last_error: Option<String>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

#[derive(Debug, Default)]
pub struct McpState {
    runtime: Arc<Mutex<McpRuntime>>,
}

fn config_dir() -> Result<PathBuf> {
    let mut path = dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
    path.push(CONFIG_DIR_NAME);
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn settings_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(SETTINGS_FILE_NAME))
}

fn audit_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(AUDIT_FILE_NAME))
}

fn clients_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CLIENTS_FILE_NAME))
}

fn load_settings() -> Result<McpSettings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(McpSettings::default());
    }

    let raw = fs::read_to_string(path)?;
    let mut settings: McpSettings = serde_json::from_str(&raw)?;
    if settings.port == 0 {
        settings.port = DEFAULT_PORT;
    }
    Ok(settings)
}

fn save_settings(settings: &McpSettings) -> Result<()> {
    let path = settings_path()?;
    let raw = serde_json::to_string_pretty(settings)?;
    fs::write(path, raw)?;
    Ok(())
}

fn load_audit_events() -> Vec<McpAuditEvent> {
    let Ok(path) = audit_path() else {
        return Vec::new();
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    match serde_json::from_str(&raw) {
        Ok(events) => events,
        Err(error) => {
            log::warn!(
                "MCP audit log is corrupt; preserving it before starting a new log: {}",
                error
            );
            let corrupt_path =
                path.with_extension(format!("json.corrupt-{}", Utc::now().timestamp()));
            let _ = fs::rename(path, corrupt_path);
            Vec::new()
        }
    }
}

fn save_audit_events(events: &[McpAuditEvent]) -> Result<()> {
    let path = audit_path()?;
    let tmp_path = path.with_extension("json.tmp");
    let raw = serde_json::to_string_pretty(events)?;
    fs::write(&tmp_path, raw)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn append_audit_event(event: McpAuditEvent) {
    let _guard = match AUDIT_FILE_LOCK.lock() {
        Ok(guard) => guard,
        Err(_) => {
            log::warn!("MCP audit log lock poisoned; dropping audit event");
            return;
        }
    };
    let mut events = load_audit_events();
    events.insert(0, event);
    events.truncate(100);
    if let Err(error) = save_audit_events(&events) {
        log::warn!("Failed to save MCP audit event: {}", error);
    }
}

fn load_trusted_clients() -> Vec<TrustedClientRecord> {
    let Ok(path) = clients_path() else {
        return Vec::new();
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    match serde_json::from_str(&raw) {
        Ok(clients) => clients,
        Err(error) => {
            log::warn!(
                "MCP client registry is corrupt; preserving it before starting a new registry: {}",
                error
            );
            let corrupt_path =
                path.with_extension(format!("json.corrupt-{}", Utc::now().timestamp()));
            let _ = fs::rename(path, corrupt_path);
            Vec::new()
        }
    }
}

fn save_trusted_clients(clients: &[TrustedClientRecord]) -> Result<()> {
    let path = clients_path()?;
    let tmp_path = path.with_extension("json.tmp");
    let raw = serde_json::to_string_pretty(clients)?;
    fs::write(&tmp_path, raw)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn random_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn token_fingerprint(token_hash: &str) -> String {
    format!("sha256:{}", token_hash.chars().take(12).collect::<String>())
}

fn default_client_scopes() -> Vec<String> {
    vec![
        SCOPE_READ_STATUS.to_string(),
        SCOPE_LIST_MEETINGS.to_string(),
        SCOPE_READ_MEETINGS.to_string(),
        SCOPE_SEARCH_MEETINGS.to_string(),
    ]
}

fn ensure_agent_client(agent: &AgentKind) -> Result<String> {
    let _guard = CLIENTS_FILE_LOCK
        .lock()
        .map_err(|_| anyhow!("MCP client registry lock poisoned"))?;
    let mut clients = load_trusted_clients();
    let client_id = format!("agent-{}", agent_label(agent).to_lowercase());
    let token = random_token();
    let token_hash = hash_token(&token);
    let now = Utc::now();
    let expires_at = now + chrono::Duration::days(DEFAULT_CLIENT_TTL_DAYS);
    let record = TrustedClientRecord {
        id: client_id.clone(),
        name: format!("{} via Meetily MCP", agent_label(agent)),
        scopes: default_client_scopes(),
        token_fingerprint: token_fingerprint(&token_hash),
        token_hash,
        expires_at: expires_at.to_rfc3339(),
        revoked: false,
        created_at: clients
            .iter()
            .find(|client| client.id == client_id)
            .map(|client| client.created_at.clone())
            .unwrap_or_else(|| now.to_rfc3339()),
        updated_at: now.to_rfc3339(),
    };

    if let Some(existing) = clients.iter_mut().find(|client| client.id == client_id) {
        *existing = record;
    } else {
        clients.push(record);
    }
    save_trusted_clients(&clients)?;
    Ok(token)
}

fn public_clients() -> Vec<McpClient> {
    load_trusted_clients()
        .into_iter()
        .map(McpClient::from)
        .collect()
}

fn revoke_trusted_client(client_id: &str) -> Result<Vec<McpClient>> {
    let _guard = CLIENTS_FILE_LOCK
        .lock()
        .map_err(|_| anyhow!("MCP client registry lock poisoned"))?;
    let mut clients = load_trusted_clients();
    let now = now_string();
    for client in &mut clients {
        if client.id == client_id {
            client.revoked = true;
            client.updated_at = now.clone();
        }
    }
    save_trusted_clients(&clients)?;
    Ok(clients.into_iter().map(McpClient::from).collect())
}

#[derive(Debug, Clone)]
struct AuthorizedClient {
    id: String,
}

#[derive(Debug, Clone)]
enum AuthFailure {
    Missing,
    Invalid,
    Expired { client_id: String },
    Revoked { client_id: String },
    InsufficientScope { client_id: String },
}

impl AuthFailure {
    fn result(&self) -> &'static str {
        match self {
            Self::Revoked { .. } => "revoked",
            _ => "denied",
        }
    }

    fn reason(&self) -> &'static str {
        match self {
            Self::Missing => "missing_authorization",
            Self::Invalid => "invalid_authorization",
            Self::Expired { .. } => "expired_client",
            Self::Revoked { .. } => "revoked_client",
            Self::InsufficientScope { .. } => "insufficient_scope",
        }
    }

    fn client_id(&self) -> String {
        match self {
            Self::Expired { client_id }
            | Self::Revoked { client_id }
            | Self::InsufficientScope { client_id } => client_id.clone(),
            Self::Missing => "unauthorized".to_string(),
            Self::Invalid => "unknown".to_string(),
        }
    }
}

fn authorize_client(
    authorization: Option<&str>,
    required_scope: &str,
) -> Result<AuthorizedClient, AuthFailure> {
    let Some(header) = authorization else {
        return Err(AuthFailure::Missing);
    };
    let Some(token) = header.trim().strip_prefix("Bearer ") else {
        return Err(AuthFailure::Invalid);
    };
    let token_hash = hash_token(token.trim());
    let now = Utc::now();

    for client in load_trusted_clients() {
        if client.token_hash != token_hash {
            continue;
        }
        if client.revoked {
            return Err(AuthFailure::Revoked {
                client_id: client.id,
            });
        }
        let expires_at = chrono::DateTime::parse_from_rfc3339(&client.expires_at)
            .map(|timestamp| timestamp.with_timezone(&Utc))
            .map_err(|_| AuthFailure::Invalid)?;
        if expires_at <= now {
            return Err(AuthFailure::Expired {
                client_id: client.id,
            });
        }
        if !client.scopes.iter().any(|scope| scope == required_scope) {
            return Err(AuthFailure::InsufficientScope {
                client_id: client.id,
            });
        }
        return Ok(AuthorizedClient { id: client.id });
    }

    Err(AuthFailure::Invalid)
}

fn now_string() -> String {
    Utc::now().to_rfc3339()
}

fn endpoint_url(port: u16) -> String {
    format!("http://127.0.0.1:{}/mcp", port)
}

fn build_status(settings: McpSettings, runtime: &McpRuntime) -> McpStatus {
    let url = if runtime.state == McpServerState::Running {
        Some(endpoint_url(settings.port))
    } else {
        None
    };

    McpStatus {
        settings: settings.clone(),
        state: runtime.state.clone(),
        bind_host: "127.0.0.1".to_string(),
        port: settings.port,
        url,
        last_error: runtime.last_error.clone(),
    }
}

async fn write_response(
    socket: &mut tokio::net::TcpStream,
    status: &str,
    body: &str,
) -> io::Result<()> {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: http://localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.as_bytes().len(),
        body
    );
    socket.write_all(response.as_bytes()).await
}

fn header_value(request: &str, header_name: &str) -> Option<String> {
    request
        .split("\r\n\r\n")
        .next()
        .unwrap_or_default()
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case(header_name) {
                Some(value.trim().to_string())
            } else {
                None
            }
        })
}

async fn read_http_request(socket: &mut tokio::net::TcpStream) -> io::Result<String> {
    let mut buffer = Vec::with_capacity(8192);
    let mut chunk = [0; 2048];
    let mut expected_len: Option<usize> = None;

    loop {
        let n = timeout(Duration::from_secs(5), socket.read(&mut chunk))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "MCP request timed out"))??;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..n]);

        if expected_len.is_none() {
            if let Some(header_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                let header_raw = String::from_utf8_lossy(&buffer[..header_end]);
                let content_length = header_raw
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                expected_len = Some(header_end + 4 + content_length);
            }
        }

        if let Some(total_len) = expected_len {
            if buffer.len() >= total_len {
                break;
            }
        }

        if buffer.len() > 1024 * 1024 {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn mcp_error(id: Value, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
    .to_string()
}

fn text_tool_result(value: Value) -> Value {
    json!({
        "content": [
            { "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()) }
        ],
        "isError": false
    })
}

fn tool_schema() -> Value {
    json!({
        "tools": [
            {
                "name": "meetily_status",
                "description": "Read the local Meetily MCP server status without exposing meeting content.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "meetily_list_meetings",
                "description": "List authorized Meetily meetings with basic metadata.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1, "maximum": 100 },
                        "query": { "type": "string" }
                    }
                }
            },
            {
                "name": "meetily_search_transcripts",
                "description": "Search Meetily transcripts and return bounded snippets.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": { "type": "string", "minLength": 1 },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                    }
                }
            },
            {
                "name": "meetily_get_meeting",
                "description": "Get one meeting's metadata and content availability.",
                "inputSchema": {
                    "type": "object",
                    "required": ["meetingId"],
                    "properties": { "meetingId": { "type": "string" } }
                }
            },
            {
                "name": "meetily_get_summary",
                "description": "Get the stored summary payload for a meeting.",
                "inputSchema": {
                    "type": "object",
                    "required": ["meetingId"],
                    "properties": { "meetingId": { "type": "string" } }
                }
            },
            {
                "name": "meetily_get_transcript",
                "description": "Get paginated transcript segments for a meeting.",
                "inputSchema": {
                    "type": "object",
                    "required": ["meetingId"],
                    "properties": {
                        "meetingId": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 500 },
                        "offset": { "type": "integer", "minimum": 0 }
                    }
                }
            },
            {
                "name": "meetily_get_action_items",
                "description": "Get action items stored for a meeting.",
                "inputSchema": {
                    "type": "object",
                    "required": ["meetingId"],
                    "properties": { "meetingId": { "type": "string" } }
                }
            },
            {
                "name": "meetily_get_artifacts",
                "description": "Get safe artifact metadata for a meeting.",
                "inputSchema": {
                    "type": "object",
                    "required": ["meetingId"],
                    "properties": { "meetingId": { "type": "string" } }
                }
            }
        ]
    })
}

fn call_arguments(parsed: &Value) -> Value {
    parsed
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn string_arg(args: &Value, name: &str) -> Option<String> {
    args.get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn bounded_i64_arg(args: &Value, name: &str, default: i64, min: i64, max: i64) -> i64 {
    args.get(name)
        .and_then(Value::as_i64)
        .unwrap_or(default)
        .clamp(min, max)
}

fn audit_denial(
    tool_name: &str,
    required_scope: &str,
    failure: &AuthFailure,
    meeting_ids: Vec<String>,
) {
    append_audit_event(McpAuditEvent {
        id: Uuid::new_v4().to_string(),
        timestamp: now_string(),
        client_id: failure.client_id(),
        tool_name: tool_name.to_string(),
        scopes: vec![required_scope.to_string()],
        meeting_ids,
        result: failure.result().to_string(),
        reason: Some(failure.reason().to_string()),
    });
}

fn audit_allowed(
    client: &AuthorizedClient,
    tool_name: &str,
    scope: &str,
    meeting_ids: Vec<String>,
) {
    append_audit_event(McpAuditEvent {
        id: Uuid::new_v4().to_string(),
        timestamp: now_string(),
        client_id: client.id.clone(),
        tool_name: tool_name.to_string(),
        scopes: vec![scope.to_string()],
        meeting_ids,
        result: "allowed".to_string(),
        reason: None,
    });
}

async fn call_meeting_tool(
    app_handle: &AppHandle,
    tool_name: &str,
    args: &Value,
    authorization: Option<&str>,
) -> Result<Value, String> {
    let required_scope = match tool_name {
        "meetily_list_meetings" => SCOPE_LIST_MEETINGS,
        "meetily_search_transcripts" => SCOPE_SEARCH_MEETINGS,
        "meetily_get_meeting"
        | "meetily_get_summary"
        | "meetily_get_transcript"
        | "meetily_get_action_items"
        | "meetily_get_artifacts" => SCOPE_READ_MEETINGS,
        _ => SCOPE_READ_STATUS,
    };

    let meeting_id = string_arg(args, "meetingId");
    let meeting_ids = meeting_id.iter().cloned().collect::<Vec<_>>();
    let client = match authorize_client(authorization, required_scope) {
        Ok(client) => client,
        Err(failure) => {
            audit_denial(tool_name, required_scope, &failure, meeting_ids);
            return Err(format!("MCP authorization failed: {}", failure.reason()));
        }
    };

    let state = app_handle.state::<AppState>();
    let pool = state.db_manager.pool();
    let result: Result<Value, String> = async {
        let value = match tool_name {
        "meetily_list_meetings" => {
            let limit = bounded_i64_arg(args, "limit", 25, 1, 100) as usize;
            let query = string_arg(args, "query").map(|value| value.to_lowercase());
            let meetings = MeetingsRepository::get_meetings(pool)
                .await
                .map_err(|error| format!("Failed to list meetings: {}", error))?
                .into_iter()
                .filter(|meeting| {
                    query
                        .as_ref()
                        .map(|query| meeting.title.to_lowercase().contains(query) || meeting.id.to_lowercase().contains(query))
                        .unwrap_or(true)
                })
                .take(limit)
                .map(|meeting| {
                    json!({
                        "id": meeting.id,
                        "title": meeting.title,
                        "createdAt": meeting.created_at.0.to_rfc3339(),
                        "updatedAt": meeting.updated_at.0.to_rfc3339()
                    })
                })
                .collect::<Vec<_>>();
            json!({ "meetings": meetings })
        }
        "meetily_search_transcripts" => {
            let query = string_arg(args, "query").ok_or_else(|| "query is required".to_string())?;
            let limit = bounded_i64_arg(args, "limit", 10, 1, 50) as usize;
            let results = TranscriptsRepository::search_transcripts(pool, &query)
                .await
                .map_err(|error| format!("Failed to search transcripts: {}", error))?
                .into_iter()
                .take(limit)
                .map(|result| {
                    json!({
                        "meetingId": result.id,
                        "title": result.title,
                        "matchContext": result.match_context,
                        "timestamp": result.timestamp
                    })
                })
                .collect::<Vec<_>>();
            json!({ "query": query, "results": results })
        }
        "meetily_get_meeting" => {
            let meeting_id = meeting_id.ok_or_else(|| "meetingId is required".to_string())?;
            let meeting = MeetingsRepository::get_meeting(pool, &meeting_id)
                .await
                .map_err(|error| format!("Failed to get meeting: {}", error))?
                .ok_or_else(|| "Meeting not found".to_string())?;
            let summary = SummaryProcessesRepository::get_summary_data(pool, &meeting_id)
                .await
                .map_err(|error| format!("Failed to inspect summary: {}", error))?;
            json!({
                "id": meeting.id,
                "title": meeting.title,
                "createdAt": meeting.created_at,
                "updatedAt": meeting.updated_at,
                "transcriptCount": meeting.transcripts.len(),
                "hasSummary": summary.as_ref().and_then(|summary| summary.result.as_ref()).is_some(),
                "summaryStatus": summary.map(|summary| summary.status)
            })
        }
        "meetily_get_summary" => {
            let meeting_id = meeting_id.ok_or_else(|| "meetingId is required".to_string())?;
            let summary = SummaryProcessesRepository::get_summary_data(pool, &meeting_id)
                .await
                .map_err(|error| format!("Failed to get summary: {}", error))?;
            json!({
                "meetingId": meeting_id,
                "summary": summary.and_then(|summary| summary.result)
            })
        }
        "meetily_get_transcript" => {
            let meeting_id = meeting_id.ok_or_else(|| "meetingId is required".to_string())?;
            let limit = bounded_i64_arg(args, "limit", 100, 1, 500);
            let offset = bounded_i64_arg(args, "offset", 0, 0, i64::MAX);
            let (transcripts, total_count) = MeetingsRepository::get_meeting_transcripts_paginated(pool, &meeting_id, limit, offset)
                .await
                .map_err(|error| format!("Failed to get transcript: {}", error))?;
            let segments = transcripts
                .into_iter()
                .map(|segment| {
                    json!({
                        "id": segment.id,
                        "text": segment.transcript,
                        "timestamp": segment.timestamp,
                        "audioStartTime": segment.audio_start_time,
                        "audioEndTime": segment.audio_end_time,
                        "duration": segment.duration
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "meetingId": meeting_id,
                "totalCount": total_count,
                "limit": limit,
                "offset": offset,
                "segments": segments
            })
        }
        "meetily_get_action_items" => {
            let meeting_id = meeting_id.ok_or_else(|| "meetingId is required".to_string())?;
            let rows = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
                "SELECT id, action_items, key_points FROM transcripts WHERE meeting_id = ? AND (action_items IS NOT NULL OR key_points IS NOT NULL)"
            )
            .bind(&meeting_id)
            .fetch_all(pool)
            .await
            .map_err(|error| format!("Failed to get action items: {}", error))?;
            let transcript_items = rows
                .into_iter()
                .map(|(id, action_items, key_points)| {
                    json!({
                        "transcriptId": id,
                        "actionItems": action_items,
                        "keyPoints": key_points
                    })
                })
                .collect::<Vec<_>>();
            let summary = SummaryProcessesRepository::get_summary_data(pool, &meeting_id)
                .await
                .map_err(|error| format!("Failed to inspect summary: {}", error))?;
            json!({
                "meetingId": meeting_id,
                "transcriptItems": transcript_items,
                "summary": summary.and_then(|summary| summary.result)
            })
        }
        "meetily_get_artifacts" => {
            let meeting_id = meeting_id.ok_or_else(|| "meetingId is required".to_string())?;
            let meeting = MeetingsRepository::get_meeting_metadata(pool, &meeting_id)
                .await
                .map_err(|error| format!("Failed to get artifacts: {}", error))?
                .ok_or_else(|| "Meeting not found".to_string())?;
            let summary = SummaryProcessesRepository::get_summary_data(pool, &meeting_id)
                .await
                .map_err(|error| format!("Failed to inspect summary: {}", error))?;
            let (_, total_transcripts) = MeetingsRepository::get_meeting_transcripts_paginated(pool, &meeting_id, 1, 0)
                .await
                .map_err(|error| format!("Failed to inspect transcript artifacts: {}", error))?;
            json!({
                "meetingId": meeting.id,
                "artifacts": [
                    { "type": "meetingMetadata", "available": true },
                    { "type": "transcript", "available": total_transcripts > 0, "segmentCount": total_transcripts },
                    { "type": "summary", "available": summary.and_then(|summary| summary.result).is_some() }
                ]
            })
        }
            _ => return Err("Unknown tool".to_string()),
        };
        Ok(value)
    }
    .await;

    match result {
        Ok(value) => {
            audit_allowed(&client, tool_name, required_scope, meeting_ids);
            Ok(value)
        }
        Err(error) => {
            append_audit_event(McpAuditEvent {
                id: Uuid::new_v4().to_string(),
                timestamp: now_string(),
                client_id: client.id,
                tool_name: tool_name.to_string(),
                scopes: vec![required_scope.to_string()],
                meeting_ids,
                result: "failed".to_string(),
                reason: Some(error.clone()),
            });
            Err(error)
        }
    }
}

async fn json_rpc_response(
    request: &str,
    authorization: Option<&str>,
    app_handle: Option<&AppHandle>,
) -> String {
    let parsed: Value = serde_json::from_str(request).unwrap_or_else(|_| json!({}));
    let id = parsed.get("id").cloned().unwrap_or(Value::Null);
    let method = parsed.get("method").and_then(Value::as_str).unwrap_or("");

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-06-18",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "meetily", "version": env!("CARGO_PKG_VERSION") }
        }),
        "tools/list" => tool_schema(),
        "tools/call" => {
            let tool_name = parsed
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if tool_name == "meetily_status" {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": text_tool_result(json!({
                        "name": "meetily",
                        "version": env!("CARGO_PKG_VERSION"),
                        "transport": "streamable-http",
                        "policy": "local-only read-only meeting tools require bearer authorization"
                    }))
                })
                .to_string();
            }
            let Some(app_handle) = app_handle else {
                return mcp_error(id, -32000, "Meeting tools are unavailable in this context");
            };
            let args = call_arguments(&parsed);
            match call_meeting_tool(app_handle, tool_name, &args, authorization).await {
                Ok(value) => text_tool_result(value),
                Err(error) => {
                    return mcp_error(id, -32001, &error);
                }
            }
        }
        _ => {
            return mcp_error(id, -32601, "Method not found");
        }
    };

    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

async fn handle_connection(
    mut socket: tokio::net::TcpStream,
    app_handle: AppHandle,
) -> io::Result<()> {
    let request = read_http_request(&mut socket).await?;
    if request.is_empty() {
        return Ok(());
    }

    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or_default();
    let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();

    if request_line.starts_with("GET /health ") {
        let body = json!({
            "ok": true,
            "name": "meetily",
            "transport": "streamable-http"
        })
        .to_string();
        return write_response(&mut socket, "200 OK", &body).await;
    }

    if request_line.starts_with("POST /mcp ") {
        let authorization = header_value(&request, "authorization");
        let body = json_rpc_response(body, authorization.as_deref(), Some(&app_handle)).await;
        return write_response(&mut socket, "200 OK", &body).await;
    }

    write_response(
        &mut socket,
        "404 Not Found",
        &json!({ "error": "not_found" }).to_string(),
    )
    .await
}

async fn run_server(
    listener: TcpListener,
    mut shutdown_rx: oneshot::Receiver<()>,
    app_handle: AppHandle,
) -> Result<()> {
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((socket, _)) => {
                        let app_handle = app_handle.clone();
                        tokio::spawn(async move {
                            if let Err(error) = handle_connection(socket, app_handle).await {
                                log::warn!("MCP connection failed: {}", error);
                            }
                        });
                    }
                    Err(error) => log::warn!("MCP accept failed: {}", error),
                }
            }
            _ = &mut shutdown_rx => {
                break;
            }
        }
    }

    Ok(())
}

async fn start_runtime(runtime: &mut McpRuntime, app_handle: AppHandle) -> Result<()> {
    if runtime.state == McpServerState::Running || runtime.state == McpServerState::Starting {
        return Ok(());
    }

    let port = runtime.settings.port;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    runtime.state = McpServerState::Starting;
    runtime.last_error = None;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("Unable to bind MCP server to 127.0.0.1:{}", port))?;

    let task = tokio::spawn(async move {
        if let Err(error) = run_server(listener, shutdown_rx, app_handle).await {
            log::error!("MCP server stopped with error: {}", error);
        }
    });

    runtime.shutdown_tx = Some(shutdown_tx);
    runtime.task = Some(task);
    runtime.state = McpServerState::Running;
    Ok(())
}

async fn stop_runtime(runtime: &mut McpRuntime) {
    if let Some(tx) = runtime.shutdown_tx.take() {
        let _ = tx.send(());
    }
    if let Some(task) = runtime.task.take() {
        task.abort();
    }
    runtime.state = McpServerState::Stopped;
}

pub async fn initialize_on_startup(app_handle: AppHandle) {
    let state = app_handle.state::<McpState>();
    let mut runtime = state.runtime.lock().await;
    runtime.settings = load_settings().unwrap_or_default();

    if runtime.settings.enabled && runtime.settings.auto_start {
        if let Err(error) = start_runtime(&mut runtime, app_handle.clone()).await {
            runtime.state = McpServerState::Error;
            runtime.last_error = Some(error.to_string());
        }
    }
}

pub async fn shutdown(state: &McpState) {
    let mut runtime = state.runtime.lock().await;
    stop_runtime(&mut runtime).await;
}

#[tauri::command]
pub async fn mcp_get_status(state: State<'_, McpState>) -> Result<McpStatus, String> {
    let mut runtime = state.runtime.lock().await;
    if runtime.state == McpServerState::Running
        && runtime.task.as_ref().is_some_and(|task| task.is_finished())
    {
        runtime.task = None;
        runtime.shutdown_tx = None;
        runtime.state = McpServerState::Error;
        runtime.last_error = Some("MCP server stopped unexpectedly.".to_string());
    }
    if runtime.state == McpServerState::Stopped || runtime.state == McpServerState::Error {
        runtime.settings = load_settings().unwrap_or_else(|_| runtime.settings.clone());
    }
    Ok(build_status(runtime.settings.clone(), &runtime))
}

#[tauri::command]
pub async fn mcp_update_settings(
    settings: McpSettings,
    app_handle: AppHandle,
    state: State<'_, McpState>,
) -> Result<McpStatus, String> {
    if settings.port == 0 {
        return Err("Port must be greater than zero".to_string());
    }

    save_settings(&settings).map_err(|_| "Unable to save MCP settings".to_string())?;

    let mut runtime = state.runtime.lock().await;
    let was_enabled = runtime.settings.enabled;
    let old_port = runtime.settings.port;
    runtime.settings = settings.clone();

    if settings.enabled {
        if was_enabled && old_port != settings.port {
            stop_runtime(&mut runtime).await;
        }
        if let Err(error) = start_runtime(&mut runtime, app_handle).await {
            runtime.state = McpServerState::Error;
            runtime.last_error = Some(error.to_string());
        }
    } else {
        stop_runtime(&mut runtime).await;
    }

    Ok(build_status(settings, &runtime))
}

#[tauri::command]
pub async fn mcp_start_server(
    app_handle: AppHandle,
    state: State<'_, McpState>,
) -> Result<McpStatus, String> {
    let mut settings = load_settings().unwrap_or_default();
    settings.enabled = true;
    save_settings(&settings).map_err(|_| "Unable to save MCP settings".to_string())?;

    let mut runtime = state.runtime.lock().await;
    runtime.settings = settings.clone();
    if let Err(error) = start_runtime(&mut runtime, app_handle).await {
        runtime.state = McpServerState::Error;
        runtime.last_error = Some(error.to_string());
    }

    Ok(build_status(settings, &runtime))
}

#[tauri::command]
pub async fn mcp_stop_server(state: State<'_, McpState>) -> Result<McpStatus, String> {
    let mut settings = load_settings().unwrap_or_default();
    settings.enabled = false;
    save_settings(&settings).map_err(|_| "Unable to save MCP settings".to_string())?;

    let mut runtime = state.runtime.lock().await;
    runtime.settings = settings.clone();
    stop_runtime(&mut runtime).await;

    Ok(build_status(settings, &runtime))
}

#[tauri::command]
pub async fn mcp_list_clients() -> Result<Vec<McpClient>, String> {
    Ok(public_clients())
}

#[tauri::command]
pub async fn mcp_revoke_client(client_id: String) -> Result<Vec<McpClient>, String> {
    revoke_trusted_client(&client_id).map_err(|_| "Unable to revoke MCP client".to_string())
}

#[tauri::command]
pub async fn mcp_list_audit_events() -> Result<Vec<McpAuditEvent>, String> {
    Ok(load_audit_events())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentKind {
    Claude,
    Codex,
    Cursor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSetupStatus {
    pub agent: AgentKind,
    pub label: String,
    pub config_path: String,
    pub installed: bool,
    pub configured: bool,
    pub working: bool,
    pub status: String,
    pub last_checked_at: String,
    pub message: String,
}

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))
}

fn agent_config_path(agent: &AgentKind) -> Result<PathBuf> {
    let home = home_dir()?;
    Ok(match agent {
        AgentKind::Claude if cfg!(target_os = "macos") => {
            home.join("Library/Application Support/Claude/claude_desktop_config.json")
        }
        AgentKind::Claude if cfg!(target_os = "windows") => dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not find config directory"))?
            .join("Claude/claude_desktop_config.json"),
        AgentKind::Claude => dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not find config directory"))?
            .join("Claude/claude_desktop_config.json"),
        AgentKind::Cursor => home.join(".cursor/mcp.json"),
        AgentKind::Codex => home.join(".codex/config.toml"),
    })
}

fn agent_label(agent: &AgentKind) -> &'static str {
    match agent {
        AgentKind::Claude => "Claude",
        AgentKind::Codex => "Codex",
        AgentKind::Cursor => "Cursor",
    }
}

fn expected_url(settings: &McpSettings) -> String {
    endpoint_url(settings.port)
}

fn config_contains_meetily(path: &PathBuf, agent: &AgentKind, url: &str) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };

    match agent {
        AgentKind::Codex => {
            raw.contains("[mcp_servers.meetily]") && raw.contains("mcp-remote") && raw.contains(url)
        }
        AgentKind::Claude | AgentKind::Cursor => raw.contains("\"meetily\"") && raw.contains(url),
    }
}

fn status_for_agent(
    agent: AgentKind,
    settings: &McpSettings,
    server_running: bool,
) -> AgentSetupStatus {
    let path = agent_config_path(&agent).unwrap_or_default();
    let installed = path.parent().map(|parent| parent.exists()).unwrap_or(false);
    let configured = config_contains_meetily(&path, &agent, &expected_url(settings));
    let working = configured && server_running;
    let status = if working {
        "working"
    } else if configured {
        "configured"
    } else if installed {
        "notConfigured"
    } else {
        "notInstalled"
    };

    AgentSetupStatus {
        agent: agent.clone(),
        label: agent_label(&agent).to_string(),
        config_path: path.display().to_string(),
        installed,
        configured,
        working,
        status: status.to_string(),
        last_checked_at: now_string(),
        message: match status {
            "working" => "Configured and MCP server is running.".to_string(),
            "configured" => "Configured. Start MCP to complete the working check.".to_string(),
            "notConfigured" => "App config found, but Meetily MCP is not configured.".to_string(),
            _ => "Config folder was not found on this machine.".to_string(),
        },
    }
}

fn mcp_remote_args(url: &str, token: &str) -> Vec<String> {
    vec![
        "-y".to_string(),
        "mcp-remote".to_string(),
        url.to_string(),
        "--header".to_string(),
        format!("Authorization: Bearer {}", token),
    ]
}

fn merge_json_mcp_config(path: &PathBuf, agent: &AgentKind, url: &str, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let backup = path.with_extension(format!(
            "json.meetily-backup-{}",
            Utc::now().timestamp_millis()
        ));
        fs::copy(path, backup)?;
    }

    let mut root = if path.exists() {
        let raw = fs::read_to_string(path)?;
        serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !root.is_object() {
        root = json!({});
    }

    let map = root.as_object_mut().expect("object checked above");
    let servers = map.entry("mcpServers").or_insert_with(|| json!({}));
    if !servers.is_object() {
        *servers = json!({});
    }

    let server_config = match agent {
        AgentKind::Cursor => json!({
            "url": url,
            "transport": "streamable-http",
            "headers": { "Authorization": format!("Bearer {}", token) }
        }),
        AgentKind::Claude => json!({
            "command": "npx",
            "args": mcp_remote_args(url, token)
        }),
        AgentKind::Codex => unreachable!(),
    };

    servers
        .as_object_mut()
        .expect("object checked above")
        .insert("meetily".to_string(), server_config);

    fs::write(path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

fn merge_codex_config(path: &PathBuf, url: &str, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let backup = path.with_extension(format!(
            "toml.meetily-backup-{}",
            Utc::now().timestamp_millis()
        ));
        fs::copy(path, backup)?;
    }

    let mut raw = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if raw.contains("[mcp_servers.meetily]") {
        let mut filtered = Vec::new();
        let mut skipping = false;
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed == "[mcp_servers.meetily]" || trimmed.starts_with("[mcp_servers.meetily.") {
                skipping = true;
                continue;
            }
            if skipping && trimmed.starts_with('[') {
                skipping = false;
            }
            if !skipping {
                filtered.push(line);
            }
        }
        raw = filtered.join("\n");
    }

    if !raw.ends_with('\n') && !raw.is_empty() {
        raw.push('\n');
    }

    let safe_url = url.replace('"', "");
    let safe_header = format!("Authorization: Bearer {}", token).replace('"', "");
    raw.push_str(&format!(
        "\n[mcp_servers.meetily]\ncommand = \"npx\"\nargs = [\"-y\", \"mcp-remote\", \"{}\", \"--header\", \"{}\"]\nenabled = true\n",
        safe_url, safe_header
    ));
    fs::write(path, raw)?;
    Ok(())
}

#[tauri::command]
pub async fn mcp_get_agent_statuses(
    state: State<'_, McpState>,
) -> Result<Vec<AgentSetupStatus>, String> {
    let runtime = state.runtime.lock().await;
    let server_running = runtime.state == McpServerState::Running;
    let settings = runtime.settings.clone();
    Ok(vec![
        status_for_agent(AgentKind::Claude, &settings, server_running),
        status_for_agent(AgentKind::Codex, &settings, server_running),
        status_for_agent(AgentKind::Cursor, &settings, server_running),
    ])
}

#[tauri::command]
pub async fn mcp_setup_agent(
    agent: AgentKind,
    state: State<'_, McpState>,
) -> Result<AgentSetupStatus, String> {
    let runtime = state.runtime.lock().await;
    let settings = runtime.settings.clone();
    let running = runtime.state == McpServerState::Running;
    drop(runtime);

    let path = agent_config_path(&agent)
        .map_err(|_| "Unable to find the user home directory".to_string())?;
    let url = expected_url(&settings);
    let token = ensure_agent_client(&agent)
        .map_err(|_| format!("Unable to authorize {} MCP client", agent_label(&agent)))?;

    let result = match agent {
        AgentKind::Claude | AgentKind::Cursor => merge_json_mcp_config(&path, &agent, &url, &token),
        AgentKind::Codex => merge_codex_config(&path, &url, &token),
    };

    result.map_err(|_| format!("Unable to update {} MCP configuration", agent_label(&agent)))?;
    Ok(status_for_agent(agent, &settings, running))
}

fn setup_agent_inner(
    agent: AgentKind,
    settings: &McpSettings,
    running: bool,
) -> Result<AgentSetupStatus> {
    let path = agent_config_path(&agent)?;
    let url = expected_url(settings);
    let token = ensure_agent_client(&agent)?;

    match agent {
        AgentKind::Claude | AgentKind::Cursor => {
            merge_json_mcp_config(&path, &agent, &url, &token)?
        }
        AgentKind::Codex => merge_codex_config(&path, &url, &token)?,
    };

    Ok(status_for_agent(agent, settings, running))
}

#[tauri::command]
pub async fn mcp_setup_all_agents(
    state: State<'_, McpState>,
) -> Result<Vec<AgentSetupStatus>, String> {
    let runtime = state.runtime.lock().await;
    let settings = runtime.settings.clone();
    let running = runtime.state == McpServerState::Running;
    drop(runtime);

    [AgentKind::Claude, AgentKind::Codex, AgentKind::Cursor]
        .into_iter()
        .map(|agent| {
            setup_agent_inner(agent.clone(), &settings, running)
                .map_err(|_| format!("Unable to update {} MCP configuration", agent_label(&agent)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_to_disabled() {
        let settings = McpSettings::default();
        assert!(!settings.enabled);
        assert!(!settings.auto_start);
        assert_eq!(settings.port, DEFAULT_PORT);
    }

    #[test]
    fn json_rpc_initialize_returns_server_info() {
        let response = tauri::async_runtime::block_on(json_rpc_response(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
            None,
            None,
        ));
        assert!(response.contains("\"name\":\"meetily\""));
        assert!(response.contains("\"protocolVersion\""));
    }

    #[test]
    fn token_hash_fingerprint_does_not_expose_raw_token() {
        let token = "local-secret-token";
        let hash = hash_token(token);
        let fingerprint = token_fingerprint(&hash);

        assert_ne!(hash, token);
        assert!(fingerprint.starts_with("sha256:"));
        assert!(!fingerprint.contains(token));
    }

    #[test]
    fn codex_config_detection_requires_meetily_server() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        fs::write(
            temp.path(),
            "[mcp_servers.meetily]\ncommand = \"npx\"\nargs = [\"-y\", \"mcp-remote\", \"http://127.0.0.1:43118/mcp\"]\n",
        )
        .unwrap();
        assert!(config_contains_meetily(
            &temp.path().to_path_buf(),
            &AgentKind::Codex,
            "http://127.0.0.1:43118/mcp"
        ));
    }

    #[test]
    fn json_config_merge_preserves_existing_servers() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        fs::write(
            temp.path(),
            r#"{"mcpServers":{"existing":{"url":"http://example.test"}}}"#,
        )
        .unwrap();
        merge_json_mcp_config(
            &temp.path().to_path_buf(),
            &AgentKind::Cursor,
            "http://127.0.0.1:43118/mcp",
            "token",
        )
        .unwrap();
        let raw = fs::read_to_string(temp.path()).unwrap();
        assert!(raw.contains("\"existing\""));
        assert!(raw.contains("\"meetily\""));
        assert!(raw.contains("\"Authorization\""));
    }

    #[test]
    fn codex_config_merge_writes_authorization_header() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        merge_codex_config(
            &temp.path().to_path_buf(),
            "http://127.0.0.1:43118/mcp",
            "token",
        )
        .unwrap();
        let raw = fs::read_to_string(temp.path()).unwrap();
        assert!(raw.contains("[mcp_servers.meetily]"));
        assert!(raw.contains("Authorization: Bearer token"));
    }
}
