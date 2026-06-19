use anyhow::{anyhow, Context, Result};
use chrono::{Duration as ChronoDuration, NaiveDate, NaiveTime, TimeZone, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    sync::Mutex as StdMutex,
    time::Duration,
};
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
const LOCAL_LOOPBACK_CLIENT_ID: &str = "local-loopback";

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
    format!("sha256:{}", token_hash.chars().take(16).collect::<String>())
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

fn active_agent_client_exists(agent: &AgentKind) -> bool {
    let client_id = format!("agent-{}", agent_label(agent).to_lowercase());
    let now = Utc::now();
    load_trusted_clients().into_iter().any(|client| {
        if client.id != client_id || client.revoked {
            return false;
        }

        chrono::DateTime::parse_from_rfc3339(&client.expires_at)
            .map(|expires_at| expires_at.with_timezone(&Utc) > now)
            .unwrap_or(false)
    })
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

fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right.iter())
        .fold(0u8, |diff, (left, right)| diff | (left ^ right))
        == 0
}

#[derive(Debug, Clone)]
struct AuthorizedClient {
    id: String,
}

#[derive(Debug, Clone)]
enum AuthFailure {
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
            Self::Invalid => "unknown".to_string(),
        }
    }
}

fn authorize_client(
    authorization: Option<&str>,
    required_scope: &str,
) -> Result<AuthorizedClient, AuthFailure> {
    let Some(header) = authorization else {
        return Ok(AuthorizedClient {
            id: LOCAL_LOOPBACK_CLIENT_ID.to_string(),
        });
    };
    let Some(token) = header.trim().strip_prefix("Bearer ") else {
        return Err(AuthFailure::Invalid);
    };
    let token_hash = hash_token(token.trim());
    let now = Utc::now();

    for client in load_trusted_clients() {
        if !constant_time_eq(&client.token_hash, &token_hash) {
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
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            },
            {
                "name": "meetily_get_latest_meeting",
                "description": "Get the most recent meeting, optionally including summary, transcript excerpts, and action items.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "includeTranscript": { "type": "boolean" },
                        "transcriptLimit": { "type": "integer", "minimum": 1, "maximum": 50 }
                    }
                }
            },
            {
                "name": "meetily_find_meetings",
                "description": "Find meetings by topic, person, title text, or date range.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "person": { "type": "string" },
                        "dateFrom": { "type": "string", "description": "Inclusive YYYY-MM-DD date." },
                        "dateTo": { "type": "string", "description": "Inclusive YYYY-MM-DD date." },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                    }
                }
            },
            {
                "name": "meetily_ask_meetings",
                "description": "Gather answer-ready meeting context for natural questions such as what was said on the last call with someone.",
                "inputSchema": {
                    "type": "object",
                    "required": ["question"],
                    "properties": {
                        "question": { "type": "string", "minLength": 1 },
                        "person": { "type": "string" },
                        "topic": { "type": "string" },
                        "latestOnly": { "type": "boolean" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 10 }
                    }
                }
            },
            {
                "name": "meetily_get_recent_action_items",
                "description": "List recent action items and follow-ups across meetings.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "person": { "type": "string" },
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                    }
                }
            },
            {
                "name": "meetily_get_decisions",
                "description": "Find decision-like excerpts and summary sections for a topic across meetings.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                    }
                }
            },
            {
                "name": "meetily_get_followups_for_person",
                "description": "Find action items, promises, and follow-up context involving a person.",
                "inputSchema": {
                    "type": "object",
                    "required": ["person"],
                    "properties": {
                        "person": { "type": "string", "minLength": 1 },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                    }
                }
            },
            {
                "name": "meetily_get_meeting_brief",
                "description": "Create an agent-ready brief for one meeting or the latest meeting.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "meetingId": { "type": "string" },
                        "latest": { "type": "boolean" },
                        "transcriptLimit": { "type": "integer", "minimum": 1, "maximum": 100 }
                    }
                }
            },
            {
                "name": "meetily_compare_meetings",
                "description": "Compare two meetings or the latest meetings matching a topic.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "meetingIds": { "type": "array", "items": { "type": "string" }, "minItems": 2, "maxItems": 5 },
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 2, "maximum": 5 }
                    }
                }
            },
            {
                "name": "meetily_get_project_context",
                "description": "Build a topic timeline across meetings with summaries, decisions, action items, and excerpts.",
                "inputSchema": {
                    "type": "object",
                    "required": ["topic"],
                    "properties": {
                        "topic": { "type": "string", "minLength": 1 },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 20 }
                    }
                }
            },
            {
                "name": "meetily_get_daily_digest",
                "description": "Build a personal daily digest of meetings, decisions, follow-ups, and open questions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "description": "YYYY-MM-DD. Defaults to today in UTC." },
                        "person": { "type": "string" },
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 25 }
                    }
                }
            },
            {
                "name": "meetily_get_weekly_digest",
                "description": "Build a weekly digest across meetings with commitments, decisions, risks, and repeated themes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dateFrom": { "type": "string", "description": "Inclusive YYYY-MM-DD or RFC3339 date." },
                        "dateTo": { "type": "string", "description": "Inclusive YYYY-MM-DD or RFC3339 date." },
                        "person": { "type": "string" },
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                    }
                }
            },
            {
                "name": "meetily_get_open_loops",
                "description": "Find unresolved-looking questions, follow-ups, commitments, and risks across recent meetings.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "person": { "type": "string" },
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 50 }
                    }
                }
            },
            {
                "name": "meetily_prepare_next_meeting",
                "description": "Prepare for a next meeting using prior meetings with a person or topic.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "person": { "type": "string" },
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 10 }
                    }
                }
            },
            {
                "name": "meetily_prepare_role_brief",
                "description": "Prepare a role-specific brief for product, engineering, sales, hiring, manager, founder, or customer-success workflows.",
                "inputSchema": {
                    "type": "object",
                    "required": ["role"],
                    "properties": {
                        "role": { "type": "string", "enum": ["product", "engineering", "sales", "hiring", "manager", "founder", "customer_success"] },
                        "topic": { "type": "string" },
                        "person": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 10 }
                    }
                }
            },
            {
                "name": "meetily_prepare_handoff",
                "description": "Prepare a Codex/Claude/Cursor/Linear handoff from a meeting or topic.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "meetingId": { "type": "string" },
                        "topic": { "type": "string" },
                        "agent": { "type": "string", "enum": ["codex", "claude", "cursor", "linear", "manual"] },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 10 }
                    }
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

