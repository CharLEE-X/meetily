use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io,
    path::PathBuf,
    sync::Arc,
};
use tauri::{AppHandle, Manager, State};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{oneshot, Mutex},
    task::JoinHandle,
};
use uuid::Uuid;

const DEFAULT_PORT: u16 = 43118;
const CONFIG_DIR_NAME: &str = "meetily";
const SETTINGS_FILE_NAME: &str = "mcp_settings.json";
const AUDIT_FILE_NAME: &str = "mcp_audit_log.json";

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
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_audit_events(events: &[McpAuditEvent]) -> Result<()> {
    let path = audit_path()?;
    let raw = serde_json::to_string_pretty(events)?;
    fs::write(path, raw)?;
    Ok(())
}

fn append_audit_event(event: McpAuditEvent) {
    let mut events = load_audit_events();
    events.insert(0, event);
    events.truncate(100);
    if let Err(error) = save_audit_events(&events) {
        log::warn!("Failed to save MCP audit event: {}", error);
    }
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

async fn write_response(socket: &mut tokio::net::TcpStream, status: &str, body: &str) -> io::Result<()> {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: http://localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.as_bytes().len(),
        body
    );
    socket.write_all(response.as_bytes()).await
}

fn json_rpc_response(request: &str) -> String {
    let parsed: Value = serde_json::from_str(request).unwrap_or_else(|_| json!({}));
    let id = parsed.get("id").cloned().unwrap_or(Value::Null);
    let method = parsed.get("method").and_then(Value::as_str).unwrap_or("");

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-06-18",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "meetily", "version": env!("CARGO_PKG_VERSION") }
        }),
        "tools/list" => json!({
            "tools": [
                {
                    "name": "meetily_status",
                    "description": "Read the local Meetily MCP server status without exposing meeting content.",
                    "inputSchema": { "type": "object", "properties": {} }
                }
            ]
        }),
        "tools/call" => {
            let tool_name = parsed
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            append_audit_event(McpAuditEvent {
                id: Uuid::new_v4().to_string(),
                timestamp: now_string(),
                client_id: "local-client".to_string(),
                tool_name: tool_name.to_string(),
                scopes: vec!["mcp:read_status".to_string()],
                meeting_ids: Vec::new(),
                result: "allowed".to_string(),
                reason: None,
            });
            json!({
                "content": [
                    { "type": "text", "text": "Meetily MCP server is running. Meeting-content tools are not enabled yet." }
                ],
                "isError": false
            })
        }
        _ => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            })
            .to_string();
        }
    };

    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

async fn handle_connection(mut socket: tokio::net::TcpStream) -> io::Result<()> {
    let mut buffer = vec![0; 8192];
    let n = socket.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
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
        let body = json_rpc_response(body);
        return write_response(&mut socket, "200 OK", &body).await;
    }

    write_response(&mut socket, "404 Not Found", &json!({ "error": "not_found" }).to_string()).await
}

