use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use tauri::State;
use uuid::Uuid;

const PROVIDER_APPLE_REMINDERS: &str = "apple_reminders";
const APPLE_REMINDERS_LABEL: &str = "Apple Reminders";
const REMINDER_CATEGORIES: [&str; 7] = [
    "pr_review",
    "linear_follow_up",
    "deploy_alert_check",
    "docs_update",
    "implementation_task",
    "experiment_revisit",
    "clarification_follow_up",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderProviderInfo {
    pub provider: String,
    pub label: String,
    pub available: bool,
    pub supports_list_discovery: bool,
    pub supports_create: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderProviderAccount {
    pub id: String,
    pub provider: String,
    pub account_label: String,
    pub status: String,
    pub default_list_id: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderList {
    pub id: String,
    pub provider_account_id: String,
    pub provider_list_id: String,
    pub name: String,
    pub color: Option<String>,
    pub selected: bool,
    pub is_default: bool,
    pub last_seen_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderSettingsState {
    pub providers: Vec<ReminderProviderInfo>,
    pub accounts: Vec<ReminderProviderAccount>,
    pub lists: Vec<ReminderList>,
    pub workflow_settings: ReminderWorkflowSettings,
    pub workflow_presets: Vec<ReminderWorkflowPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderListSyncRequest {
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderListSyncResult {
    pub provider: String,
    pub status: String,
    pub synced_list_count: usize,
    pub started_at: String,
    pub completed_at: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDefaultListRequest {
    pub provider: Option<String>,
    pub list_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderWorkflowSettings {
    pub provider: String,
    pub global_priority: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderWorkflowPreset {
    pub category: String,
    pub enabled: bool,
    pub default_list_id: Option<String>,
    pub default_priority: Option<i64>,
    pub due_preset: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderWorkflowPresetUpdateRequest {
    pub category: Option<String>,
    pub global_priority: Option<i64>,
    pub enabled: Option<bool>,
    pub default_list_id: Option<String>,
    pub use_global_list: Option<bool>,
    pub default_priority: Option<i64>,
    pub use_global_priority: Option<bool>,
    pub due_preset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDraftRequest {
    pub meeting_id: String,
    pub include_low_confidence: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderSourceEvidence {
    pub label: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDraft {
    pub id: String,
    pub meeting_id: String,
    pub summary_id: Option<String>,
    pub title: String,
    pub notes: Option<String>,
    pub due_at: Option<String>,
    pub priority: Option<i64>,
    pub list_id: Option<String>,
    pub category: String,
    pub confidence: f64,
    pub source_evidence: Vec<ReminderSourceEvidence>,
    pub dedupe_key: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDraftGenerationResult {
    pub meeting_id: String,
    pub drafts: Vec<ReminderDraft>,
    pub hidden_low_confidence_count: usize,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDraftUpdateRequest {
    pub draft_id: String,
    pub title: String,
    pub notes: Option<String>,
    pub due_at: Option<String>,
    pub priority: Option<i64>,
    pub list_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReminderRequest {
    pub meeting_id: String,
    pub draft_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedReminderLink {
    pub id: String,
    pub meeting_id: String,
    pub meeting_title: Option<String>,
    pub draft_id: Option<String>,
    pub dedupe_key: String,
    pub provider: String,
    pub provider_reminder_id: String,
    pub list_id: Option<String>,
    pub list_name: Option<String>,
    pub title: String,
    pub due_at: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderCreationFailure {
    pub draft_id: String,
    pub title: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReminderResult {
    pub meeting_id: String,
    pub created: Vec<CreatedReminderLink>,
    pub skipped: Vec<CreatedReminderLink>,
    pub failed: Vec<ReminderCreationFailure>,
}

#[tauri::command]
pub async fn list_reminder_providers() -> Result<Vec<ReminderProviderInfo>, String> {
    Ok(provider_infos())
}

#[tauri::command]
pub async fn get_reminder_settings(
    state: State<'_, AppState>,
) -> Result<ReminderSettingsState, String> {
    let pool = state.db_manager.pool();
    ensure_reminder_workflow_defaults(pool).await?;
    Ok(ReminderSettingsState {
        providers: provider_infos(),
        accounts: list_accounts(pool).await?,
        lists: list_lists(pool).await?,
        workflow_settings: get_workflow_settings(pool).await?,
        workflow_presets: list_workflow_presets(pool).await?,
    })
}

#[tauri::command]
pub async fn connect_reminder_provider(
    state: State<'_, AppState>,
    provider: String,
) -> Result<ReminderProviderAccount, String> {
    let provider = normalize_provider(&provider)?;
    connect_provider_account(state.db_manager.pool(), &provider).await
}

#[tauri::command]
pub async fn disconnect_reminder_provider(
    state: State<'_, AppState>,
    provider: String,
) -> Result<ReminderProviderAccount, String> {
    let provider = normalize_provider(&provider)?;
    let pool = state.db_manager.pool();
    let now = chrono::Utc::now().to_rfc3339();
    let account_id = existing_account_id(pool, &provider)
        .await?
        .ok_or_else(|| "Reminder provider is not connected".to_string())?;

    sqlx::query(
        "UPDATE reminder_lists
         SET selected = 0, is_default = 0, updated_at = ?
         WHERE provider_account_id = ?",
    )
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to disable reminder lists: {}", err))?;

    sqlx::query(
        "UPDATE reminder_provider_accounts
         SET status = 'revoked', default_list_id = NULL, last_error = NULL,
             last_sync_at = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to disconnect reminder provider: {}", err))?;

    get_account(pool, &provider)
        .await?
        .ok_or_else(|| "Reminder provider was not found after disconnect".to_string())
}

#[tauri::command]
pub async fn sync_reminder_lists(
    state: State<'_, AppState>,
    request: Option<ReminderListSyncRequest>,
) -> Result<ReminderListSyncResult, String> {
    let request = request.unwrap_or(ReminderListSyncRequest { provider: None });
    let provider = normalize_provider(
        request
            .provider
            .as_deref()
            .unwrap_or(PROVIDER_APPLE_REMINDERS),
    )?;
    let pool = state.db_manager.pool();
    let started_at = chrono::Utc::now();
    let account = match get_account(pool, &provider).await? {
        Some(account) if account.status != "revoked" => account,
        _ => {
            return Err("Connect Apple Reminders before refreshing reminder lists.".to_string());
        }
    };

    let result = if provider == PROVIDER_APPLE_REMINDERS {
        sync_apple_reminder_lists(pool, &account.id).await
    } else {
        Err("This reminder provider is not implemented yet.".to_string())
    };

    let completed_at = chrono::Utc::now();
    match result {
        Ok(count) => {
            update_account_sync(pool, &provider, "connected", Some(completed_at), None).await?;
            Ok(ReminderListSyncResult {
                provider,
                status: "connected".to_string(),
                synced_list_count: count,
                started_at: started_at.to_rfc3339(),
                completed_at: completed_at.to_rfc3339(),
                error: None,
            })
        }
        Err(error) => {
            let status = sync_error_status(&error);
            let user_error = sanitize_reminder_error(&error);
            update_account_sync(
                pool,
                &provider,
                status,
                Some(completed_at),
                Some(&user_error),
            )
            .await?;
            Ok(ReminderListSyncResult {
                provider,
                status: status.to_string(),
                synced_list_count: 0,
                started_at: started_at.to_rfc3339(),
                completed_at: completed_at.to_rfc3339(),
                error: Some(user_error),
            })
        }
    }
}

#[tauri::command]
pub async fn update_default_reminder_list(
    state: State<'_, AppState>,
    request: ReminderDefaultListRequest,
) -> Result<ReminderProviderAccount, String> {
    let provider = normalize_provider(
        request
            .provider
            .as_deref()
            .unwrap_or(PROVIDER_APPLE_REMINDERS),
    )?;
    let pool = state.db_manager.pool();
    let now = chrono::Utc::now().to_rfc3339();
    let account_id = existing_account_id(pool, &provider)
        .await?
        .ok_or_else(|| "Reminder provider is not connected".to_string())?;

    let list_exists: Option<String> = sqlx::query_scalar(
        "SELECT id FROM reminder_lists WHERE id = ? AND provider_account_id = ?",
    )
    .bind(&request.list_id)
    .bind(&account_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect reminder list: {}", err))?;

    if list_exists.is_none() {
        return Err("Selected reminder list is not available.".to_string());
    }

    sqlx::query(
        "UPDATE reminder_lists SET is_default = 0, updated_at = ? WHERE provider_account_id = ?",
    )
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update reminder lists: {}", err))?;

    sqlx::query(
        "UPDATE reminder_lists
         SET selected = 1, is_default = 1, updated_at = ?
         WHERE id = ?",
    )
    .bind(&now)
    .bind(&request.list_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to set default reminder list: {}", err))?;

    sqlx::query(
        "UPDATE reminder_provider_accounts
         SET default_list_id = ?, status = 'connected', last_error = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(&request.list_id)
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save default reminder list: {}", err))?;

    get_account(pool, &provider)
        .await?
        .ok_or_else(|| "Reminder provider was not found after updating default list".to_string())
}

#[tauri::command]
pub async fn update_reminder_workflow_preset(
    state: State<'_, AppState>,
    request: ReminderWorkflowPresetUpdateRequest,
) -> Result<ReminderSettingsState, String> {
    let pool = state.db_manager.pool();
    ensure_reminder_workflow_defaults(pool).await?;
    update_workflow_preset(pool, request).await?;
    Ok(ReminderSettingsState {
        providers: provider_infos(),
        accounts: list_accounts(pool).await?,
        lists: list_lists(pool).await?,
        workflow_settings: get_workflow_settings(pool).await?,
        workflow_presets: list_workflow_presets(pool).await?,
    })
}

#[tauri::command]
pub async fn generate_reminder_drafts(
    state: State<'_, AppState>,
    request: ReminderDraftRequest,
) -> Result<ReminderDraftGenerationResult, String> {
    generate_reminder_drafts_for_meeting(
        state.db_manager.pool(),
        &request.meeting_id,
        request.include_low_confidence.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn list_reminder_drafts(
    state: State<'_, AppState>,
    meeting_id: String,
    include_low_confidence: Option<bool>,
) -> Result<Vec<ReminderDraft>, String> {
    list_drafts(
        state.db_manager.pool(),
        &meeting_id,
        include_low_confidence.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn update_reminder_draft(
    state: State<'_, AppState>,
    request: ReminderDraftUpdateRequest,
) -> Result<ReminderDraft, String> {
    update_draft(state.db_manager.pool(), request).await
}

#[tauri::command]
pub async fn dismiss_reminder_draft(
    state: State<'_, AppState>,
    draft_id: String,
) -> Result<ReminderDraft, String> {
    set_draft_status(state.db_manager.pool(), &draft_id, "dismissed").await
}

#[tauri::command]
pub async fn create_selected_reminders(
    state: State<'_, AppState>,
    request: CreateReminderRequest,
) -> Result<CreateReminderResult, String> {
    create_reminders_for_drafts(state.db_manager.pool(), request).await
}

#[tauri::command]
pub async fn list_created_reminders(
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<CreatedReminderLink>, String> {
    refresh_created_link_statuses(state.db_manager.pool(), Some(&meeting_id)).await?;
    list_created_links(state.db_manager.pool(), &meeting_id).await
}

#[tauri::command]
pub async fn list_recent_created_reminders(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<CreatedReminderLink>, String> {
    refresh_created_link_statuses(state.db_manager.pool(), None).await?;
    list_recent_created_links(state.db_manager.pool(), limit.unwrap_or(12).clamp(1, 50)).await
}

fn provider_infos() -> Vec<ReminderProviderInfo> {
    vec![ReminderProviderInfo {
        provider: PROVIDER_APPLE_REMINDERS.to_string(),
        label: APPLE_REMINDERS_LABEL.to_string(),
        available: cfg!(target_os = "macos"),
        supports_list_discovery: cfg!(target_os = "macos"),
        supports_create: cfg!(target_os = "macos"),
        notes: Some(
            if cfg!(target_os = "macos") {
                "Creates selected follow-up reminders only after review and explicit confirmation."
            } else {
                "Apple Reminders is available only on macOS."
            }
            .to_string(),
        ),
    }]
}

async fn connect_provider_account(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<ReminderProviderAccount, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let account_id = existing_account_id(pool, provider)
        .await?
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let status = if provider == PROVIDER_APPLE_REMINDERS && cfg!(target_os = "macos") {
        "permission_needed"
    } else {
        "error"
    };
    let error = if status == "error" {
        Some("Apple Reminders is not available on this platform.".to_string())
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO reminder_provider_accounts
            (id, provider, account_label, status, default_list_id, last_sync_at, last_error, created_at, updated_at)
         VALUES (?, ?, ?, ?, NULL, NULL, ?, ?, ?)
         ON CONFLICT(provider) DO UPDATE SET
            account_label = excluded.account_label,
            status = excluded.status,
            last_error = excluded.last_error,
            updated_at = excluded.updated_at",
    )
    .bind(&account_id)
    .bind(provider)
    .bind(provider_label(provider))
    .bind(status)
    .bind(error)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to connect reminder provider: {}", err))?;

    get_account(pool, provider)
        .await?
        .ok_or_else(|| "Reminder provider was not saved".to_string())
}

async fn sync_apple_reminder_lists(
    pool: &sqlx::SqlitePool,
    account_id: &str,
) -> Result<usize, String> {
    if !cfg!(target_os = "macos") {
        return Err("Apple Reminders is not available on this platform.".to_string());
    }

    let script = apple_reminder_lists_script();
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|err| format!("Failed to run Apple Reminders list discovery: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "Apple Reminders permission is required before listing reminder lists.".to_string()
        } else {
            stderr
        };
        return Err(message);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Err(
            "Apple Reminders permission is required before listing reminder lists.".to_string(),
        );
    }

    let mut synced = 0usize;
    let mut seen_list_ids = Vec::new();
    for row in stdout
        .split('\u{1d}')
        .filter(|line| !line.trim().is_empty())
    {
        match parse_apple_reminder_list_row(row) {
            Ok(Some(list)) => {
                let list_id = upsert_list(pool, account_id, &list).await?;
                seen_list_ids.push(list_id);
                synced += 1;
            }
            Ok(None) => {}
            Err(error) => {
                log::warn!("Skipping malformed Apple Reminders list row: {}", error);
            }
        }
    }

    mark_missing_lists_unselected(pool, account_id, &seen_list_ids).await?;
    ensure_default_list(pool, account_id).await?;
    Ok(synced)
}

fn apple_reminder_lists_script() -> String {
    r#"set rowDelimiter to character id 29
set fieldDelimiter to character id 30
set rows to {}
tell application id "com.apple.reminders"
    repeat with reminderList in lists
        set listName to name of reminderList as text
        set listId to listName
        try
            set listId to id of reminderList as text
        end try
        set listName to my cleanReminderField(listName)
        set listId to my cleanReminderField(listId)
        copy (listId & fieldDelimiter & listName) to end of rows
    end repeat
end tell
set AppleScript's text item delimiters to rowDelimiter
set outputRows to rows as text
set AppleScript's text item delimiters to ""
return outputRows

on cleanReminderField(rawValue)
    set valueText to rawValue as text
    set AppleScript's text item delimiters to {character id 29, character id 30, linefeed, return}
    set parts to text items of valueText
    set AppleScript's text item delimiters to " "
    set cleanedValue to parts as text
    set AppleScript's text item delimiters to ""
    return cleanedValue
end cleanReminderField
"#
    .to_string()
}

#[derive(Debug, Clone)]
struct ParsedReminderList {
    provider_list_id: String,
    name: String,
}

fn parse_apple_reminder_list_row(row: &str) -> Result<Option<ParsedReminderList>, String> {
    let fields: Vec<&str> = row.split('\u{1e}').collect();
    if fields.len() < 2 {
        return Err("Apple Reminders row did not contain list id and name".to_string());
    }
    let provider_list_id = fields[0].trim();
    let name = fields[1].trim();
    if provider_list_id.is_empty() || name.is_empty() {
        return Ok(None);
    }

    Ok(Some(ParsedReminderList {
        provider_list_id: provider_list_id.to_string(),
        name: name.to_string(),
    }))
}

async fn upsert_list(
    pool: &sqlx::SqlitePool,
    account_id: &str,
    list: &ParsedReminderList,
) -> Result<String, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let list_id = existing_list_id(pool, account_id, &list.provider_list_id)
        .await?
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        "INSERT INTO reminder_lists
            (id, provider_account_id, provider_list_id, name, color, selected, is_default, last_seen_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, NULL, 1, 0, ?, ?, ?)
         ON CONFLICT(provider_account_id, provider_list_id) DO UPDATE SET
            name = excluded.name,
            selected = 1,
            last_seen_at = excluded.last_seen_at,
            updated_at = excluded.updated_at",
    )
    .bind(&list_id)
    .bind(account_id)
    .bind(&list.provider_list_id)
    .bind(&list.name)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save reminder list: {}", err))?;

    Ok(list_id)
}

async fn mark_missing_lists_unselected(
    pool: &sqlx::SqlitePool,
    account_id: &str,
    seen_list_ids: &[String],
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let rows = sqlx::query("SELECT id FROM reminder_lists WHERE provider_account_id = ?")
        .bind(account_id)
        .fetch_all(pool)
        .await
        .map_err(|err| format!("Failed to inspect reminder lists: {}", err))?;
    for row in rows {
        let id: String = row.get("id");
        if !seen_list_ids.contains(&id) {
            sqlx::query(
                "UPDATE reminder_lists
                 SET selected = 0, is_default = 0, updated_at = ?
                 WHERE id = ?",
            )
            .bind(&now)
            .bind(&id)
            .execute(pool)
            .await
            .map_err(|err| format!("Failed to update stale reminder list: {}", err))?;
            sqlx::query(
                "UPDATE reminder_workflow_presets
                 SET default_list_id = NULL, updated_at = ?
                 WHERE default_list_id = ?",
            )
            .bind(&now)
            .bind(&id)
            .execute(pool)
            .await
            .map_err(|err| format!("Failed to clear stale reminder preset list: {}", err))?;
        }
    }
    Ok(())
}

async fn ensure_default_list(pool: &sqlx::SqlitePool, account_id: &str) -> Result<(), String> {
    let current_default: Option<String> = sqlx::query_scalar(
        "SELECT id FROM reminder_lists
         WHERE provider_account_id = ? AND selected = 1 AND is_default = 1
         LIMIT 1",
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect default reminder list: {}", err))?;

    let default_list_id = if let Some(default_list_id) = current_default {
        default_list_id
    } else {
        let Some(default_list_id) = sqlx::query_scalar(
            "SELECT id FROM reminder_lists
             WHERE provider_account_id = ? AND selected = 1
             ORDER BY name ASC
             LIMIT 1",
        )
        .bind(account_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| format!("Failed to choose default reminder list: {}", err))?
        else {
            return Ok(());
        };
        default_list_id
    };

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE reminder_lists SET is_default = CASE WHEN id = ? THEN 1 ELSE 0 END, updated_at = ?
         WHERE provider_account_id = ?",
    )
    .bind(&default_list_id)
    .bind(&now)
    .bind(account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to persist default reminder list: {}", err))?;

    sqlx::query(
        "UPDATE reminder_provider_accounts
         SET default_list_id = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(default_list_id)
    .bind(&now)
    .bind(account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update reminder account default list: {}", err))?;
    Ok(())
}

async fn existing_account_id(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<Option<String>, String> {
    sqlx::query_scalar("SELECT id FROM reminder_provider_accounts WHERE provider = ?")
        .bind(provider)
        .fetch_optional(pool)
        .await
        .map_err(|err| format!("Failed to get reminder account: {}", err))
}

async fn existing_list_id(
    pool: &sqlx::SqlitePool,
    account_id: &str,
    provider_list_id: &str,
) -> Result<Option<String>, String> {
    sqlx::query_scalar(
        "SELECT id FROM reminder_lists WHERE provider_account_id = ? AND provider_list_id = ?",
    )
    .bind(account_id)
    .bind(provider_list_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect reminder list: {}", err))
}

async fn list_accounts(pool: &sqlx::SqlitePool) -> Result<Vec<ReminderProviderAccount>, String> {
    let rows = sqlx::query(
        "SELECT id, provider, account_label, status, default_list_id, last_sync_at,
                last_error, created_at, updated_at
         FROM reminder_provider_accounts
         ORDER BY provider ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list reminder accounts: {}", err))?;

    Ok(rows
        .into_iter()
        .map(|row| ReminderProviderAccount {
            id: row.get("id"),
            provider: row.get("provider"),
            account_label: row.get("account_label"),
            status: row.get("status"),
            default_list_id: row.get("default_list_id"),
            last_sync_at: row.get("last_sync_at"),
            last_error: row.get("last_error"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn list_lists(pool: &sqlx::SqlitePool) -> Result<Vec<ReminderList>, String> {
    let rows = sqlx::query(
        "SELECT id, provider_account_id, provider_list_id, name, color, selected,
                is_default, last_seen_at, created_at, updated_at
         FROM reminder_lists
         ORDER BY is_default DESC, name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list reminder lists: {}", err))?;

    Ok(rows
        .into_iter()
        .map(|row| ReminderList {
            id: row.get("id"),
            provider_account_id: row.get("provider_account_id"),
            provider_list_id: row.get("provider_list_id"),
            name: row.get("name"),
            color: row.get("color"),
            selected: row.get::<i64, _>("selected") != 0,
            is_default: row.get::<i64, _>("is_default") != 0,
            last_seen_at: row.get("last_seen_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn get_account(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<Option<ReminderProviderAccount>, String> {
    let row = sqlx::query(
        "SELECT id, provider, account_label, status, default_list_id, last_sync_at,
                last_error, created_at, updated_at
         FROM reminder_provider_accounts
         WHERE provider = ?",
    )
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to get reminder account: {}", err))?;

    Ok(row.map(|row| ReminderProviderAccount {
        id: row.get("id"),
        provider: row.get("provider"),
        account_label: row.get("account_label"),
        status: row.get("status"),
        default_list_id: row.get("default_list_id"),
        last_sync_at: row.get("last_sync_at"),
        last_error: row.get("last_error"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

async fn update_account_sync(
    pool: &sqlx::SqlitePool,
    provider: &str,
    status: &str,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    error: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let sync_at = completed_at.map(|value| value.to_rfc3339());
    sqlx::query(
        "UPDATE reminder_provider_accounts
         SET status = ?, last_sync_at = ?, last_error = ?, updated_at = ?
         WHERE provider = ?",
    )
    .bind(status)
    .bind(sync_at)
    .bind(error)
    .bind(&now)
    .bind(provider)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update reminder sync status: {}", err))?;
    Ok(())
}

fn default_due_preset_for_category(category: &str) -> &'static str {
    match category {
        "deploy_alert_check" => "in_2_hours",
        "pr_review" | "linear_follow_up" | "clarification_follow_up" => "tomorrow_morning",
        "docs_update" => "in_2_days",
        "experiment_revisit" => "next_week",
        _ => "none",
    }
}

fn default_priority_for_category(category: &str) -> i64 {
    match category {
        "deploy_alert_check" => 1,
        "docs_update" | "experiment_revisit" => 9,
        _ => 5,
    }
}

fn validate_reminder_category(category: &str) -> Result<(), String> {
    if REMINDER_CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err("Unknown reminder workflow category.".to_string())
    }
}

fn validate_due_preset(due_preset: &str) -> Result<(), String> {
    match due_preset {
        "none" | "in_2_hours" | "tomorrow_morning" | "in_2_days" | "next_week" => Ok(()),
        _ => Err("Unknown reminder due-date default.".to_string()),
    }
}

fn validate_priority(priority: i64) -> Result<(), String> {
    if matches!(priority, 1 | 5 | 9) {
        Ok(())
    } else {
        Err("Reminder workflow priority must be high, medium, or low.".to_string())
    }
}

async fn ensure_reminder_workflow_defaults(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO reminder_workflow_settings (provider, global_priority, created_at, updated_at)
         VALUES (?, 5, ?, ?)
         ON CONFLICT(provider) DO NOTHING",
    )
    .bind(PROVIDER_APPLE_REMINDERS)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to prepare reminder workflow settings: {}", err))?;

    for category in REMINDER_CATEGORIES {
        sqlx::query(
            "INSERT INTO reminder_workflow_presets
                (category, enabled, default_list_id, default_priority, due_preset, updated_at)
             VALUES (?, 1, NULL, ?, ?, ?)
             ON CONFLICT(category) DO NOTHING",
        )
        .bind(category)
        .bind(default_priority_for_category(category))
        .bind(default_due_preset_for_category(category))
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to prepare reminder workflow presets: {}", err))?;
    }
    Ok(())
}

async fn get_workflow_settings(
    pool: &sqlx::SqlitePool,
) -> Result<ReminderWorkflowSettings, String> {
    ensure_reminder_workflow_defaults(pool).await?;
    let row = sqlx::query(
        "SELECT provider, global_priority, created_at, updated_at
         FROM reminder_workflow_settings
         WHERE provider = ?",
    )
    .bind(PROVIDER_APPLE_REMINDERS)
    .fetch_one(pool)
    .await
    .map_err(|err| format!("Failed to load reminder workflow settings: {}", err))?;

    Ok(ReminderWorkflowSettings {
        provider: row.get("provider"),
        global_priority: row.get("global_priority"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

async fn list_workflow_presets(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<ReminderWorkflowPreset>, String> {
    ensure_reminder_workflow_defaults(pool).await?;
    let rows = sqlx::query(
        "SELECT category, enabled, default_list_id, default_priority, due_preset, updated_at
         FROM reminder_workflow_presets",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list reminder workflow presets: {}", err))?;
    let mut by_category = HashMap::new();
    for row in rows {
        let preset = ReminderWorkflowPreset {
            category: row.get("category"),
            enabled: row.get::<i64, _>("enabled") != 0,
            default_list_id: row.get("default_list_id"),
            default_priority: row.get("default_priority"),
            due_preset: row.get("due_preset"),
            updated_at: row.get("updated_at"),
        };
        by_category.insert(preset.category.clone(), preset);
    }

    Ok(REMINDER_CATEGORIES
        .iter()
        .filter_map(|category| by_category.remove(*category))
        .collect())
}

async fn update_workflow_preset(
    pool: &sqlx::SqlitePool,
    request: ReminderWorkflowPresetUpdateRequest,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(global_priority) = request.global_priority {
        validate_priority(global_priority)?;
        sqlx::query(
            "UPDATE reminder_workflow_settings
             SET global_priority = ?, updated_at = ?
             WHERE provider = ?",
        )
        .bind(global_priority)
        .bind(&now)
        .bind(PROVIDER_APPLE_REMINDERS)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to update global reminder priority: {}", err))?;
    }

    let has_preset_fields = request.enabled.is_some()
        || request.default_list_id.is_some()
        || request.use_global_list.unwrap_or(false)
        || request.default_priority.is_some()
        || request.use_global_priority.unwrap_or(false)
        || request.due_preset.is_some();
    let Some(category) = request
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        if has_preset_fields {
            return Err("Reminder workflow category is required.".to_string());
        }
        return Ok(());
    };
    validate_reminder_category(category)?;
    if let Some(priority) = request.default_priority {
        validate_priority(priority)?;
    }
    if let Some(due_preset) = request.due_preset.as_deref() {
        validate_due_preset(due_preset)?;
    }
    let list_id = request
        .default_list_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(list_id) = list_id.as_deref() {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM reminder_lists WHERE id = ? AND selected = 1")
                .bind(list_id)
                .fetch_optional(pool)
                .await
                .map_err(|err| format!("Failed to inspect reminder preset list: {}", err))?;
        if exists.is_none() {
            return Err("Selected reminder list is not available.".to_string());
        }
    }

    let current = get_workflow_preset(pool, category).await?;
    let next_list_id = if request.use_global_list.unwrap_or(false) {
        None
    } else {
        list_id.or(current.default_list_id)
    };
    let next_priority = if request.use_global_priority.unwrap_or(false) {
        None
    } else {
        request.default_priority.or(current.default_priority)
    };

    sqlx::query(
        "UPDATE reminder_workflow_presets
         SET enabled = ?, default_list_id = ?, default_priority = ?, due_preset = ?, updated_at = ?
         WHERE category = ?",
    )
    .bind(request.enabled.unwrap_or(current.enabled) as i64)
    .bind(next_list_id)
    .bind(next_priority)
    .bind(request.due_preset.unwrap_or(current.due_preset))
    .bind(now)
    .bind(category)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update reminder workflow preset: {}", err))?;
    Ok(())
}

async fn get_workflow_preset(
    pool: &sqlx::SqlitePool,
    category: &str,
) -> Result<ReminderWorkflowPreset, String> {
    let row = sqlx::query(
        "SELECT category, enabled, default_list_id, default_priority, due_preset, updated_at
         FROM reminder_workflow_presets
         WHERE category = ?",
    )
    .bind(category)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load reminder workflow preset: {}", err))?
    .ok_or_else(|| "Reminder workflow preset was not found.".to_string())?;

    Ok(ReminderWorkflowPreset {
        category: row.get("category"),
        enabled: row.get::<i64, _>("enabled") != 0,
        default_list_id: row.get("default_list_id"),
        default_priority: row.get("default_priority"),
        due_preset: row.get("due_preset"),
        updated_at: row.get("updated_at"),
    })
}

async fn generate_reminder_drafts_for_meeting(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    include_low_confidence: bool,
) -> Result<ReminderDraftGenerationResult, String> {
    let meeting = crate::database::repositories::meeting::MeetingsRepository::get_meeting_metadata(
        pool, meeting_id,
    )
    .await
    .map_err(|err| format!("Failed to load meeting metadata: {}", err))?
    .ok_or_else(|| "Meeting was not found.".to_string())?;

    let summary =
        crate::database::repositories::summary::SummaryProcessesRepository::get_summary_data(
            pool, meeting_id,
        )
        .await
        .map_err(|err| format!("Failed to load meeting summary: {}", err))?
        .ok_or_else(|| "Generate a meeting summary before creating reminder drafts.".to_string())?;

    let result = summary
        .result
        .as_deref()
        .ok_or_else(|| "Generate a meeting summary before creating reminder drafts.".to_string())?;
    let summary_value: serde_json::Value = serde_json::from_str(result)
        .map_err(|err| format!("Failed to parse meeting summary: {}", err))?;
    let summary_text = summary_value_to_text(&summary_value);
    if summary_text.trim().is_empty() {
        return Err(
            "Meeting summary does not contain enough text for reminder drafts.".to_string(),
        );
    }

    let base_time = summary.end_time.unwrap_or(meeting.updated_at.0);
    let defaults = load_workflow_defaults(pool).await?;
    let summary_id = Some(summary.meeting_id.as_str());
    let generated = build_draft_candidates(
        meeting_id,
        summary_id.as_deref(),
        &meeting.title,
        &summary_text,
        base_time,
        &defaults,
    );

    let mut hidden_low_confidence_count = 0usize;
    for draft in generated {
        if draft.confidence < 0.5 && !include_low_confidence {
            hidden_low_confidence_count += 1;
            continue;
        }
        upsert_draft(pool, &draft).await?;
    }

    let drafts = list_drafts(pool, meeting_id, include_low_confidence).await?;
    Ok(ReminderDraftGenerationResult {
        meeting_id: meeting_id.to_string(),
        drafts,
        hidden_low_confidence_count,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn summary_value_to_text(value: &serde_json::Value) -> String {
    if let Some(markdown) = value.get("markdown").and_then(|v| v.as_str()) {
        return markdown.to_string();
    }
    let mut lines = Vec::new();
    flatten_summary_value(value, &mut lines);
    lines.join("\n")
}

fn flatten_summary_value(value: &serde_json::Value, lines: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) => {
            if !text.trim().is_empty() {
                lines.push(text.trim().to_string());
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                flatten_summary_value(item, lines);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                if matches!(key.as_str(), "type" | "id" | "props") {
                    continue;
                }
                flatten_summary_value(item, lines);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone)]
struct DraftCandidate {
    title: String,
    due_at: Option<String>,
    priority: Option<i64>,
    list_id: Option<String>,
    category: String,
    confidence: f64,
    source_evidence: Vec<ReminderSourceEvidence>,
    preset_reason: String,
    dedupe_key: String,
}

#[derive(Debug, Clone)]
struct ReminderWorkflowDefaults {
    global_priority: i64,
    default_list_id: Option<String>,
    presets: HashMap<String, ReminderWorkflowPreset>,
}

fn build_draft_candidates(
    meeting_id: &str,
    summary_id: Option<&str>,
    meeting_title: &str,
    summary_text: &str,
    base_time: chrono::DateTime<chrono::Utc>,
    defaults: &ReminderWorkflowDefaults,
) -> Vec<ReminderDraft> {
    let mut seen = HashSet::new();
    extract_action_candidates(summary_text)
        .into_iter()
        .filter_map(|candidate| {
            let candidate =
                candidate_to_draft_candidate(meeting_id, &candidate, base_time, defaults)?;
            if !seen.insert(candidate.dedupe_key.clone()) {
                return None;
            }
            Some(candidate_to_reminder_draft(
                meeting_id,
                summary_id,
                meeting_title,
                candidate,
            ))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ActionCandidate {
    text: String,
    due_hint: Option<String>,
    evidence_label: String,
    in_action_section: bool,
    from_table: bool,
}

fn extract_action_candidates(summary_text: &str) -> Vec<ActionCandidate> {
    let mut candidates = Vec::new();
    let mut in_action_section = false;
    for raw_line in summary_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            let heading = line.trim_start_matches('#').trim().to_lowercase();
            in_action_section = heading.contains("action")
                || heading.contains("follow-up")
                || heading.contains("follow up")
                || heading.contains("next step");
            continue;
        }
        if line.contains('|') {
            if let Some((task, due)) = parse_action_table_row(line) {
                candidates.push(ActionCandidate {
                    text: task,
                    due_hint: due,
                    evidence_label: "Action item table".to_string(),
                    in_action_section,
                    from_table: true,
                });
            }
            continue;
        }
        let clean = clean_action_line(line);
        if clean.len() < 8 {
            continue;
        }
        if in_action_section || looks_actionable(&clean) {
            candidates.push(ActionCandidate {
                text: clean,
                due_hint: None,
                evidence_label: if in_action_section {
                    "Action items".to_string()
                } else {
                    "Summary".to_string()
                },
                in_action_section,
                from_table: false,
            });
        }
    }
    candidates
}

fn parse_action_table_row(line: &str) -> Option<(String, Option<String>)> {
    let cells: Vec<String> = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().trim_matches('"').to_string())
        .collect();
    if cells.len() < 2
        || cells.iter().all(|cell| {
            cell.chars()
                .all(|ch| ch == '-' || ch == ':' || ch.is_whitespace())
        })
    {
        return None;
    }
    let lowered = cells
        .iter()
        .map(|cell| cell.to_lowercase())
        .collect::<Vec<_>>();
    if lowered
        .iter()
        .any(|cell| cell == "task" || cell == "owner" || cell == "due")
    {
        return None;
    }
    let task = cells
        .get(1)
        .filter(|cell| cell.len() > 6)
        .cloned()
        .or_else(|| cells.iter().max_by_key(|cell| cell.len()).cloned())?;
    let due = cells
        .get(2)
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty() && !cell.eq_ignore_ascii_case("tbd"))
        .map(str::to_string);
    Some((clean_action_line(&task), due))
}

fn clean_action_line(line: &str) -> String {
    line.trim()
        .trim_start_matches(|ch: char| {
            ch == '-' || ch == '*' || ch == '•' || ch.is_ascii_digit() || ch == '.' || ch == ')'
        })
        .trim()
        .trim_matches('"')
        .trim()
        .to_string()
}

fn looks_actionable(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "need to",
        "needs to",
        "todo",
        "follow up",
        "will ",
        "should ",
        "review",
        "check",
        "update",
        "implement",
        "deploy",
        "ask ",
        "confirm",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn candidate_to_draft_candidate(
    meeting_id: &str,
    candidate: &ActionCandidate,
    base_time: chrono::DateTime<chrono::Utc>,
    defaults: &ReminderWorkflowDefaults,
) -> Option<DraftCandidate> {
    let title = normalize_title(&candidate.text);
    if title.len() < 8 || is_noise(&title) {
        return None;
    }
    let combined_due_text = candidate
        .due_hint
        .as_ref()
        .map(|due| format!("{} {}", title, due))
        .unwrap_or_else(|| title.clone());
    let category = categorize_action(&title);
    let preset = defaults.presets.get(&category);
    if preset.is_some_and(|preset| !preset.enabled) {
        return None;
    }
    let due_at = infer_due_at(&combined_due_text, base_time).or_else(|| {
        due_at_for_preset(
            preset
                .map(|preset| preset.due_preset.as_str())
                .unwrap_or_else(|| default_due_preset_for_category(&category)),
            base_time,
        )
    });
    let priority = Some(
        preset
            .and_then(|preset| preset.default_priority)
            .unwrap_or(defaults.global_priority),
    );
    let list_id = preset
        .and_then(|preset| preset.default_list_id.clone())
        .or_else(|| defaults.default_list_id.clone());
    let confidence = confidence_for_candidate(candidate, &category);
    let evidence = vec![ReminderSourceEvidence {
        label: candidate.evidence_label.clone(),
        snippet: truncate_evidence(&candidate.text),
    }];
    let preset_reason = preset_reason(&category, due_at.as_deref(), priority, list_id.as_deref());
    let due_bucket = due_at.as_deref().unwrap_or("undated");
    let dedupe_key = build_dedupe_key(meeting_id, &title, &category, due_bucket);
    Some(DraftCandidate {
        title,
        due_at,
        priority,
        list_id,
        category,
        confidence,
        source_evidence: evidence,
        preset_reason,
        dedupe_key,
    })
}

fn normalize_title(text: &str) -> String {
    let mut title = text
        .replace("**", "")
        .replace("__", "")
        .trim()
        .trim_end_matches('.')
        .to_string();
    if let Some((_, task)) = title.split_once(':') {
        if task.trim().len() > 8 {
            title = task.trim().to_string();
        }
    }
    title
}

fn is_noise(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.contains("no action")
        || lower.contains("no follow")
        || lower.contains("nothing to do")
        || lower.contains("not required")
}

fn categorize_action(title: &str) -> String {
    let lower = title.to_lowercase();
    if any_contains(&lower, &["pull request", " pr ", "merge", "ci"])
        || (lower.contains("review") && any_contains(&lower, &["pr", "pull request", "ci"]))
    {
        "pr_review"
    } else if any_contains(
        &lower,
        &["linear", "jira", "ticket", "issue", "acceptance criteria"],
    ) {
        "linear_follow_up"
    } else if any_contains(
        &lower,
        &[
            "deploy",
            "production",
            "alert",
            "observability",
            "log",
            "monitor",
        ],
    ) {
        "deploy_alert_check"
    } else if any_contains(
        &lower,
        &["doc", "readme", "confluence", "release note", "write up"],
    ) {
        "docs_update"
    } else if any_contains(
        &lower,
        &["experiment", "revisit", "metric", "benchmark", "spike"],
    ) {
        "experiment_revisit"
    } else if any_contains(
        &lower,
        &["ask", "confirm", "clarify", "follow up", "message", "email"],
    ) {
        "clarification_follow_up"
    } else {
        "implementation_task"
    }
    .to_string()
}

fn any_contains(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn infer_due_at(text: &str, base_time: chrono::DateTime<chrono::Utc>) -> Option<String> {
    use chrono::{Datelike, Duration, TimeZone};
    let lower = text.to_lowercase();
    if lower.contains("in a few hours") || lower.contains("few hours") {
        return Some((base_time + Duration::hours(3)).to_rfc3339());
    }
    if lower.contains("tomorrow") {
        let date = (base_time + Duration::days(1)).date_naive();
        return chrono::Utc
            .with_ymd_and_hms(date.year(), date.month(), date.day(), 9, 0, 0)
            .single()
            .map(|dt| dt.to_rfc3339());
    }
    if lower.contains("today") || lower.contains("end of day") {
        let date = base_time.date_naive();
        return chrono::Utc
            .with_ymd_and_hms(date.year(), date.month(), date.day(), 17, 0, 0)
            .single()
            .map(|dt| dt.to_rfc3339());
    }
    if lower.contains("next week") || lower.contains("one week") {
        return Some((base_time + Duration::days(7)).to_rfc3339());
    }
    if lower.contains("few days") || lower.contains("2 days") || lower.contains("two days") {
        return Some((base_time + Duration::days(2)).to_rfc3339());
    }
    None
}

fn due_at_for_preset(due_preset: &str, base_time: chrono::DateTime<chrono::Utc>) -> Option<String> {
    use chrono::{Datelike, Duration, TimeZone};
    match due_preset {
        "in_2_hours" => Some((base_time + Duration::hours(2)).to_rfc3339()),
        "tomorrow_morning" => {
            let date = (base_time + Duration::days(1)).date_naive();
            chrono::Utc
                .with_ymd_and_hms(date.year(), date.month(), date.day(), 9, 0, 0)
                .single()
                .map(|dt| dt.to_rfc3339())
        }
        "in_2_days" => Some((base_time + Duration::days(2)).to_rfc3339()),
        "next_week" => Some((base_time + Duration::days(7)).to_rfc3339()),
        _ => None,
    }
}

fn preset_reason(
    category: &str,
    due_at: Option<&str>,
    priority: Option<i64>,
    list_id: Option<&str>,
) -> String {
    let mut parts = vec![format!("Category: {}", category.replace('_', " "))];
    if let Some(due_at) = due_at {
        parts.push(format!("Due default: {}", due_at));
    }
    if let Some(priority) = priority {
        parts.push(format!("Priority default: {}", priority));
    }
    if list_id.is_some() {
        parts.push("List default applied".to_string());
    } else {
        parts.push("Uses global list default".to_string());
    }
    parts.join(". ")
}

fn confidence_for_candidate(candidate: &ActionCandidate, category: &str) -> f64 {
    let mut confidence = if candidate.from_table {
        0.82
    } else if candidate.in_action_section {
        0.72
    } else {
        0.58
    };
    if category == "implementation_task" && !looks_actionable(&candidate.text) {
        confidence -= 0.18;
    }
    confidence
}

fn truncate_evidence(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() <= 280 {
        return text.to_string();
    }
    format!("{}...", text.chars().take(277).collect::<String>())
}

fn build_dedupe_key(meeting_id: &str, title: &str, category: &str, due_bucket: &str) -> String {
    let normalized_title = title
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let input = format!(
        "{}|{}|{}|{}",
        meeting_id, category, normalized_title, due_bucket
    );
    format!("{:016x}", fnv1a64(&input))
}

fn fnv1a64(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn candidate_to_reminder_draft(
    meeting_id: &str,
    summary_id: Option<&str>,
    meeting_title: &str,
    candidate: DraftCandidate,
) -> ReminderDraft {
    let now = chrono::Utc::now().to_rfc3339();
    let notes = Some(format!(
        "From Meetily meeting: {}\nWhy: {}\nPreset: {}",
        meeting_title,
        candidate
            .source_evidence
            .first()
            .map(|evidence| evidence.snippet.as_str())
            .unwrap_or(candidate.title.as_str()),
        candidate.preset_reason
    ));
    ReminderDraft {
        id: Uuid::new_v4().to_string(),
        meeting_id: meeting_id.to_string(),
        summary_id: summary_id.map(str::to_string),
        title: candidate.title,
        notes,
        due_at: candidate.due_at,
        priority: candidate.priority,
        list_id: candidate.list_id,
        category: candidate.category,
        confidence: candidate.confidence,
        source_evidence: candidate.source_evidence,
        dedupe_key: candidate.dedupe_key,
        status: if candidate.confidence < 0.5 {
            "low_confidence".to_string()
        } else {
            "suggested".to_string()
        },
        created_at: now.clone(),
        updated_at: now,
    }
}

async fn load_workflow_defaults(
    pool: &sqlx::SqlitePool,
) -> Result<ReminderWorkflowDefaults, String> {
    ensure_reminder_workflow_defaults(pool).await?;
    let workflow_settings = get_workflow_settings(pool).await?;
    let selected_list_ids = selected_reminder_list_ids(pool).await?;
    let presets = list_workflow_presets(pool)
        .await?
        .into_iter()
        .map(|mut preset| {
            if preset
                .default_list_id
                .as_ref()
                .is_some_and(|list_id| !selected_list_ids.contains(list_id))
            {
                preset.default_list_id = None;
            }
            (preset.category.clone(), preset)
        })
        .collect();
    let default_list_id: Option<String> = sqlx::query_scalar(
        "SELECT default_list_id FROM reminder_provider_accounts
         WHERE provider = ? AND status = 'connected'
         LIMIT 1",
    )
    .bind(PROVIDER_APPLE_REMINDERS)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect default reminder list: {}", err))?;

    Ok(ReminderWorkflowDefaults {
        global_priority: workflow_settings.global_priority,
        default_list_id,
        presets,
    })
}

async fn selected_reminder_list_ids(pool: &sqlx::SqlitePool) -> Result<HashSet<String>, String> {
    let rows = sqlx::query("SELECT id FROM reminder_lists WHERE selected = 1")
        .fetch_all(pool)
        .await
        .map_err(|err| format!("Failed to inspect selected reminder lists: {}", err))?;
    Ok(rows.into_iter().map(|row| row.get("id")).collect())
}

async fn upsert_draft(pool: &sqlx::SqlitePool, draft: &ReminderDraft) -> Result<(), String> {
    let evidence_json = serde_json::to_string(&draft.source_evidence)
        .map_err(|err| format!("Failed to serialize reminder evidence: {}", err))?;
    sqlx::query(
        "INSERT INTO reminder_drafts
            (id, meeting_id, summary_id, title, notes, due_at, priority, list_id, category, confidence,
             source_evidence_json, dedupe_key, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(meeting_id, dedupe_key) DO UPDATE SET
            summary_id = excluded.summary_id,
            title = excluded.title,
            notes = excluded.notes,
            due_at = excluded.due_at,
            priority = excluded.priority,
            list_id = COALESCE(reminder_drafts.list_id, excluded.list_id),
            category = excluded.category,
            confidence = excluded.confidence,
            source_evidence_json = excluded.source_evidence_json,
            status = CASE
                WHEN reminder_drafts.status IN ('created', 'dismissed') THEN reminder_drafts.status
                ELSE excluded.status
            END,
            updated_at = excluded.updated_at",
    )
    .bind(&draft.id)
    .bind(&draft.meeting_id)
    .bind(&draft.summary_id)
    .bind(&draft.title)
    .bind(&draft.notes)
    .bind(&draft.due_at)
    .bind(draft.priority)
    .bind(&draft.list_id)
    .bind(&draft.category)
    .bind(draft.confidence)
    .bind(evidence_json)
    .bind(&draft.dedupe_key)
    .bind(&draft.status)
    .bind(&draft.created_at)
    .bind(&draft.updated_at)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save reminder draft: {}", err))?;
    Ok(())
}

async fn list_drafts(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    include_low_confidence: bool,
) -> Result<Vec<ReminderDraft>, String> {
    let mut query = "SELECT id, meeting_id, summary_id, title, notes, due_at, priority, list_id,
            category, confidence, source_evidence_json, dedupe_key, status, created_at, updated_at
         FROM reminder_drafts
         WHERE meeting_id = ?"
        .to_string();
    // Created and dismissed drafts move out of the review flow. Low-confidence drafts are
    // separately controlled because users can explicitly ask to inspect them.
    query.push_str(" AND status NOT IN ('created', 'dismissed')");
    if !include_low_confidence {
        query.push_str(" AND confidence >= 0.5 AND status != 'low_confidence'");
    }
    query.push_str(" ORDER BY confidence DESC, created_at ASC");
    let rows = sqlx::query(&query)
        .bind(meeting_id)
        .fetch_all(pool)
        .await
        .map_err(|err| format!("Failed to list reminder drafts: {}", err))?;
    rows.into_iter()
        .map(|row| reminder_draft_from_row(&row))
        .collect()
}

async fn update_draft(
    pool: &sqlx::SqlitePool,
    request: ReminderDraftUpdateRequest,
) -> Result<ReminderDraft, String> {
    let title = request.title.trim();
    if title.chars().count() < 3 {
        return Err("Reminder title is required.".to_string());
    }
    if let Some(priority) = request.priority {
        if !(1..=9).contains(&priority) {
            return Err("Reminder priority must be between 1 and 9.".to_string());
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let notes = request
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let due_at = request
        .due_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let list_id = request
        .list_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let result = sqlx::query(
        "UPDATE reminder_drafts
         SET title = ?, notes = ?, due_at = ?, priority = ?, list_id = ?, updated_at = ?
         WHERE id = ? AND status NOT IN ('created', 'dismissed')",
    )
    .bind(title)
    .bind(notes)
    .bind(due_at)
    .bind(request.priority)
    .bind(list_id)
    .bind(now)
    .bind(&request.draft_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update reminder draft: {}", err))?;
    if result.rows_affected() == 0 {
        return Err("Reminder draft could not be edited.".to_string());
    }

    get_draft_by_id(pool, &request.draft_id).await
}

async fn set_draft_status(
    pool: &sqlx::SqlitePool,
    draft_id: &str,
    status: &str,
) -> Result<ReminderDraft, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query("UPDATE reminder_drafts SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(now)
        .bind(draft_id)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to update reminder draft status: {}", err))?;
    if result.rows_affected() == 0 {
        return Err("Reminder draft was not found.".to_string());
    }

    get_draft_by_id(pool, draft_id).await
}

async fn get_draft_by_id(pool: &sqlx::SqlitePool, draft_id: &str) -> Result<ReminderDraft, String> {
    let row = sqlx::query(
        "SELECT id, meeting_id, summary_id, title, notes, due_at, priority, list_id,
            category, confidence, source_evidence_json, dedupe_key, status, created_at, updated_at
         FROM reminder_drafts
         WHERE id = ?",
    )
    .bind(draft_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load reminder draft: {}", err))?
    .ok_or_else(|| "Reminder draft was not found.".to_string())?;

    reminder_draft_from_row(&row)
}

async fn create_reminders_for_drafts(
    pool: &sqlx::SqlitePool,
    request: CreateReminderRequest,
) -> Result<CreateReminderResult, String> {
    if request.draft_ids.is_empty() {
        return Err("Select at least one reminder draft.".to_string());
    }
    if !cfg!(target_os = "macos") {
        return Err("Apple Reminders is available only on macOS.".to_string());
    }

    let account = get_account(pool, PROVIDER_APPLE_REMINDERS)
        .await?
        .ok_or_else(|| "Connect Apple Reminders before creating reminders.".to_string())?;
    if account.status != "connected" {
        return Err(
            "Apple Reminders permission is required before creating reminders.".to_string(),
        );
    }

    let meeting = crate::database::repositories::meeting::MeetingsRepository::get_meeting_metadata(
        pool,
        &request.meeting_id,
    )
    .await
    .map_err(|err| format!("Failed to load meeting metadata: {}", err))?
    .ok_or_else(|| "Meeting was not found.".to_string())?;

    let mut created = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for draft_id in request.draft_ids {
        let draft = match get_draft_by_id(pool, &draft_id).await {
            Ok(draft) => draft,
            Err(error) => {
                failed.push(ReminderCreationFailure {
                    draft_id,
                    title: "Unknown reminder".to_string(),
                    error,
                });
                continue;
            }
        };

        if draft.meeting_id != request.meeting_id {
            failed.push(ReminderCreationFailure {
                draft_id: draft.id,
                title: draft.title,
                error: "Reminder draft does not belong to this meeting.".to_string(),
            });
            continue;
        }

        if let Some(existing) =
            get_created_link_by_dedupe(pool, &draft.meeting_id, &draft.dedupe_key).await?
        {
            skipped.push(existing);
            continue;
        }

        let list_id = draft
            .list_id
            .as_deref()
            .or(account.default_list_id.as_deref())
            .ok_or_else(|| {
                "Choose a default Apple Reminders list before creating reminders.".to_string()
            })?;
        let list = get_list_by_id(pool, list_id).await?;
        let notes = reminder_notes(&meeting.title, &meeting.id, draft.notes.as_deref());

        match create_apple_reminder(
            &list.provider_list_id,
            &draft.title,
            &notes,
            draft.due_at.as_deref(),
            draft.priority,
        ) {
            Ok(provider_reminder_id) => {
                match save_created_link(pool, &draft, &list.id, &provider_reminder_id).await {
                    Ok(link) => {
                        if let Err(error) = set_draft_status(pool, &draft.id, "created").await {
                            log::warn!("Failed to mark reminder draft as created: {}", error);
                        }
                        created.push(link);
                    }
                    Err(error) => failed.push(ReminderCreationFailure {
                        draft_id: draft.id,
                        title: draft.title,
                        error,
                    }),
                }
            }
            Err(error) => failed.push(ReminderCreationFailure {
                draft_id: draft.id,
                title: draft.title,
                error,
            }),
        }
    }

    Ok(CreateReminderResult {
        meeting_id: request.meeting_id,
        created,
        skipped,
        failed,
    })
}

fn reminder_notes(meeting_title: &str, meeting_id: &str, draft_notes: Option<&str>) -> String {
    let mut lines = Vec::new();
    if let Some(notes) = draft_notes.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(notes.to_string());
    }
    lines.push(format!("Source: Meetily meeting \"{}\"", meeting_title));
    lines.push(format!("Meeting ID: {}", meeting_id));
    lines.join("\n\n")
}

fn create_apple_reminder(
    provider_list_id: &str,
    title: &str,
    notes: &str,
    due_at: Option<&str>,
    priority: Option<i64>,
) -> Result<String, String> {
    let script = apple_reminder_create_script();
    let due_text = match due_at.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => apple_script_due_date(value).ok_or_else(|| {
            "Reminder due date could not be converted for Apple Reminders.".to_string()
        })?,
        None => String::new(),
    };
    let priority_text = priority.map(|value| value.to_string()).unwrap_or_default();
    let output = Command::new("osascript")
        .args([
            "-e",
            &script,
            "--",
            provider_list_id,
            title,
            notes,
            &due_text,
            &priority_text,
        ])
        .output()
        .map_err(|err| format!("Failed to run Apple Reminders creation: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Apple Reminders could not create this reminder.".to_string()
        } else {
            stderr
        });
    }

    let reminder_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if reminder_id.is_empty() {
        return Err("Apple Reminders did not return a reminder id.".to_string());
    }
    Ok(reminder_id)
}

fn apple_script_due_date(value: &str) -> Option<String> {
    let date = chrono::DateTime::parse_from_rfc3339(value).ok()?;
    Some(
        date.with_timezone(&chrono::Local)
            .format("%A, %B %-d, %Y at %-I:%M:%S %p")
            .to_string(),
    )
}

fn apple_reminder_create_script() -> String {
    r#"on run argv
    set targetListId to item 1 of argv
    set reminderTitle to item 2 of argv
    set reminderNotes to item 3 of argv
    set dueText to item 4 of argv
    set priorityText to item 5 of argv
    tell application id "com.apple.reminders"
        set targetList to missing value
        repeat with reminderList in lists
            try
                set candidateId to id of reminderList as text
                if candidateId is targetListId then
                    set targetList to reminderList
                    exit repeat
                end if
            end try
        end repeat
        if targetList is missing value then error "Apple Reminders list was not found."
        set newReminder to make new reminder at end of reminders of targetList with properties {name:reminderTitle, body:reminderNotes}
        if priorityText is not "" then
            try
                set priority of newReminder to priorityText as integer
            end try
        end if
        if dueText is not "" then
            try
                set due date of newReminder to date dueText
            end try
        end if
        return id of newReminder as text
    end tell
end run
"#
    .to_string()
}

async fn save_created_link(
    pool: &sqlx::SqlitePool,
    draft: &ReminderDraft,
    list_id: &str,
    provider_reminder_id: &str,
) -> Result<CreatedReminderLink, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO reminder_created_links
            (id, meeting_id, draft_id, dedupe_key, provider, provider_reminder_id, list_id, title, status, created_at, updated_at, last_error)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'open', ?, ?, NULL)
         ON CONFLICT(meeting_id, dedupe_key) DO UPDATE SET
            updated_at = reminder_created_links.updated_at",
    )
    .bind(&id)
    .bind(&draft.meeting_id)
    .bind(&draft.id)
    .bind(&draft.dedupe_key)
    .bind(PROVIDER_APPLE_REMINDERS)
    .bind(provider_reminder_id)
    .bind(list_id)
    .bind(&draft.title)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save created reminder link: {}", err))?;

    get_created_link_by_dedupe(pool, &draft.meeting_id, &draft.dedupe_key)
        .await?
        .ok_or_else(|| "Created reminder link was not saved.".to_string())
}

async fn get_created_link_by_dedupe(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    dedupe_key: &str,
) -> Result<Option<CreatedReminderLink>, String> {
    let row = sqlx::query(
        "SELECT r.id, r.meeting_id, m.title AS meeting_title, r.draft_id, r.dedupe_key,
                r.provider, r.provider_reminder_id, r.list_id, l.name AS list_name,
                r.title, d.due_at AS due_at, r.status, r.created_at, r.updated_at, r.last_error
         FROM reminder_created_links r
         LEFT JOIN reminder_drafts d ON d.id = r.draft_id
         LEFT JOIN reminder_lists l ON l.id = r.list_id
         LEFT JOIN meetings m ON m.id = r.meeting_id
         WHERE r.meeting_id = ? AND r.dedupe_key = ?",
    )
    .bind(meeting_id)
    .bind(dedupe_key)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect created reminders: {}", err))?;

    row.map(|row| created_link_from_row(&row)).transpose()
}

async fn list_created_links(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Vec<CreatedReminderLink>, String> {
    let rows = sqlx::query(
        "SELECT r.id, r.meeting_id, m.title AS meeting_title, r.draft_id, r.dedupe_key,
                r.provider, r.provider_reminder_id, r.list_id, l.name AS list_name,
                r.title, d.due_at AS due_at, r.status, r.created_at, r.updated_at, r.last_error
         FROM reminder_created_links r
         LEFT JOIN reminder_drafts d ON d.id = r.draft_id
         LEFT JOIN reminder_lists l ON l.id = r.list_id
         LEFT JOIN meetings m ON m.id = r.meeting_id
         WHERE r.meeting_id = ?
         ORDER BY r.created_at DESC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list created reminders: {}", err))?;

    rows.into_iter()
        .map(|row| created_link_from_row(&row))
        .collect()
}

async fn list_recent_created_links(
    pool: &sqlx::SqlitePool,
    limit: i64,
) -> Result<Vec<CreatedReminderLink>, String> {
    let rows = sqlx::query(
        "SELECT r.id, r.meeting_id, m.title AS meeting_title, r.draft_id, r.dedupe_key,
                r.provider, r.provider_reminder_id, r.list_id, l.name AS list_name,
                r.title, d.due_at AS due_at, r.status, r.created_at, r.updated_at, r.last_error
         FROM reminder_created_links r
         LEFT JOIN reminder_drafts d ON d.id = r.draft_id
         LEFT JOIN reminder_lists l ON l.id = r.list_id
         LEFT JOIN meetings m ON m.id = r.meeting_id
         ORDER BY r.created_at DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list reminder follow-up history: {}", err))?;

    rows.into_iter()
        .map(|row| created_link_from_row(&row))
        .collect()
}

async fn refresh_created_link_statuses(
    pool: &sqlx::SqlitePool,
    meeting_id: Option<&str>,
) -> Result<(), String> {
    let account = get_account(pool, PROVIDER_APPLE_REMINDERS).await?;
    if !cfg!(target_os = "macos")
        || !matches!(
            account.as_ref().map(|a| a.status.as_str()),
            Some("connected")
        )
    {
        let now = chrono::Utc::now().to_rfc3339();
        let query = if meeting_id.is_some() {
            "UPDATE reminder_created_links SET status = 'unavailable', updated_at = ? WHERE meeting_id = ?"
        } else {
            "UPDATE reminder_created_links SET status = 'unavailable', updated_at = ?"
        };
        let mut statement = sqlx::query(query).bind(&now);
        if let Some(meeting_id) = meeting_id {
            statement = statement.bind(meeting_id);
        }
        statement
            .execute(pool)
            .await
            .map_err(|err| format!("Failed to mark reminder statuses unavailable: {}", err))?;
        return Ok(());
    }

    let rows = if let Some(meeting_id) = meeting_id {
        sqlx::query("SELECT id, provider_reminder_id FROM reminder_created_links WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_all(pool)
            .await
    } else {
        sqlx::query("SELECT id, provider_reminder_id FROM reminder_created_links ORDER BY created_at DESC LIMIT 50")
            .fetch_all(pool)
            .await
    }
    .map_err(|err| format!("Failed to load reminders for status refresh: {}", err))?;

    for row in rows {
        let id: String = row.get("id");
        let provider_reminder_id: String = row.get("provider_reminder_id");
        let (status, error) = match read_apple_reminder_status(&provider_reminder_id) {
            Ok(status) => (status, None),
            Err(error) => (
                "unavailable".to_string(),
                Some(sanitize_reminder_error(&error)),
            ),
        };
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE reminder_created_links
             SET status = ?, last_error = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(status)
        .bind(error)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to save reminder status: {}", err))?;
    }
    Ok(())
}

fn read_apple_reminder_status(provider_reminder_id: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            &apple_reminder_status_script(),
            "--",
            provider_reminder_id,
        ])
        .output()
        .map_err(|err| format!("Failed to read Apple Reminders status: {}", err))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Apple Reminders status is unavailable.".to_string()
        } else {
            stderr
        });
    }
    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match status.as_str() {
        "open" | "completed" | "missing" => Ok(status),
        _ => Ok("unavailable".to_string()),
    }
}

fn apple_reminder_status_script() -> String {
    r#"on run argv
    set targetReminderId to item 1 of argv
    tell application id "com.apple.reminders"
        try
            set targetReminder to reminder id targetReminderId
        on error
            return "missing"
        end try
        try
            if completed of targetReminder then
                return "completed"
            end if
            return "open"
        on error
            return "unavailable"
        end try
    end tell
end run
"#
    .to_string()
}

async fn get_list_by_id(pool: &sqlx::SqlitePool, list_id: &str) -> Result<ReminderList, String> {
    let row = sqlx::query(
        "SELECT id, provider_account_id, provider_list_id, name, color, selected,
                is_default, last_seen_at, created_at, updated_at
         FROM reminder_lists
         WHERE id = ?",
    )
    .bind(list_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load reminder list: {}", err))?
    .ok_or_else(|| "Reminder list was not found.".to_string())?;
    let selected = row.get::<i64, _>("selected") != 0;
    if !selected {
        return Err(
            "This Apple Reminders list is no longer enabled. Pick a different list and try again."
                .to_string(),
        );
    }

    Ok(ReminderList {
        id: row.get("id"),
        provider_account_id: row.get("provider_account_id"),
        provider_list_id: row.get("provider_list_id"),
        name: row.get("name"),
        color: row.get("color"),
        selected,
        is_default: row.get::<i64, _>("is_default") != 0,
        last_seen_at: row.get("last_seen_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn created_link_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<CreatedReminderLink, String> {
    Ok(CreatedReminderLink {
        id: row.get("id"),
        meeting_id: row.get("meeting_id"),
        meeting_title: row.try_get("meeting_title").ok(),
        draft_id: row.get("draft_id"),
        dedupe_key: row.get("dedupe_key"),
        provider: row.get("provider"),
        provider_reminder_id: row.get("provider_reminder_id"),
        list_id: row.get("list_id"),
        list_name: row.try_get("list_name").ok(),
        title: row.get("title"),
        due_at: row.try_get("due_at").ok(),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        last_error: row.get("last_error"),
    })
}

fn reminder_draft_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ReminderDraft, String> {
    let evidence_json: String = row.get("source_evidence_json");
    let source_evidence = serde_json::from_str::<Vec<ReminderSourceEvidence>>(&evidence_json)
        .map_err(|err| format!("Failed to parse reminder evidence: {}", err))?;
    Ok(ReminderDraft {
        id: row.get("id"),
        meeting_id: row.get("meeting_id"),
        summary_id: row.get("summary_id"),
        title: row.get("title"),
        notes: row.get("notes"),
        due_at: row.get("due_at"),
        priority: row.get("priority"),
        list_id: row.get("list_id"),
        category: row.get("category"),
        confidence: row.get("confidence"),
        source_evidence,
        dedupe_key: row.get("dedupe_key"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn normalize_provider(provider: &str) -> Result<String, String> {
    match provider.trim().to_lowercase().replace('-', "_").as_str() {
        "apple" | "apple_reminders" | "reminders" => Ok(PROVIDER_APPLE_REMINDERS.to_string()),
        _ => Err("Unsupported reminder provider".to_string()),
    }
}

fn provider_label(provider: &str) -> &'static str {
    match provider {
        PROVIDER_APPLE_REMINDERS => APPLE_REMINDERS_LABEL,
        _ => "Reminder Provider",
    }
}

fn sync_error_status(error: &str) -> &'static str {
    let lower = error.to_lowercase();
    if lower.contains("permission")
        || lower.contains("not authorized")
        || lower.contains("not authorised")
        || lower.contains("-1743")
    {
        "permission_needed"
    } else {
        "error"
    }
}

fn sanitize_reminder_error(error: &str) -> String {
    let lower = error.to_lowercase();
    if sync_error_status(error) == "permission_needed"
        || lower.contains("not authorized")
        || lower.contains("not authorised")
        || lower.contains("not permitted")
        || lower.contains("permission")
    {
        return "Apple Reminders permission is required. Allow Meetily in macOS Privacy & Security settings, then refresh lists again.".to_string();
    }
    if lower.contains("not available") {
        return "Apple Reminders is available only on macOS.".to_string();
    }
    "Apple Reminders lists could not be refreshed. Check app permissions and try again.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_defaults() -> ReminderWorkflowDefaults {
        let now = "2026-06-19T10:00:00Z".to_string();
        let presets = REMINDER_CATEGORIES
            .iter()
            .map(|category| {
                (
                    (*category).to_string(),
                    ReminderWorkflowPreset {
                        category: (*category).to_string(),
                        enabled: true,
                        default_list_id: None,
                        default_priority: Some(default_priority_for_category(category)),
                        due_preset: default_due_preset_for_category(category).to_string(),
                        updated_at: now.clone(),
                    },
                )
            })
            .collect();
        ReminderWorkflowDefaults {
            global_priority: 5,
            default_list_id: Some("global-list".to_string()),
            presets,
        }
    }

    #[test]
    fn parses_apple_reminder_list_row_without_content() {
        let row = format!("{}{}{}", "x-apple-list-id", '\u{1e}', "Engineering");
        let parsed = parse_apple_reminder_list_row(&row).unwrap().unwrap();
        assert_eq!(parsed.provider_list_id, "x-apple-list-id");
        assert_eq!(parsed.name, "Engineering");
    }

    #[test]
    fn ignores_empty_apple_reminder_list_rows() {
        let row = format!("{}{}{}", "", '\u{1e}', "");
        assert!(parse_apple_reminder_list_row(&row).unwrap().is_none());
    }

    #[test]
    fn apple_reminder_lists_script_reads_lists_not_reminders() {
        let script = apple_reminder_lists_script();
        assert!(script.contains("application id \"com.apple.reminders\""));
        assert!(script.contains("repeat with reminderList in lists"));
        assert!(!script.contains("every reminder"));
        assert!(!script.contains("body of"));
    }

    #[test]
    fn apple_reminder_create_script_writes_only_selected_list() {
        let script = apple_reminder_create_script();
        assert!(script.contains("make new reminder"));
        assert!(script.contains("targetListId"));
        assert!(script.contains("body:reminderNotes"));
        assert!(!script.contains("every reminder"));
    }

    #[test]
    fn reminder_notes_include_meeting_backlink_context() {
        let notes = reminder_notes("API review", "meeting-123", Some("Check CI after deploy"));
        assert!(notes.contains("Check CI after deploy"));
        assert!(notes.contains("Source: Meetily meeting \"API review\""));
        assert!(notes.contains("Meeting ID: meeting-123"));
    }

    #[test]
    fn apple_script_due_date_formats_rfc3339_values() {
        let formatted = apple_script_due_date("2026-06-20T09:30:00Z").unwrap();
        assert!(formatted.contains("2026"));
        assert!(formatted.contains(":30:00"));
    }

    #[test]
    fn apple_script_due_date_rejects_unparseable_values() {
        assert!(apple_script_due_date("tomorrow morning").is_none());
    }

    #[test]
    fn normalizes_apple_reminders_provider_aliases() {
        assert_eq!(
            normalize_provider("apple").unwrap(),
            PROVIDER_APPLE_REMINDERS
        );
        assert_eq!(
            normalize_provider("reminders").unwrap(),
            PROVIDER_APPLE_REMINDERS
        );
        assert_eq!(
            normalize_provider("apple-reminders").unwrap(),
            PROVIDER_APPLE_REMINDERS
        );
    }

    #[test]
    fn maps_reminders_permission_errors_to_user_safe_copy() {
        let error = "execution error: Not authorized to send Apple events to Reminders. (-1743)";
        assert_eq!(sync_error_status(error), "permission_needed");
        assert!(sanitize_reminder_error(error).contains("Apple Reminders permission is required"));
        assert!(!sanitize_reminder_error(error).contains("-1743"));
    }

    #[test]
    fn rejects_unknown_reminder_provider() {
        assert!(normalize_provider("todoist").is_err());
    }

    #[test]
    fn extracts_action_table_reminder_drafts() {
        let summary = r#"
# Summary
Work is proceeding.

## Action Items
| Owner | Task | Due | Reference |
| --- | --- | --- | --- |
| Adrian | Complete observability setup and alerting for Connected Mobility repository | in a few hours | "after that" |
| Adrian | Set up AI chat knowledge base with Confluence product information | TBD | "knowledge base" |
"#;
        let base = chrono::DateTime::parse_from_rfc3339("2026-06-19T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let drafts = build_draft_candidates(
            "meeting-1",
            Some("summary-1"),
            "Planning",
            summary,
            base,
            &test_defaults(),
        );
        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].category, "deploy_alert_check");
        assert!(drafts[0].due_at.is_some());
        assert!(drafts[0].confidence >= 0.8);
    }

    #[test]
    fn assigns_programmer_categories() {
        assert_eq!(
            categorize_action("Review the payment PR after CI passes"),
            "pr_review"
        );
        assert_eq!(
            categorize_action("Review experiment metrics next week"),
            "experiment_revisit"
        );
        assert_eq!(
            categorize_action("Update CHA-123 in Linear with acceptance criteria"),
            "linear_follow_up"
        );
        assert_eq!(
            categorize_action("Write README docs for the new setup"),
            "docs_update"
        );
        assert_eq!(
            categorize_action("Ask Kris to confirm the rollout owner"),
            "clarification_follow_up"
        );
    }

    #[test]
    fn infers_due_dates_conservatively() {
        let base = chrono::DateTime::parse_from_rfc3339("2026-06-19T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert!(infer_due_at("check production in a few hours", base)
            .unwrap()
            .starts_with("2026-06-19T13:00:00"));
        assert!(infer_due_at("Review PR tomorrow", base)
            .unwrap()
            .starts_with("2026-06-20T09:00:00"));
        assert!(infer_due_at("Implement vague repository cleanup", base).is_none());
    }

    #[test]
    fn dedupes_repeated_action_items() {
        let summary = r#"
## Action Items
- Review the auth PR tomorrow
- Review the auth PR tomorrow.
"#;
        let base = chrono::DateTime::parse_from_rfc3339("2026-06-19T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let drafts = build_draft_candidates(
            "meeting-1",
            Some("summary-1"),
            "Planning",
            summary,
            base,
            &test_defaults(),
        );
        assert_eq!(drafts.len(), 1);
    }

    #[test]
    fn workflow_presets_can_disable_categories() {
        let mut defaults = test_defaults();
        defaults.presets.get_mut("pr_review").unwrap().enabled = false;
        let base = chrono::DateTime::parse_from_rfc3339("2026-06-19T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let drafts = build_draft_candidates(
            "meeting-1",
            Some("summary-1"),
            "Planning",
            "## Action Items\n- Review the auth PR tomorrow",
            base,
            &defaults,
        );
        assert!(drafts.is_empty());
    }

    #[test]
    fn workflow_presets_apply_due_priority_list_and_reason() {
        let mut defaults = test_defaults();
        let preset = defaults.presets.get_mut("docs_update").unwrap();
        preset.default_list_id = Some("docs-list".to_string());
        preset.default_priority = Some(9);
        preset.due_preset = "in_2_days".to_string();
        let base = chrono::DateTime::parse_from_rfc3339("2026-06-19T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let drafts = build_draft_candidates(
            "meeting-1",
            Some("summary-1"),
            "Planning",
            "## Action Items\n- Update README docs for the release process",
            base,
            &defaults,
        );
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].category, "docs_update");
        assert_eq!(drafts[0].priority, Some(9));
        assert_eq!(drafts[0].list_id.as_deref(), Some("docs-list"));
        assert!(drafts[0]
            .due_at
            .as_deref()
            .unwrap()
            .starts_with("2026-06-21T10:00:00"));
        assert!(drafts[0]
            .notes
            .as_deref()
            .unwrap()
            .contains("Preset: Category: docs update"));
        assert!(!drafts[0]
            .source_evidence
            .iter()
            .any(|evidence| evidence.label == "Reminder preset"));
    }
}