async fn ensure_meeting_exists(pool: &sqlx::SqlitePool, meeting_id: &str) -> Result<(), String> {
    MeetingsRepository::get_meeting_metadata(pool, meeting_id)
        .await
        .map_err(|error| format!("Failed to inspect meeting: {}", error))?
        .map(|_| ())
        .ok_or_else(|| "Meeting not found".to_string())
}

fn bool_arg(args: &Value, name: &str, default: bool) -> bool {
    args.get(name).and_then(Value::as_bool).unwrap_or(default)
}

fn string_array_arg(args: &Value, name: &str) -> Vec<String> {
    args.get(name)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn summary_json(summary: Option<String>) -> Value {
    summary
        .and_then(|raw| {
            serde_json::from_str::<Value>(&raw)
                .ok()
                .or_else(|| Some(json!(raw)))
        })
        .unwrap_or(Value::Null)
}

async fn get_summary_json(pool: &sqlx::SqlitePool, meeting_id: &str) -> Result<Value, String> {
    SummaryProcessesRepository::get_summary_data(pool, meeting_id)
        .await
        .map_err(|error| format!("Failed to get summary: {}", error))
        .map(|summary| summary_json(summary.and_then(|summary| summary.result)))
}

async fn get_transcript_excerpt_json(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    limit: i64,
) -> Result<Vec<Value>, String> {
    let (segments, _) =
        MeetingsRepository::get_meeting_transcripts_paginated(pool, meeting_id, limit, 0)
            .await
            .map_err(|error| format!("Failed to get transcript excerpts: {}", error))?;
    Ok(segments
        .into_iter()
        .map(|segment| {
            json!({
                "transcriptId": segment.id,
                "text": segment.transcript,
                "timestamp": segment.timestamp,
                "audioStartTime": segment.audio_start_time,
                "audioEndTime": segment.audio_end_time
            })
        })
        .collect())
}

async fn get_action_context_json(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Vec<Value>, String> {
    let rows = sqlx::query_as::<_, (String, Option<String>, Option<String>, String)>(
        "SELECT id, action_items, key_points, transcript FROM transcripts
         WHERE meeting_id = ? AND (action_items IS NOT NULL OR key_points IS NOT NULL)
         ORDER BY audio_start_time ASC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("Failed to get action context: {}", error))?;

    Ok(rows
        .into_iter()
        .map(|(id, action_items, key_points, transcript)| {
            json!({
                "transcriptId": id,
                "actionItems": action_items,
                "keyPoints": key_points,
                "sourceText": transcript
            })
        })
        .collect())
}

async fn meeting_card(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    include_transcript: bool,
    transcript_limit: i64,
) -> Result<Value, String> {
    let meeting = MeetingsRepository::get_meeting_metadata(pool, meeting_id)
        .await
        .map_err(|error| format!("Failed to get meeting: {}", error))?
        .ok_or_else(|| "Meeting not found".to_string())?;
    let summary = get_summary_json(pool, meeting_id).await?;
    let actions = get_action_context_json(pool, meeting_id).await?;
    let transcript = if include_transcript {
        get_transcript_excerpt_json(pool, meeting_id, transcript_limit).await?
    } else {
        Vec::new()
    };

    Ok(json!({
        "id": meeting.id,
        "title": meeting.title,
        "createdAt": meeting.created_at.0.to_rfc3339(),
        "updatedAt": meeting.updated_at.0.to_rfc3339(),
        "summary": summary,
        "actionContext": actions,
        "transcriptExcerpts": transcript
    }))
}

async fn latest_meeting_id(pool: &sqlx::SqlitePool) -> Result<String, String> {
    MeetingsRepository::get_meetings(pool)
        .await
        .map_err(|error| format!("Failed to list meetings: {}", error))?
        .into_iter()
        .next()
        .map(|meeting| meeting.id)
        .ok_or_else(|| "No meetings found".to_string())
}

async fn transcript_matches_meeting(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    query: &str,
) -> Result<bool, String> {
    if query.trim().is_empty() {
        return Ok(true);
    }
    let pattern = format!("%{}%", query.to_lowercase());
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ? AND LOWER(transcript) LIKE ?",
    )
    .bind(meeting_id)
    .bind(pattern)
    .fetch_one(pool)
    .await
    .map_err(|error| format!("Failed to search meeting transcripts: {}", error))?;
    Ok(count.0 > 0)
}

async fn find_meeting_ids(
    pool: &sqlx::SqlitePool,
    query: Option<&str>,
    person: Option<&str>,
    date_from: Option<&str>,
    date_to: Option<&str>,
    limit: usize,
) -> Result<Vec<String>, String> {
    let from_bound = parse_date_bound(date_from, false)?;
    let to_bound = parse_date_bound(date_to, true)?;
    let meetings = MeetingsRepository::get_meetings(pool)
        .await
        .map_err(|error| format!("Failed to list meetings: {}", error))?;
    let mut matches = Vec::new();
    for meeting in meetings {
        if from_bound
            .as_ref()
            .map(|from| meeting.created_at.0 < *from)
            .unwrap_or(false)
        {
            continue;
        }
        if to_bound
            .as_ref()
            .map(|to| meeting.created_at.0 > *to)
            .unwrap_or(false)
        {
            continue;
        }
        let mut matched = true;
        if let Some(query) = query {
            matched = contains_ci(&meeting.title, query)
                || transcript_matches_meeting(pool, &meeting.id, query).await?;
        }
        if matched {
            if let Some(person) = person {
                matched = contains_ci(&meeting.title, person)
                    || transcript_matches_meeting(pool, &meeting.id, person).await?;
            }
        }
        if matched {
            matches.push(meeting.id);
        }
        if matches.len() >= limit {
            break;
        }
    }
    Ok(matches)
}

fn parse_date_bound(
    value: Option<&str>,
    end_of_day: bool,
) -> Result<Option<chrono::DateTime<Utc>>, String> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(value) {
        return Ok(Some(datetime.with_timezone(&Utc)));
    }

    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("Invalid date '{}'. Use YYYY-MM-DD or RFC3339.", value))?;
    let time = if end_of_day {
        NaiveTime::from_hms_opt(23, 59, 59)
    } else {
        NaiveTime::from_hms_opt(0, 0, 0)
    }
    .ok_or_else(|| "Failed to build date boundary".to_string())?;

    Ok(Some(Utc.from_utc_datetime(&date.and_time(time))))
}