async fn run_server(port: u16, mut shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .with_context(|| format!("Unable to bind MCP server to 127.0.0.1:{}", port))?;

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((socket, _)) => {
                        tokio::spawn(async move {
                            if let Err(error) = handle_connection(socket).await {
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

async fn start_runtime(runtime: &mut McpRuntime) -> Result<()> {
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
    drop(listener);

    let task = tokio::spawn(async move {
        if let Err(error) = run_server(port, shutdown_rx).await {
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
        if let Err(error) = start_runtime(&mut runtime).await {
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
    runtime.settings = load_settings().unwrap_or_else(|_| runtime.settings.clone());
    Ok(build_status(runtime.settings.clone(), &runtime))
}

#[tauri::command]
pub async fn mcp_update_settings(settings: McpSettings, state: State<'_, McpState>) -> Result<McpStatus, String> {
    if settings.port == 0 {
        return Err("Port must be greater than zero".to_string());
    }

    save_settings(&settings).map_err(|_| "Unable to save MCP settings".to_string())?;

    let mut runtime = state.runtime.lock().await;
    runtime.settings = settings.clone();

    if settings.enabled {
        if let Err(error) = start_runtime(&mut runtime).await {
            runtime.state = McpServerState::Error;
            runtime.last_error = Some(error.to_string());
        }
    } else {
        stop_runtime(&mut runtime).await;
    }

    Ok(build_status(settings, &runtime))
}

#[tauri::command]
pub async fn mcp_start_server(state: State<'_, McpState>) -> Result<McpStatus, String> {
    let mut settings = load_settings().unwrap_or_default();
    settings.enabled = true;
    save_settings(&settings).map_err(|_| "Unable to save MCP settings".to_string())?;

    let mut runtime = state.runtime.lock().await;
    runtime.settings = settings.clone();
    if let Err(error) = start_runtime(&mut runtime).await {
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
    Ok(Vec::new())
}

#[tauri::command]
pub async fn mcp_revoke_client(_client_id: String) -> Result<Vec<McpClient>, String> {
    Ok(Vec::new())
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
        AgentKind::Claude => home.join("Library/Application Support/Claude/claude_desktop_config.json"),
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
        AgentKind::Codex => raw.contains("[mcp_servers.meetily]") && raw.contains("mcp-remote") && raw.contains(url),
        AgentKind::Claude | AgentKind::Cursor => raw.contains("\"meetily\"") && raw.contains(url),
    }
}

fn status_for_agent(agent: AgentKind, settings: &McpSettings, server_running: bool) -> AgentSetupStatus {
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

fn merge_json_mcp_config(path: &PathBuf, agent: &AgentKind, url: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let backup = path.with_extension("json.meetily-backup");
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
        AgentKind::Cursor => json!({ "url": url, "transport": "streamable-http" }),
        AgentKind::Claude => json!({
            "command": "npx",
            "args": ["-y", "mcp-remote", url]
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

fn merge_codex_config(path: &PathBuf, url: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        let backup = path.with_extension("toml.meetily-backup");
        fs::copy(path, backup)?;
    }

    let mut raw = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if raw.contains("[mcp_servers.meetily]") {
        return Ok(());
    }

    if !raw.ends_with('\n') && !raw.is_empty() {
        raw.push('\n');
    }

    raw.push_str(&format!(
        "\n[mcp_servers.meetily]\ncommand = \"npx\"\nargs = [\"-y\", \"mcp-remote\", \"{}\"]\nenabled = true\n",
        url.replace('"', "")
    ));
    fs::write(path, raw)?;
    Ok(())
}

#[tauri::command]
pub async fn mcp_get_agent_statuses(state: State<'_, McpState>) -> Result<Vec<AgentSetupStatus>, String> {
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
pub async fn mcp_setup_agent(agent: AgentKind, state: State<'_, McpState>) -> Result<AgentSetupStatus, String> {
    let runtime = state.runtime.lock().await;
    let settings = runtime.settings.clone();
    let running = runtime.state == McpServerState::Running;
    drop(runtime);

    let path = agent_config_path(&agent).map_err(|_| "Unable to find the user home directory".to_string())?;
    let url = expected_url(&settings);

    let result = match agent {
        AgentKind::Claude | AgentKind::Cursor => merge_json_mcp_config(&path, &agent, &url),
        AgentKind::Codex => merge_codex_config(&path, &url),
    };

    result.map_err(|_| format!("Unable to update {} MCP configuration", agent_label(&agent)))?;
    Ok(status_for_agent(agent, &settings, running))
}

fn setup_agent_inner(agent: AgentKind, settings: &McpSettings, running: bool) -> Result<AgentSetupStatus> {
    let path = agent_config_path(&agent)?;
    let url = expected_url(settings);

    match agent {
        AgentKind::Claude | AgentKind::Cursor => merge_json_mcp_config(&path, &agent, &url)?,
        AgentKind::Codex => merge_codex_config(&path, &url)?,
    };

    Ok(status_for_agent(agent, settings, running))
}

#[tauri::command]
pub async fn mcp_setup_all_agents(state: State<'_, McpState>) -> Result<Vec<AgentSetupStatus>, String> {
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
        let response = json_rpc_response(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
        assert!(response.contains("\"name\":\"meetily\""));
        assert!(response.contains("\"protocolVersion\""));
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
        fs::write(temp.path(), r#"{"mcpServers":{"existing":{"url":"http://example.test"}}}"#).unwrap();
        merge_json_mcp_config(
            &temp.path().to_path_buf(),
            &AgentKind::Cursor,
            "http://127.0.0.1:43118/mcp",
        )
        .unwrap();
        let raw = fs::read_to_string(temp.path()).unwrap();
        assert!(raw.contains("\"existing\""));
        assert!(raw.contains("\"meetily\""));
    }
}
