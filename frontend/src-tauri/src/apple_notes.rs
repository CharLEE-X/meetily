use crate::database::repositories::meeting::MeetingsRepository;
use crate::database::repositories::summary::SummaryProcessesRepository;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::fs;
use std::io::Write;
use std::process::Command;
use tauri::State;
use uuid::Uuid;

const PROVIDER_APPLE_NOTES: &str = "apple_notes";
const APPLE_NOTES_LABEL: &str = "Apple Notes";
const DEFAULT_ROOT_FOLDER: &str = "Meetily";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotesProviderInfo {
    pub provider: String,
    pub label: String,
    pub available: bool,
    pub supports_write: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotesProviderAccount {
    pub id: String,
    pub provider: String,
    pub account_label: String,
    pub status: String,
    pub root_folder_name: String,
    pub grouping_mode: String,
    pub auto_export_enabled: bool,
    pub confirmed_destination_hash: Option<String>,
    pub last_export_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotesSettingsState {
    pub providers: Vec<AppleNotesProviderInfo>,
    pub accounts: Vec<AppleNotesProviderAccount>,
    pub recent_exports: Vec<AppleNotesExportRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotesExportRecord {
    pub id: String,
    pub meeting_id: String,
    pub provider: String,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub provider_note_id: Option<String>,
    pub note_title: String,
    pub content_hash: String,
    pub status: String,
    pub last_error: Option<String>,
    pub exported_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotesExportPreview {
    pub meeting_id: String,
    pub note_title: String,
    pub account_label: String,
    pub folder_name: String,
    pub content_hash: String,
    pub destination_hash: String,
    pub summary_available: bool,
    pub transcript_reference: Option<String>,
    pub sections: Vec<String>,
    pub requires_destination_confirmation: bool,
    pub i_cloud_sync_disclosure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotesExportRequest {
    pub meeting_id: String,
    pub confirm_destination_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNotesSettingsUpdateRequest {
    pub provider: Option<String>,
    pub root_folder_name: Option<String>,
    pub auto_export_enabled: Option<bool>,
}

#[derive(Debug, Clone)]
struct NotesExportPayload {
    meeting_id: String,
    note_title: String,
    body_html: String,
    content_hash: String,
    transcript_reference: Option<String>,
    sections: Vec<String>,
}

#[derive(Debug, Clone)]
struct AppleNotesWriteResult {
    account_id: Option<String>,
    account_name: Option<String>,
    folder_id: Option<String>,
    folder_name: Option<String>,
    note_id: String,
    status: String,
}

#[tauri::command]
pub async fn list_apple_notes_providers() -> Result<Vec<AppleNotesProviderInfo>, String> {
    Ok(provider_infos())
}

#[tauri::command]
pub async fn get_apple_notes_settings(
    state: State<'_, AppState>,
) -> Result<AppleNotesSettingsState, String> {
    let pool = state.db_manager.pool();
    Ok(AppleNotesSettingsState {
        providers: provider_infos(),
        accounts: list_accounts(pool).await?,
        recent_exports: list_recent_exports_for_pool(pool, 10).await?,
    })
}

#[tauri::command]
pub async fn connect_apple_notes_provider(
    state: State<'_, AppState>,
    provider: String,
) -> Result<AppleNotesProviderAccount, String> {
    let provider = normalize_provider(&provider)?;
    let pool = state.db_manager.pool();
    connect_provider_account(pool, &provider).await
}

#[tauri::command]
pub async fn disconnect_apple_notes_provider(
    state: State<'_, AppState>,
    provider: String,
) -> Result<AppleNotesProviderAccount, String> {
    let provider = normalize_provider(&provider)?;
    let pool = state.db_manager.pool();
    let account_id = existing_account_id(pool, &provider)
        .await?
        .ok_or_else(|| "Apple Notes is not connected.".to_string())?;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE apple_notes_provider_accounts
         SET status = 'revoked', last_error = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to disconnect Apple Notes: {}", err))?;

    sqlx::query(
        "UPDATE apple_notes_exports
         SET status = CASE WHEN status = 'failed' THEN status ELSE 'revoked' END,
             updated_at = ?
         WHERE provider = ?",
    )
    .bind(&now)
    .bind(&provider)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update Apple Notes export history: {}", err))?;

    get_account(pool, &provider)
        .await?
        .ok_or_else(|| "Apple Notes account was not found after disconnect.".to_string())
}

#[tauri::command]
pub async fn update_apple_notes_settings(
    state: State<'_, AppState>,
    request: AppleNotesSettingsUpdateRequest,
) -> Result<AppleNotesProviderAccount, String> {
    let provider = normalize_provider(request.provider.as_deref().unwrap_or(PROVIDER_APPLE_NOTES))?;
    let pool = state.db_manager.pool();
    let existing = match get_account(pool, &provider).await? {
        Some(account) => account,
        None => connect_provider_account(pool, &provider).await?,
    };
    let now = Utc::now().to_rfc3339();
    let root_folder_name = request
        .root_folder_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&existing.root_folder_name);
    let auto_export_enabled = request
        .auto_export_enabled
        .unwrap_or(existing.auto_export_enabled);

    sqlx::query(
        "UPDATE apple_notes_provider_accounts
         SET root_folder_name = ?,
             auto_export_enabled = ?,
             confirmed_destination_hash = CASE
                WHEN root_folder_name = ? THEN confirmed_destination_hash
                ELSE NULL
             END,
             updated_at = ?
         WHERE provider = ?",
    )
    .bind(root_folder_name)
    .bind(if auto_export_enabled { 1 } else { 0 })
    .bind(root_folder_name)
    .bind(&now)
    .bind(&provider)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update Apple Notes settings: {}", err))?;

    get_account(pool, &provider)
        .await?
        .ok_or_else(|| "Apple Notes settings were not saved.".to_string())
}

#[tauri::command]
pub async fn preview_apple_notes_export(
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<AppleNotesExportPreview, String> {
    let pool = state.db_manager.pool();
    let account = get_account(pool, PROVIDER_APPLE_NOTES).await?;
    let payload = build_export_payload(pool, &meeting_id).await?;
    let account_label = account
        .as_ref()
        .map(|account| account.account_label.clone())
        .unwrap_or_else(|| APPLE_NOTES_LABEL.to_string());
    let folder_name = account
        .as_ref()
        .map(|account| account.root_folder_name.clone())
        .unwrap_or_else(|| DEFAULT_ROOT_FOLDER.to_string());
    let grouping_mode = account
        .as_ref()
        .map(|account| account.grouping_mode.as_str())
        .unwrap_or("none");
    let destination_hash = destination_hash(
        &account_label,
        &folder_name,
        grouping_mode,
        &payload.note_title,
    );
    let requires_destination_confirmation = account
        .as_ref()
        .and_then(|account| account.confirmed_destination_hash.as_deref())
        != Some(destination_hash.as_str());
    Ok(AppleNotesExportPreview {
        meeting_id: payload.meeting_id,
        note_title: payload.note_title,
        account_label: account_label.clone(),
        folder_name,
        content_hash: payload.content_hash,
        destination_hash,
        summary_available: payload.sections.iter().any(|section| section == "Summary"),
        transcript_reference: payload.transcript_reference,
        sections: payload.sections,
        requires_destination_confirmation,
        i_cloud_sync_disclosure: if account_label.to_ascii_lowercase().contains("icloud") {
            Some(
                "Apple Notes may sync this exported meeting through your Apple account."
                    .to_string(),
            )
        } else {
            None
        },
    })
}

#[tauri::command]
pub async fn export_meeting_to_apple_notes(
    state: State<'_, AppState>,
    request: AppleNotesExportRequest,
) -> Result<AppleNotesExportRecord, String> {
    if !cfg!(target_os = "macos") {
        return Err("Apple Notes export is available only on macOS.".to_string());
    }

    let pool = state.db_manager.pool();
    let account = match get_account(pool, PROVIDER_APPLE_NOTES).await? {
        Some(account) if account.status == "connected" || account.status == "permission_needed" => {
            account
        }
        Some(_) => return Err("Reconnect Apple Notes before exporting this meeting.".to_string()),
        None => {
            return Err(
                "Connect Apple Notes in Settings before exporting this meeting.".to_string(),
            );
        }
    };

    let payload = build_export_payload(pool, &request.meeting_id).await?;
    let destination = destination_hash(
        &account.account_label,
        &account.root_folder_name,
        &account.grouping_mode,
        &payload.note_title,
    );
    if request.confirm_destination_hash.as_deref() != Some(destination.as_str()) {
        return Err(
            "Confirm the Apple Notes destination before exporting this meeting.".to_string(),
        );
    }

    let existing = get_export_record(pool, &request.meeting_id).await?;
    let write_result = match write_apple_note(
        existing
            .as_ref()
            .and_then(|record| record.provider_note_id.as_deref()),
        &account.root_folder_name,
        &payload.note_title,
        &payload.body_html,
    ) {
        Ok(result) => result,
        Err(error) => {
            let safe_error = sanitize_notes_error(&error);
            save_failed_export(pool, &account, &payload, &safe_error).await?;
            update_account_status(pool, "error", Some(&safe_error), None).await?;
            return Err(safe_error);
        }
    };

    let record =
        save_successful_export(pool, &account, &payload, &write_result, &destination).await?;
    attach_notes_export_to_calendar_link(pool, &record.meeting_id, &record.id).await?;
    update_account_status(pool, "connected", None, Some(Utc::now().to_rfc3339())).await?;
    Ok(record)
}

#[tauri::command]
pub async fn get_meeting_apple_notes_export(
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<Option<AppleNotesExportRecord>, String> {
    get_export_record(state.db_manager.pool(), &meeting_id).await
}

#[tauri::command]
pub async fn list_recent_apple_notes_exports(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<AppleNotesExportRecord>, String> {
    list_recent_exports_for_pool(state.db_manager.pool(), limit.unwrap_or(10).clamp(1, 50)).await
}

fn provider_infos() -> Vec<AppleNotesProviderInfo> {
    vec![AppleNotesProviderInfo {
        provider: PROVIDER_APPLE_NOTES.to_string(),
        label: APPLE_NOTES_LABEL.to_string(),
        available: cfg!(target_os = "macos"),
        supports_write: cfg!(target_os = "macos"),
        notes: Some(
            if cfg!(target_os = "macos") {
                "Writes app-managed meeting summary notes through local macOS Automation after Connect verifies permission."
            } else {
                "Apple Notes export is available only on macOS."
            }
            .to_string(),
        ),
    }]
}

async fn connect_provider_account(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<AppleNotesProviderAccount, String> {
    let now = Utc::now().to_rfc3339();
    let account_id = existing_account_id(pool, provider)
        .await?
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let (status, account_label, error) = probe_apple_notes_access()
        .map(|label| ("connected", label, None))
        .unwrap_or_else(|error| {
            let status = notes_error_status(&error);
            (
                status,
                APPLE_NOTES_LABEL.to_string(),
                Some(sanitize_notes_error(&error)),
            )
        });

    sqlx::query(
        "INSERT INTO apple_notes_provider_accounts
            (id, provider, account_label, status, root_folder_name, grouping_mode,
             auto_export_enabled, confirmed_destination_hash, last_export_at, last_error, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'none', 0, NULL, NULL, ?, ?, ?)
         ON CONFLICT(provider) DO UPDATE SET
            account_label = excluded.account_label,
            status = excluded.status,
            last_error = excluded.last_error,
            updated_at = excluded.updated_at",
    )
    .bind(&account_id)
    .bind(provider)
    .bind(account_label)
    .bind(status)
    .bind(DEFAULT_ROOT_FOLDER)
    .bind(error)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to connect Apple Notes: {}", err))?;

    get_account(pool, provider)
        .await?
        .ok_or_else(|| "Apple Notes account was not saved.".to_string())
}

fn probe_apple_notes_access() -> Result<String, String> {
    if !cfg!(target_os = "macos") {
        return Err("Apple Notes export is available only on macOS.".to_string());
    }

    let output = Command::new("osascript")
        .args(["-e", &apple_notes_access_probe_script()])
        .output()
        .map_err(|err| format!("Failed to verify Apple Notes access: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Apple Notes permission is required before exporting summaries.".to_string()
        } else {
            stderr
        });
    }

    let label = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if label.is_empty() || label == "missing value" {
        APPLE_NOTES_LABEL.to_string()
    } else {
        label
    })
}

fn apple_notes_access_probe_script() -> String {
    r#"tell application id "com.apple.Notes"
    set accountName to "Apple Notes"
    try
        if (count of accounts) > 0 then set accountName to (name of first account as text)
    end try
    return accountName
end tell
"#
    .to_string()
}

fn notes_error_status(error: &str) -> &'static str {
    let lower = error.to_lowercase();
    if lower.contains("permission")
        || lower.contains("not authorized")
        || lower.contains("not authorised")
        || lower.contains("not permitted")
        || lower.contains("automation")
        || lower.contains("-1743")
    {
        "permission_needed"
    } else {
        "error"
    }
}

async fn build_export_payload(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<NotesExportPayload, String> {
    let meeting = MeetingsRepository::get_meeting_metadata(pool, meeting_id)
        .await
        .map_err(|err| format!("Failed to load meeting for Apple Notes export: {}", err))?
        .ok_or_else(|| "Meeting was not found for Apple Notes export.".to_string())?;
    let summary = SummaryProcessesRepository::get_summary_data(pool, meeting_id)
        .await
        .map_err(|err| {
            format!(
                "Failed to load meeting summary for Apple Notes export: {}",
                err
            )
        })?;
    let summary_text = summary
        .and_then(|summary| summary.result)
        .map(|result| summary_result_to_text(&result))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Generate a meeting summary before exporting to Apple Notes.".to_string())?;
    let transcript_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_one(pool)
            .await
            .map_err(|err| {
                format!(
                    "Failed to inspect meeting transcript for Apple Notes export: {}",
                    err
                )
            })?;
    let transcript_reference = if transcript_count > 0 {
        Some(format!(
            "Meetily transcript available locally for meeting {}",
            meeting_id
        ))
    } else {
        None
    };
    let meeting_created_at = meeting.created_at.0.to_rfc3339();
    let note_title = note_title(&meeting.title, &meeting_created_at);
    let sections = vec![
        "Meeting metadata".to_string(),
        "Summary".to_string(),
        "Transcript reference".to_string(),
    ];
    let body_text = format!(
        "{}\n\nDate: {}\nMeeting ID: {}\n\n{}\n\n{}\n\nCreated by Meetily",
        meeting.title,
        meeting_created_at,
        meeting_id,
        summary_text,
        transcript_reference
            .clone()
            .unwrap_or_else(|| "No transcript reference available.".to_string())
    );
    let body_html = render_note_html(
        &meeting.title,
        &meeting_created_at,
        meeting_id,
        &summary_text,
        transcript_reference.as_deref(),
    );
    let content_hash = content_hash(&[&note_title, &body_text]);

    Ok(NotesExportPayload {
        meeting_id: meeting_id.to_string(),
        note_title,
        body_html,
        content_hash,
        transcript_reference,
        sections,
    })
}

fn note_title(meeting_title: &str, created_at: &str) -> String {
    let date = created_at.split('T').next().unwrap_or(created_at);
    format!(
        "{} - {}",
        date,
        meeting_title.trim().if_empty("Untitled meeting")
    )
}

trait IfEmpty {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl IfEmpty for str {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.is_empty() {
            fallback
        } else {
            self
        }
    }
}

fn summary_result_to_text(raw: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(value) => json_summary_to_text(&value),
        Err(_) => raw.to_string(),
    }
}

fn json_summary_to_text(value: &serde_json::Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    if let Some(summary) = value.get("summary").and_then(|value| value.as_str()) {
        return summary.to_string();
    }
    if let Some(markdown) = value.get("markdown").and_then(|value| value.as_str()) {
        return markdown.to_string();
    }
    if let Some(result) = value.get("result") {
        let text = json_summary_to_text(result);
        if !text.trim().is_empty() {
            return text;
        }
    }
    if let Some(object) = value.as_object() {
        let mut sections = Vec::new();
        for (key, value) in object {
            let text = json_summary_to_text(value);
            if !text.trim().is_empty() {
                sections.push(format!("{}\n{}", humanize_key(key), text));
            }
        }
        return sections.join("\n\n");
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(json_summary_to_text)
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }
    String::new()
}

fn humanize_key(key: &str) -> String {
    key.replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_note_html(
    meeting_title: &str,
    created_at: &str,
    meeting_id: &str,
    summary_text: &str,
    transcript_reference: Option<&str>,
) -> String {
    format!(
        "<h1>{}</h1><p><strong>Date:</strong> {}</p><p><strong>Meeting ID:</strong> {}</p><h2>Summary</h2>{}<h2>Transcript</h2><p>{}</p><hr><p>Created by Meetily</p>",
        html_escape(meeting_title),
        html_escape(created_at),
        html_escape(meeting_id),
        paragraphs_to_html(summary_text),
        html_escape(transcript_reference.unwrap_or("No transcript reference available."))
    )
}

fn paragraphs_to_html(value: &str) -> String {
    value
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(|paragraph| format!("<p>{}</p>", html_escape(paragraph).replace('\n', "<br>")))
        .collect::<Vec<_>>()
        .join("")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn write_apple_note(
    existing_note_id: Option<&str>,
    root_folder_name: &str,
    note_title: &str,
    body_html: &str,
) -> Result<AppleNotesWriteResult, String> {
    let body_file = write_restrictive_temp_file(body_html)?;
    let script = apple_notes_write_script();
    let output = Command::new("osascript")
        .args([
            "-e",
            &script,
            "--",
            existing_note_id.unwrap_or(""),
            root_folder_name,
            note_title,
            body_file.to_string_lossy().as_ref(),
        ])
        .output()
        .map_err(|err| format!("Failed to run Apple Notes export: {}", err));
    let _ = fs::remove_file(&body_file);
    let output = output?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Apple Notes could not export this meeting.".to_string()
        } else {
            stderr
        });
    }

    parse_write_result(String::from_utf8_lossy(&output.stdout).trim())
}

fn write_restrictive_temp_file(body_html: &str) -> Result<std::path::PathBuf, String> {
    let path = std::env::temp_dir().join(format!("meetily-notes-{}.html", Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .map_err(|err| format!("Failed to prepare Apple Notes export body: {}", err))?;
    file.write_all(body_html.as_bytes())
        .map_err(|err| format!("Failed to write Apple Notes export body: {}", err))?;
    Ok(path)
}

fn apple_notes_write_script() -> String {
    r#"on run argv
    set existingNoteId to item 1 of argv
    set rootFolderName to item 2 of argv
    set noteTitle to item 3 of argv
    set bodyFilePath to item 4 of argv
    set noteBody to read POSIX file bodyFilePath as «class utf8»
    tell application id "com.apple.Notes"
        set targetAccount to missing value
        repeat with notesAccount in accounts
            try
                if (name of notesAccount as text) is "On My Mac" then
                    set targetAccount to notesAccount
                    exit repeat
                end if
            end try
        end repeat
        if targetAccount is missing value then set targetAccount to first account
        set targetFolder to missing value
        repeat with notesFolder in folders of targetAccount
            try
                if (name of notesFolder as text) is rootFolderName then
                    set targetFolder to notesFolder
                    exit repeat
                end if
            end try
        end repeat
        if targetFolder is missing value then
            set targetFolder to make new folder at targetAccount with properties {name:rootFolderName}
        end if
        set targetNote to missing value
        if existingNoteId is not "" then
            try
                if exists note id existingNoteId then set targetNote to note id existingNoteId
            end try
        end if
        set writeStatus to "created"
        if targetNote is missing value then
            set targetNote to make new note at targetFolder with properties {name:noteTitle, body:noteBody}
        else
            set name of targetNote to noteTitle
            set body of targetNote to noteBody
            set writeStatus to "updated"
        end if
        return (id of targetAccount as text) & (character id 30) & (name of targetAccount as text) & (character id 30) & (id of targetFolder as text) & (character id 30) & (name of targetFolder as text) & (character id 30) & (id of targetNote as text) & (character id 30) & writeStatus
    end tell
end run
"#
    .to_string()
}

fn parse_write_result(row: &str) -> Result<AppleNotesWriteResult, String> {
    let parts = row.split('\u{1e}').collect::<Vec<_>>();
    if parts.len() < 6 {
        return Err("Apple Notes did not return note export metadata.".to_string());
    }
    let note_id = parts[4].trim();
    if note_id.is_empty() {
        return Err("Apple Notes did not return a note id.".to_string());
    }
    Ok(AppleNotesWriteResult {
        account_id: clean_optional(parts[0]),
        account_name: clean_optional(parts[1]),
        folder_id: clean_optional(parts[2]),
        folder_name: clean_optional(parts[3]),
        note_id: note_id.to_string(),
        status: if parts[5].trim() == "updated" {
            "updated"
        } else {
            "exported"
        }
        .to_string(),
    })
}

async fn save_successful_export(
    pool: &sqlx::SqlitePool,
    account: &AppleNotesProviderAccount,
    payload: &NotesExportPayload,
    write_result: &AppleNotesWriteResult,
    confirmed_destination_hash: &str,
) -> Result<AppleNotesExportRecord, String> {
    let now = Utc::now().to_rfc3339();
    let export_id = existing_export_id(pool, &payload.meeting_id)
        .await?
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        "INSERT INTO apple_notes_exports
            (id, meeting_id, provider, account_id, account_name, folder_id, folder_name,
             provider_note_id, note_title, content_hash, status, last_error, exported_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?)
         ON CONFLICT(meeting_id, provider) DO UPDATE SET
            account_id = excluded.account_id,
            account_name = excluded.account_name,
            folder_id = excluded.folder_id,
            folder_name = excluded.folder_name,
            provider_note_id = excluded.provider_note_id,
            note_title = excluded.note_title,
            content_hash = excluded.content_hash,
            status = excluded.status,
            last_error = NULL,
            exported_at = excluded.exported_at,
            updated_at = excluded.updated_at",
    )
    .bind(&export_id)
    .bind(&payload.meeting_id)
    .bind(PROVIDER_APPLE_NOTES)
    .bind(write_result.account_id.as_deref())
    .bind(write_result.account_name.as_deref())
    .bind(write_result.folder_id.as_deref())
    .bind(write_result.folder_name.as_deref())
    .bind(&write_result.note_id)
    .bind(&payload.note_title)
    .bind(&payload.content_hash)
    .bind(&write_result.status)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save Apple Notes export record: {}", err))?;

    sqlx::query(
        "UPDATE apple_notes_provider_accounts
         SET account_label = ?, status = 'connected', confirmed_destination_hash = ?,
             last_export_at = ?, last_error = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(
        write_result
            .account_name
            .as_deref()
            .unwrap_or(&account.account_label),
    )
    .bind(confirmed_destination_hash)
    .bind(&now)
    .bind(&now)
    .bind(&account.id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update Apple Notes account status: {}", err))?;

    get_export_record(pool, &payload.meeting_id)
        .await?
        .ok_or_else(|| "Apple Notes export record was not saved.".to_string())
}

async fn save_failed_export(
    pool: &sqlx::SqlitePool,
    account: &AppleNotesProviderAccount,
    payload: &NotesExportPayload,
    error: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let export_id = existing_export_id(pool, &payload.meeting_id)
        .await?
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    sqlx::query(
        "INSERT INTO apple_notes_exports
            (id, meeting_id, provider, account_id, account_name, folder_id, folder_name,
             provider_note_id, note_title, content_hash, status, last_error, exported_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, NULL, ?, NULL, ?, ?, 'failed', ?, NULL, ?, ?)
         ON CONFLICT(meeting_id, provider) DO UPDATE SET
            content_hash = excluded.content_hash,
            status = 'failed',
            last_error = excluded.last_error,
            updated_at = excluded.updated_at",
    )
    .bind(&export_id)
    .bind(&payload.meeting_id)
    .bind(PROVIDER_APPLE_NOTES)
    .bind(&account.id)
    .bind(&account.account_label)
    .bind(&account.root_folder_name)
    .bind(&payload.note_title)
    .bind(&payload.content_hash)
    .bind(error)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save Apple Notes export failure: {}", err))?;
    Ok(())
}

async fn attach_notes_export_to_calendar_link(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    notes_export_id: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE meeting_calendar_links
         SET notes_export_id = ?,
             updated_at = ?
         WHERE meeting_id = ?",
    )
    .bind(notes_export_id)
    .bind(&now)
    .bind(meeting_id)
    .execute(pool)
    .await
    .map_err(|err| {
        format!(
            "Failed to link Apple Notes export with the meeting calendar record: {}",
            err
        )
    })?;
    Ok(())
}

async fn update_account_status(
    pool: &sqlx::SqlitePool,
    status: &str,
    error: Option<&str>,
    last_export_at: Option<String>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE apple_notes_provider_accounts
         SET status = ?, last_error = ?, last_export_at = COALESCE(?, last_export_at), updated_at = ?
         WHERE provider = ?",
    )
    .bind(status)
    .bind(error)
    .bind(last_export_at)
    .bind(&now)
    .bind(PROVIDER_APPLE_NOTES)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update Apple Notes account status: {}", err))?;
    Ok(())
}

async fn get_account(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<Option<AppleNotesProviderAccount>, String> {
    let row = sqlx::query(
        "SELECT id, provider, account_label, status, root_folder_name, grouping_mode,
                auto_export_enabled, confirmed_destination_hash, last_export_at,
                last_error, created_at, updated_at
         FROM apple_notes_provider_accounts
         WHERE provider = ?",
    )
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load Apple Notes account: {}", err))?;
    row.map(account_from_row).transpose()
}

async fn list_accounts(pool: &sqlx::SqlitePool) -> Result<Vec<AppleNotesProviderAccount>, String> {
    let rows = sqlx::query(
        "SELECT id, provider, account_label, status, root_folder_name, grouping_mode,
                auto_export_enabled, confirmed_destination_hash, last_export_at,
                last_error, created_at, updated_at
         FROM apple_notes_provider_accounts
         ORDER BY provider ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list Apple Notes accounts: {}", err))?;
    rows.into_iter().map(account_from_row).collect()
}

async fn get_export_record(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Option<AppleNotesExportRecord>, String> {
    let row = sqlx::query(
        "SELECT id, meeting_id, provider, account_id, account_name, folder_id, folder_name,
                provider_note_id, note_title, content_hash, status, last_error,
                exported_at, created_at, updated_at
         FROM apple_notes_exports
         WHERE meeting_id = ? AND provider = ?",
    )
    .bind(meeting_id)
    .bind(PROVIDER_APPLE_NOTES)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load Apple Notes export record: {}", err))?;
    row.map(export_record_from_row).transpose()
}

async fn list_recent_exports_for_pool(
    pool: &sqlx::SqlitePool,
    limit: i64,
) -> Result<Vec<AppleNotesExportRecord>, String> {
    let rows = sqlx::query(
        "SELECT id, meeting_id, provider, account_id, account_name, folder_id, folder_name,
                provider_note_id, note_title, content_hash, status, last_error,
                exported_at, created_at, updated_at
         FROM apple_notes_exports
         ORDER BY COALESCE(exported_at, updated_at) DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list Apple Notes export history: {}", err))?;
    rows.into_iter().map(export_record_from_row).collect()
}

async fn existing_account_id(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<Option<String>, String> {
    sqlx::query_scalar("SELECT id FROM apple_notes_provider_accounts WHERE provider = ?")
        .bind(provider)
        .fetch_optional(pool)
        .await
        .map_err(|err| format!("Failed to inspect Apple Notes account: {}", err))
}

async fn existing_export_id(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Option<String>, String> {
    sqlx::query_scalar("SELECT id FROM apple_notes_exports WHERE meeting_id = ? AND provider = ?")
        .bind(meeting_id)
        .bind(PROVIDER_APPLE_NOTES)
        .fetch_optional(pool)
        .await
        .map_err(|err| format!("Failed to inspect Apple Notes export record: {}", err))
}

fn account_from_row(row: sqlx::sqlite::SqliteRow) -> Result<AppleNotesProviderAccount, String> {
    let auto_export: i64 = row.get("auto_export_enabled");
    Ok(AppleNotesProviderAccount {
        id: row.get("id"),
        provider: row.get("provider"),
        account_label: row.get("account_label"),
        status: row.get("status"),
        root_folder_name: row.get("root_folder_name"),
        grouping_mode: row.get("grouping_mode"),
        auto_export_enabled: auto_export != 0,
        confirmed_destination_hash: row.get("confirmed_destination_hash"),
        last_export_at: row.get("last_export_at"),
        last_error: row.get("last_error"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn export_record_from_row(row: sqlx::sqlite::SqliteRow) -> Result<AppleNotesExportRecord, String> {
    Ok(AppleNotesExportRecord {
        id: row.get("id"),
        meeting_id: row.get("meeting_id"),
        provider: row.get("provider"),
        account_id: row.get("account_id"),
        account_name: row.get("account_name"),
        folder_id: row.get("folder_id"),
        folder_name: row.get("folder_name"),
        provider_note_id: row.get("provider_note_id"),
        note_title: row.get("note_title"),
        content_hash: row.get("content_hash"),
        status: row.get("status"),
        last_error: row.get("last_error"),
        exported_at: row.get("exported_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn normalize_provider(provider: &str) -> Result<String, String> {
    match provider.trim() {
        PROVIDER_APPLE_NOTES => Ok(PROVIDER_APPLE_NOTES.to_string()),
        "apple" => Ok(PROVIDER_APPLE_NOTES.to_string()),
        _ => Err("Unsupported Apple Notes provider.".to_string()),
    }
}

fn destination_hash(
    account_label: &str,
    folder_name: &str,
    grouping_mode: &str,
    note_title: &str,
) -> String {
    content_hash(&[account_label, folder_name, grouping_mode, note_title])
}

fn content_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn clean_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn sanitize_notes_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if notes_error_status(error) == "permission_needed" {
        return "Apple Notes permission is required. Allow Meetily to control Notes in System Settings > Privacy & Security > Automation, then retry.".to_string();
    }
    if lower.contains("can’t get application")
        || lower.contains("can't get application")
        || lower.contains("not available")
    {
        return "Apple Notes is not available on this Mac.".to_string();
    }
    let trimmed = error.trim();
    if trimmed.is_empty() {
        "Apple Notes export failed. Check Notes permissions and try again.".to_string()
    } else {
        trimmed.chars().take(280).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_html_escapes_meeting_content() {
        let html = render_note_html(
            "Planning <Review>",
            "2026-06-19T10:00:00Z",
            "meeting-1",
            "Ship & verify\n\nUse \"safe\" output",
            Some("Local transcript <available>"),
        );
        assert!(html.contains("Planning &lt;Review&gt;"));
        assert!(html.contains("Ship &amp; verify"));
        assert!(html.contains("&quot;safe&quot;"));
        assert!(html.contains("Local transcript &lt;available&gt;"));
        assert!(!html.contains("Planning <Review>"));
    }

    #[test]
    fn apple_notes_script_uses_temp_body_file_and_existing_note_probe() {
        let script = apple_notes_write_script();
        assert!(script.contains("read POSIX file bodyFilePath"));
        assert!(script.contains("exists note id existingNoteId"));
        assert!(script.contains("make new folder at targetAccount"));
        assert!(script.contains("make new note at targetFolder"));
        assert!(!script.contains("noteBody to item"));
    }

    #[test]
    fn parse_write_result_maps_created_and_updated() {
        let created = parse_write_result(
            "acct\u{1e}On My Mac\u{1e}folder\u{1e}Meetily\u{1e}note-1\u{1e}created",
        )
        .unwrap();
        assert_eq!(created.note_id, "note-1");
        assert_eq!(created.status, "exported");
        assert_eq!(created.folder_name.as_deref(), Some("Meetily"));

        let updated = parse_write_result(
            "acct\u{1e}On My Mac\u{1e}folder\u{1e}Meetily\u{1e}note-1\u{1e}updated",
        )
        .unwrap();
        assert_eq!(updated.status, "updated");
    }

    #[test]
    fn json_summary_to_text_extracts_common_shapes() {
        let raw = serde_json::json!({
            "summary": "Main summary",
            "action_items": ["A", "B"]
        });
        assert_eq!(json_summary_to_text(&raw), "Main summary");

        let raw = serde_json::json!({
            "key_decisions": "Decision",
            "risks": "None"
        });
        let text = json_summary_to_text(&raw);
        assert!(text.contains("Key Decisions"));
        assert!(text.contains("Decision"));
        assert!(text.contains("Risks"));
    }

    #[test]
    fn sanitize_permission_error_is_actionable() {
        let error = "execution error: Not authorized to send Apple events to Notes. (-1743)";
        let sanitized = sanitize_notes_error(error);
        assert!(sanitized.contains("System Settings"));
        assert!(!sanitized.contains("-1743"));
    }
}