async fn matching_transcript_excerpts(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    query: &str,
    limit: i64,
) -> Result<Vec<Value>, String> {
    let pattern = format!("%{}%", query.to_lowercase());
    let rows = sqlx::query_as::<_, (String, String, String, Option<f64>, Option<f64>)>(
        "SELECT id, transcript, timestamp, audio_start_time, audio_end_time FROM transcripts
         WHERE meeting_id = ? AND LOWER(transcript) LIKE ?
         ORDER BY audio_start_time ASC LIMIT ?",
    )
    .bind(meeting_id)
    .bind(pattern)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| format!("Failed to get matching excerpts: {}", error))?;

    Ok(rows
        .into_iter()
        .map(
            |(id, transcript, timestamp, audio_start_time, audio_end_time)| {
                json!({
                    "transcriptId": id,
                    "text": transcript,
                    "timestamp": timestamp,
                    "audioStartTime": audio_start_time,
                    "audioEndTime": audio_end_time
                })
            },
        )
        .collect())
}

fn day_bounds(date: Option<&str>) -> Result<(String, String), String> {
    let day = match date.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| format!("Invalid date '{}'. Use YYYY-MM-DD.", value))?,
        None => Utc::now().date_naive(),
    };
    Ok((day.to_string(), day.to_string()))
}

