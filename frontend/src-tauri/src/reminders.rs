use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::Row;
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
}
