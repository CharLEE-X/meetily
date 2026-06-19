use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashSet;
use std::process::Command;
use tauri::State;
use uuid::Uuid;

const PROVIDER_APPLE_REMINDERS: &str = "apple_reminders";
const APPLE_REMINDERS_LABEL: &str = "Apple Reminders";

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

#[tauri::command]
pub async fn list_reminder_providers() -> Result<Vec<ReminderProviderInfo>, String> {
    Ok(provider_infos())
}

#[tauri::command]
pub async fn get_reminder_settings(
    state: State<'_, AppState>,
) -> Result<ReminderSettingsState, String> {
    let pool = state.db_manager.pool();
    Ok(ReminderSettingsState {
        providers: provider_infos(),
        accounts: list_accounts(pool).await?,
        lists: list_lists(pool).await?,
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

fn provider_infos() -> Vec<ReminderProviderInfo> {
    vec![ReminderProviderInfo {
        provider: PROVIDER_APPLE_REMINDERS.to_string(),
        label: APPLE_REMINDERS_LABEL.to_string(),
        available: cfg!(target_os = "macos"),
        supports_list_discovery: cfg!(target_os = "macos"),
        supports_create: false,
        notes: Some(
            if cfg!(target_os = "macos") {
                "Lists Apple Reminders destinations locally after the user grants permission. Reminder creation is not enabled in this slice."
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
            .bind(id)
            .execute(pool)
            .await
            .map_err(|err| format!("Failed to update stale reminder list: {}", err))?;
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
    let default_list_id = default_reminder_list_id(pool).await?;
    let summary_id = Some(summary.meeting_id.as_str());
    let generated = build_draft_candidates(
        meeting_id,
        summary_id.as_deref(),
        &meeting.title,
        &summary_text,
        base_time,
        default_list_id.as_deref(),
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
    category: String,
    confidence: f64,
    source_evidence: Vec<ReminderSourceEvidence>,
    dedupe_key: String,
}

fn build_draft_candidates(
    meeting_id: &str,
    summary_id: Option<&str>,
    meeting_title: &str,
    summary_text: &str,
    base_time: chrono::DateTime<chrono::Utc>,
    default_list_id: Option<&str>,
) -> Vec<ReminderDraft> {
    let mut seen = HashSet::new();
    extract_action_candidates(summary_text)
        .into_iter()
        .filter_map(|candidate| {
            let candidate = candidate_to_draft_candidate(meeting_id, &candidate, base_time)?;
            if !seen.insert(candidate.dedupe_key.clone()) {
                return None;
            }
            Some(candidate_to_reminder_draft(
                meeting_id,
                summary_id,
                meeting_title,
                default_list_id,
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
    let due_at = infer_due_at(&combined_due_text, &category, base_time);
    let priority = Some(priority_for_category(&category));
    let confidence = confidence_for_candidate(candidate, &category);
    let evidence = ReminderSourceEvidence {
        label: candidate.evidence_label.clone(),
        snippet: truncate_evidence(&candidate.text),
    };
    let due_bucket = due_at.as_deref().unwrap_or("undated");
    let dedupe_key = build_dedupe_key(meeting_id, &title, &category, due_bucket);
    Some(DraftCandidate {
        title,
        due_at,
        priority,
        category,
        confidence,
        source_evidence: vec![evidence],
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

fn infer_due_at(
    text: &str,
    category: &str,
    base_time: chrono::DateTime<chrono::Utc>,
) -> Option<String> {
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
    match category {
        "deploy_alert_check" => Some((base_time + Duration::hours(2)).to_rfc3339()),
        "pr_review" | "linear_follow_up" | "clarification_follow_up" => {
            let date = (base_time + Duration::days(1)).date_naive();
            chrono::Utc
                .with_ymd_and_hms(date.year(), date.month(), date.day(), 9, 0, 0)
                .single()
                .map(|dt| dt.to_rfc3339())
        }
        "docs_update" => Some((base_time + Duration::days(2)).to_rfc3339()),
        "experiment_revisit" => Some((base_time + Duration::days(7)).to_rfc3339()),
        _ => None,
    }
}

fn priority_for_category(category: &str) -> i64 {
    match category {
        "deploy_alert_check" => 1,
        "docs_update" | "experiment_revisit" => 9,
        _ => 5,
    }
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
    default_list_id: Option<&str>,
    candidate: DraftCandidate,
) -> ReminderDraft {
    let now = chrono::Utc::now().to_rfc3339();
    let notes = Some(format!(
        "From Meetily meeting: {}\nWhy: {}",
        meeting_title,
        candidate
            .source_evidence
            .first()
            .map(|evidence| evidence.snippet.as_str())
            .unwrap_or(candidate.title.as_str())
    ));
    ReminderDraft {
        id: Uuid::new_v4().to_string(),
        meeting_id: meeting_id.to_string(),
        summary_id: summary_id.map(str::to_string),
        title: candidate.title,
        notes,
        due_at: candidate.due_at,
        priority: candidate.priority,
        list_id: default_list_id.map(str::to_string),
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

async fn default_reminder_list_id(pool: &sqlx::SqlitePool) -> Result<Option<String>, String> {
    sqlx::query_scalar(
        "SELECT default_list_id FROM reminder_provider_accounts
         WHERE provider = ? AND status = 'connected'
         LIMIT 1",
    )
    .bind(PROVIDER_APPLE_REMINDERS)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect default reminder list: {}", err))
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
            None,
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
        assert!(infer_due_at(
            "check production in a few hours",
            "deploy_alert_check",
            base
        )
        .unwrap()
        .starts_with("2026-06-19T13:00:00"));
        assert!(infer_due_at("Review PR tomorrow", "pr_review", base)
            .unwrap()
            .starts_with("2026-06-20T09:00:00"));
        assert!(infer_due_at(
            "Implement vague repository cleanup",
            "implementation_task",
            base
        )
        .is_none());
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
            None,
        );
        assert_eq!(drafts.len(), 1);
    }
}