fn default_week_bounds(
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<(String, String), String> {
    let to_day = match date_to.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| format!("Invalid dateTo '{}'. Use YYYY-MM-DD.", value))?,
        None => Utc::now().date_naive(),
    };
    let from_day = match date_from.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| format!("Invalid dateFrom '{}'. Use YYYY-MM-DD.", value))?,
        None => to_day - ChronoDuration::days(6),
    };
    Ok((from_day.to_string(), to_day.to_string()))
}

async fn digest_meeting_cards(
    pool: &sqlx::SqlitePool,
    ids: Vec<String>,
    transcript_limit: i64,
) -> Result<Vec<Value>, String> {
    let mut meetings = Vec::new();
    for id in ids {
        meetings.push(meeting_card(pool, &id, transcript_limit > 0, transcript_limit).await?);
    }
    Ok(meetings)
}

fn role_brief_guidance(role: &str) -> &'static str {
    match role {
        "product" => "Extract user problems, feature requests, product decisions, tradeoffs, owners, and validation questions.",
        "engineering" => "Extract implementation tasks, blockers, risks, dependencies, architecture decisions, owners, and follow-up checks.",
        "sales" => "Extract customer pain, objections, competitors, buying signals, stakeholders, next steps, and follow-up message points.",
        "hiring" => "Extract candidate signals, concerns, evidence, follow-up questions, interviewer alignment gaps, and recommendation confidence.",
        "manager" => "Extract commitments, blockers, morale signals, coaching topics, career goals, and recurring 1:1 themes.",
        "founder" => "Extract strategic decisions, investor/customer signals, risks, hiring/product/commercial priorities, and urgent follow-through.",
        "customer_success" => "Extract account health, risks, promised follow-ups, escalation points, adoption blockers, and renewal or expansion signals.",
        _ => "Extract decisions, actions, risks, owners, and follow-up questions.",
    }
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
        | "meetily_get_artifacts"
        | "meetily_get_latest_meeting"
        | "meetily_find_meetings"
        | "meetily_ask_meetings"
        | "meetily_get_recent_action_items"
        | "meetily_get_decisions"
        | "meetily_get_followups_for_person"
        | "meetily_get_meeting_brief"
        | "meetily_compare_meetings"
        | "meetily_get_project_context"
        | "meetily_get_daily_digest"
        | "meetily_get_weekly_digest"
        | "meetily_get_open_loops"
        | "meetily_prepare_next_meeting"
        | "meetily_prepare_role_brief"
        | "meetily_prepare_handoff" => SCOPE_READ_MEETINGS,
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
            ensure_meeting_exists(pool, &meeting_id).await?;
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
            ensure_meeting_exists(pool, &meeting_id).await?;
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
            ensure_meeting_exists(pool, &meeting_id).await?;
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
        "meetily_get_latest_meeting" => {
            let latest_id = latest_meeting_id(pool).await?;
            let include_transcript = bool_arg(args, "includeTranscript", true);
            let transcript_limit = bounded_i64_arg(args, "transcriptLimit", 12, 1, 50);
            json!({
                "meeting": meeting_card(pool, &latest_id, include_transcript, transcript_limit).await?
            })
        }
        "meetily_find_meetings" => {
            let limit = bounded_i64_arg(args, "limit", 10, 1, 50) as usize;
            let query = string_arg(args, "query");
            let person = string_arg(args, "person");
            let date_from = string_arg(args, "dateFrom");
            let date_to = string_arg(args, "dateTo");
            let ids = find_meeting_ids(
                pool,
                query.as_deref(),
                person.as_deref(),
                date_from.as_deref(),
                date_to.as_deref(),
                limit,
            )
            .await?;
            let mut meetings = Vec::new();
            for id in ids {
                meetings.push(meeting_card(pool, &id, false, 0).await?);
            }
            json!({
                "query": query,
                "person": person,
                "dateFrom": date_from,
                "dateTo": date_to,
                "meetings": meetings
            })
        }
        "meetily_ask_meetings" => {
            let question = string_arg(args, "question").ok_or_else(|| "question is required".to_string())?;
            let person = string_arg(args, "person");
            let topic = string_arg(args, "topic").or_else(|| Some(question.clone()));
            let latest_only = bool_arg(args, "latestOnly", false);
            let limit = if latest_only { 1 } else { bounded_i64_arg(args, "limit", 5, 1, 10) as usize };
            let ids = if latest_only {
                vec![latest_meeting_id(pool).await?]
            } else {
                find_meeting_ids(pool, topic.as_deref(), person.as_deref(), None, None, limit).await?
            };
            let mut context = Vec::new();
            for id in ids {
                let search_term = person.as_deref().or(topic.as_deref()).unwrap_or(&question);
                context.push(json!({
                    "meeting": meeting_card(pool, &id, false, 0).await?,
                    "matchingExcerpts": matching_transcript_excerpts(pool, &id, search_term, 8).await?
                }));
            }
            json!({
                "question": question,
                "answerGuidance": "Use the meeting summaries and matching excerpts as evidence. Cite meeting id/title and transcript timestamps when answering.",
                "context": context
            })
        }
        "meetily_get_recent_action_items" => {
            let limit = bounded_i64_arg(args, "limit", 25, 1, 100) as usize;
            let person = string_arg(args, "person");
            let topic = string_arg(args, "topic");
            let ids = find_meeting_ids(pool, topic.as_deref(), person.as_deref(), None, None, limit).await?;
            let mut items = Vec::new();
            for id in ids {
                let meeting = MeetingsRepository::get_meeting_metadata(pool, &id)
                    .await
                    .map_err(|error| format!("Failed to get meeting: {}", error))?
                    .ok_or_else(|| "Meeting not found".to_string())?;
                for item in get_action_context_json(pool, &id).await? {
                    if person.as_ref().map(|p| contains_ci(&item.to_string(), p)).unwrap_or(true) {
                        items.push(json!({
                            "meetingId": meeting.id,
                            "meetingTitle": meeting.title,
                            "createdAt": meeting.created_at.0.to_rfc3339(),
                            "item": item
                        }));
                    }
                    if items.len() >= limit {
                        break;
                    }
                }
                if items.len() >= limit {
                    break;
                }
            }
            json!({ "person": person, "topic": topic, "actionItems": items })
        }
        "meetily_get_decisions" => {
            let topic = string_arg(args, "topic");
            let limit = bounded_i64_arg(args, "limit", 20, 1, 50) as usize;
            let ids = find_meeting_ids(pool, topic.as_deref(), None, None, None, limit).await?;
            let mut decisions = Vec::new();
            for id in ids {
                let meeting = MeetingsRepository::get_meeting_metadata(pool, &id)
                    .await
                    .map_err(|error| format!("Failed to get meeting: {}", error))?
                    .ok_or_else(|| "Meeting not found".to_string())?;
                let decision_query = topic.as_deref().unwrap_or("decision");
                let excerpts = matching_transcript_excerpts(pool, &id, decision_query, 8).await?;
                decisions.push(json!({
                    "meetingId": meeting.id,
                    "meetingTitle": meeting.title,
                    "createdAt": meeting.created_at.0.to_rfc3339(),
                    "summary": get_summary_json(pool, &id).await?,
                    "matchingExcerpts": excerpts,
                    "guidance": "Inspect summary sections named decisions/key decisions and transcript excerpts for final decisions."
                }));
            }
            json!({ "topic": topic, "decisions": decisions })
        }
        "meetily_get_followups_for_person" => {
            let person = string_arg(args, "person").ok_or_else(|| "person is required".to_string())?;
            let limit = bounded_i64_arg(args, "limit", 25, 1, 50) as usize;
            let ids = find_meeting_ids(pool, Some(&person), Some(&person), None, None, limit).await?;
            let mut followups = Vec::new();
            for id in ids {
                let meeting = MeetingsRepository::get_meeting_metadata(pool, &id)
                    .await
                    .map_err(|error| format!("Failed to get meeting: {}", error))?
                    .ok_or_else(|| "Meeting not found".to_string())?;
                followups.push(json!({
                    "meetingId": meeting.id,
                    "meetingTitle": meeting.title,
                    "createdAt": meeting.created_at.0.to_rfc3339(),
                    "actionContext": get_action_context_json(pool, &id).await?,
                    "personExcerpts": matching_transcript_excerpts(pool, &id, &person, 8).await?
                }));
            }
            json!({ "person": person, "followups": followups })
        }
        "meetily_get_meeting_brief" => {
            let id = match meeting_id {
                Some(id) => id,
                None => latest_meeting_id(pool).await?,
            };
            let transcript_limit = bounded_i64_arg(args, "transcriptLimit", 25, 1, 100);
            json!({
                "brief": meeting_card(pool, &id, true, transcript_limit).await?,
                "suggestedUse": "Use this as a compact context packet before replying, filing issues, or planning follow-up work."
            })
        }
        "meetily_compare_meetings" => {
            let limit = bounded_i64_arg(args, "limit", 2, 2, 5) as usize;
            let mut ids = string_array_arg(args, "meetingIds");
            if ids.len() < 2 {
                let topic = string_arg(args, "topic");
                ids = find_meeting_ids(pool, topic.as_deref(), None, None, None, limit).await?;
            }
            if ids.len() < 2 {
                return Err("At least two meetings are required for comparison".to_string());
            }
            ids.truncate(limit);
            let mut meetings = Vec::new();
            for id in ids {
                meetings.push(meeting_card(pool, &id, true, 10).await?);
            }
            json!({
                "meetings": meetings,
                "comparisonGuidance": "Compare decisions, action items, summary deltas, risks, and repeated or changed topics across these meetings."
            })
        }
        "meetily_get_project_context" => {
            let topic = string_arg(args, "topic").ok_or_else(|| "topic is required".to_string())?;
            let limit = bounded_i64_arg(args, "limit", 10, 1, 20) as usize;
            let ids = find_meeting_ids(pool, Some(&topic), None, None, None, limit).await?;
            let mut timeline = Vec::new();
            for id in ids {
                let meeting = MeetingsRepository::get_meeting_metadata(pool, &id)
                    .await
                    .map_err(|error| format!("Failed to get meeting: {}", error))?
                    .ok_or_else(|| "Meeting not found".to_string())?;
                timeline.push(json!({
                    "meetingId": meeting.id,
                    "meetingTitle": meeting.title,
                    "createdAt": meeting.created_at.0.to_rfc3339(),
                    "summary": get_summary_json(pool, &id).await?,
                    "actionContext": get_action_context_json(pool, &id).await?,
                    "topicExcerpts": matching_transcript_excerpts(pool, &id, &topic, 8).await?
                }));
            }
            json!({ "topic": topic, "timeline": timeline })
        }
        "meetily_get_daily_digest" => {
            let date = string_arg(args, "date");
            let person = string_arg(args, "person");
            let topic = string_arg(args, "topic");
            let limit = bounded_i64_arg(args, "limit", 12, 1, 25) as usize;
            let (date_from, date_to) = day_bounds(date.as_deref())?;
            let ids = find_meeting_ids(
                pool,
                topic.as_deref(),
                person.as_deref(),
                Some(&date_from),
                Some(&date_to),
                limit,
            )
            .await?;
            json!({
                "date": date_from,
                "person": person,
                "topic": topic,
                "meetings": digest_meeting_cards(pool, ids, 8).await?,
                "digestGuidance": "Create a concise personal digest: meetings attended, decisions, commitments I made, commitments others made to me, risks, unresolved questions, and suggested next actions. Cite meeting ids and transcript timestamps where available."
            })
        }
        "meetily_get_weekly_digest" => {
            let person = string_arg(args, "person");
            let topic = string_arg(args, "topic");
            let date_from = string_arg(args, "dateFrom");
            let date_to = string_arg(args, "dateTo");
            let limit = bounded_i64_arg(args, "limit", 25, 1, 50) as usize;
            let (from_bound, to_bound) = default_week_bounds(date_from.as_deref(), date_to.as_deref())?;
            let ids = find_meeting_ids(
                pool,
                topic.as_deref(),
                person.as_deref(),
                Some(&from_bound),
                Some(&to_bound),
                limit,
            )
            .await?;
            json!({
                "dateFrom": from_bound,
                "dateTo": to_bound,
                "person": person,
                "topic": topic,
                "meetings": digest_meeting_cards(pool, ids, 5).await?,
                "digestGuidance": "Create a weekly digest grouped by decisions, progress, commitments, risks, repeated themes, and recommended follow-up. Highlight items that appeared in multiple meetings."
            })
        }
        "meetily_get_open_loops" => {
            let person = string_arg(args, "person");
            let topic = string_arg(args, "topic");
            let limit = bounded_i64_arg(args, "limit", 25, 1, 50) as usize;
            let ids = find_meeting_ids(pool, topic.as_deref(), person.as_deref(), None, None, limit).await?;
            let mut loops = Vec::new();
            for id in ids {
                let search_term = topic.as_deref().or(person.as_deref()).unwrap_or("?");
                loops.push(json!({
                    "meeting": meeting_card(pool, &id, false, 0).await?,
                    "actionContext": get_action_context_json(pool, &id).await?,
                    "questionExcerpts": matching_transcript_excerpts(pool, &id, search_term, 8).await?
                }));
            }
            json!({
                "person": person,
                "topic": topic,
                "openLoops": loops,
                "openLoopGuidance": "Identify unresolved questions, promised follow-ups, ownerless action items, risks without mitigation, and decisions that need confirmation. Mark uncertainty clearly."
            })
        }
        "meetily_prepare_next_meeting" => {
            let person = string_arg(args, "person");
            let topic = string_arg(args, "topic");
            if person.is_none() && topic.is_none() {
                return Err("person or topic is required".to_string());
            }
            let limit = bounded_i64_arg(args, "limit", 5, 1, 10) as usize;
            let ids = find_meeting_ids(pool, topic.as_deref(), person.as_deref(), None, None, limit).await?;
            json!({
                "person": person,
                "topic": topic,
                "sourceMeetings": digest_meeting_cards(pool, ids, 10).await?,
                "prepGuidance": "Prepare a next-meeting brief: previous decisions, unresolved questions, promised follow-ups, likely agenda, suggested questions to ask, and points that need confirmation. Cite source meetings."
            })
        }
        "meetily_prepare_role_brief" => {
            let role = string_arg(args, "role").ok_or_else(|| "role is required".to_string())?;
            let person = string_arg(args, "person");
            let topic = string_arg(args, "topic");
            let limit = bounded_i64_arg(args, "limit", 5, 1, 10) as usize;
            let ids = find_meeting_ids(pool, topic.as_deref(), person.as_deref(), None, None, limit).await?;
            json!({
                "role": role,
                "person": person,
                "topic": topic,
                "sourceMeetings": digest_meeting_cards(pool, ids, 10).await?,
                "briefGuidance": role_brief_guidance(&role),
                "outputGuidance": "Return a concise brief with evidence, owners, confidence, and explicit follow-up recommendations. Do not create external records without user approval."
            })
        }
        "meetily_prepare_handoff" => {
            let agent = string_arg(args, "agent").unwrap_or_else(|| "manual".to_string());
            let topic = string_arg(args, "topic");
            let limit = bounded_i64_arg(args, "limit", 3, 1, 10) as usize;
            let ids = match meeting_id {
                Some(id) => vec![id],
                None => find_meeting_ids(pool, topic.as_deref(), None, None, None, limit).await?,
            };
            let mut source = Vec::new();
            for id in ids {
                source.push(meeting_card(pool, &id, true, 15).await?);
            }
            json!({
                "agent": agent,
                "topic": topic,
                "sourceMeetings": source,
                "handoffPrompt": "Review the source meetings. Extract concrete follow-ups, decisions, risks, and implementation tasks. Preserve citations to meeting ids and transcript timestamps. Do not invent facts outside this MCP context."
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
                        "policy": "local-only read-only meeting tools are available without authorization on 127.0.0.1; bearer tokens remain supported for trusted client audit"
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
    let clients =
        revoke_trusted_client(&client_id).map_err(|_| "Unable to revoke MCP client".to_string())?;
    append_audit_event(McpAuditEvent {
        id: Uuid::new_v4().to_string(),
        timestamp: now_string(),
        client_id,
        tool_name: "mcp_revoke_client".to_string(),
        scopes: Vec::new(),
        meeting_ids: Vec::new(),
        result: "revoked".to_string(),
        reason: Some("user_revoked_client".to_string()),
    });
    Ok(clients)
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
    pub endpoint_configured: bool,
    pub working: bool,
    pub status: String,
    pub last_checked_at: String,
    pub message: String,
    pub invocation_mode: String,
    pub capabilities: Vec<String>,
    pub fallback: String,
    pub setup_hint: String,
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

fn command_exists(command: &str) -> bool {
    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths).any(|dir| {
        let candidate = dir.join(command);
        candidate.is_file()
            || if cfg!(target_os = "windows") {
                dir.join(format!("{command}.exe")).is_file()
            } else {
                false
            }
    })
}

fn macos_app_exists(name: &str) -> bool {
    cfg!(target_os = "macos") && Path::new("/Applications").join(name).exists()
}

fn agent_installed(agent: &AgentKind, config_path: &Path) -> bool {
    let config_parent_exists = config_path
        .parent()
        .map(|parent| parent.exists())
        .unwrap_or(false);
    match agent {
        AgentKind::Claude => {
            config_parent_exists || command_exists("claude") || macos_app_exists("Claude.app")
        }
        AgentKind::Codex => config_parent_exists || command_exists("codex"),
        AgentKind::Cursor => {
            config_parent_exists || command_exists("cursor") || macos_app_exists("Cursor.app")
        }
    }
}

fn agent_invocation_mode(agent: &AgentKind) -> &'static str {
    match agent {
        AgentKind::Claude | AgentKind::Codex | AgentKind::Cursor => "copyPrompt",
    }
}

fn agent_fallback(agent: &AgentKind) -> &'static str {
    match agent {
        AgentKind::Claude => "Open Claude Desktop and paste the prepared prompt. The prompt references Meetily MCP sources after setup.",
        AgentKind::Codex => "Open Codex in the target workspace and paste the prepared prompt. The prompt references Meetily MCP sources after setup.",
        AgentKind::Cursor => "Open Cursor and paste the prepared prompt. The prompt references Meetily MCP sources after setup.",
    }
}

fn agent_setup_hint(agent: &AgentKind) -> &'static str {
    match agent {
        AgentKind::Claude => "Install Claude Desktop or create its config file, then run setup to add the Meetily MCP server.",
        AgentKind::Codex => "Install Codex or create ~/.codex/config.toml, then run setup to add the Meetily MCP server.",
        AgentKind::Cursor => "Install Cursor or create ~/.cursor/mcp.json, then run setup to add the Meetily MCP server.",
    }
}

fn agent_capabilities(agent: &AgentKind, configured: bool, working: bool) -> Vec<String> {
    let mut capabilities = vec![
        "Source-cited meeting handoff prompt".to_string(),
        "Copyable fallback prompt".to_string(),
    ];

    if configured {
        capabilities.push("Meetily MCP endpoint configured".to_string());
    }

    if working {
        capabilities.push("Local MCP server reachable".to_string());
    }

    match agent {
        AgentKind::Codex => {
            capabilities
                .push("Agent can use local codebase and developer tools after handoff".to_string());
        }
        AgentKind::Claude => {
            capabilities.push("Agent can use Claude Desktop MCP tools after handoff".to_string());
        }
        AgentKind::Cursor => {
            capabilities.push("Agent can use Cursor workspace context after handoff".to_string());
        }
    }

    capabilities
}

fn status_for_agent(
    agent: AgentKind,
    settings: &McpSettings,
    server_running: bool,
) -> AgentSetupStatus {
    let path = agent_config_path(&agent).unwrap_or_default();
    let installed = agent_installed(&agent, &path);
    status_for_agent_at_path(agent, path, settings, server_running, installed)
}

fn status_for_agent_at_path(
    agent: AgentKind,
    path: PathBuf,
    settings: &McpSettings,
    server_running: bool,
    installed: bool,
) -> AgentSetupStatus {
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
        endpoint_configured: configured,
        working,
        status: status.to_string(),
        last_checked_at: now_string(),
        message: match status {
            "working" => "Configured and MCP server is running.".to_string(),
            "configured" => "Configured. Start MCP to complete the working check.".to_string(),
            "notConfigured" => "App config found, but Meetily MCP is not configured.".to_string(),
            _ => "Config folder was not found on this machine.".to_string(),
        },
        invocation_mode: agent_invocation_mode(&agent).to_string(),
        capabilities: agent_capabilities(&agent, configured, working),
        fallback: agent_fallback(&agent).to_string(),
        setup_hint: agent_setup_hint(&agent).to_string(),
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

    let safe_url = toml_string_value(url);
    let safe_header = toml_string_value(&format!("Authorization: Bearer {}", token));
    raw.push_str(&format!(
        "\n[mcp_servers.meetily]\ncommand = \"npx\"\nargs = [\"-y\", \"mcp-remote\", \"{}\", \"--header\", \"{}\"]\nenabled = true\n",
        safe_url, safe_header
    ));
    fs::write(path, raw)?;
    Ok(())
}

fn toml_string_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
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
    if config_contains_meetily(&path, &agent, &url) && active_agent_client_exists(&agent) {
        return Ok(status_for_agent(agent, &settings, running));
    }
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
    if config_contains_meetily(&path, &agent, &url) && active_agent_client_exists(&agent) {
        return Ok(status_for_agent(agent, settings, running));
    }
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
    fn missing_authorization_uses_local_loopback_client() {
        let client = authorize_client(None, SCOPE_READ_MEETINGS).unwrap();
        assert_eq!(client.id, LOCAL_LOOPBACK_CLIENT_ID);
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

    #[test]
    fn readiness_status_does_not_expose_authorization_token() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        merge_codex_config(
            &temp.path().to_path_buf(),
            "http://127.0.0.1:43118/mcp",
            "local-secret-token",
        )
        .unwrap();

        let status = status_for_agent_at_path(
            AgentKind::Codex,
            temp.path().to_path_buf(),
            &McpSettings::default(),
            true,
            true,
        );
        let serialized = serde_json::to_string(&status).unwrap();

        assert!(status.working);
        assert!(status.endpoint_configured);
        assert!(status
            .capabilities
            .iter()
            .any(|capability| capability.contains("MCP")));
        assert!(!serialized.contains("local-secret-token"));
        assert!(!serialized.contains("Authorization: Bearer"));
    }
}
