use crate::database::repositories::meeting::MeetingsRepository;
use crate::database::repositories::summary::SummaryProcessesRepository;
use crate::state::AppState;
use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::process::Command;
use tauri::State;
use url::Url;
use uuid::Uuid;

const PROVIDER_APPLE: &str = "apple";
const APPLE_ACCOUNT_LABEL: &str = "Apple Calendar";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarProviderInfo {
    pub provider: String,
    pub label: String,
    pub available: bool,
    pub supports_read: bool,
    pub supports_write: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarProviderAccount {
    pub id: String,
    pub provider: String,
    pub account_label: String,
    pub status: String,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub target_calendar_name: String,
    pub auto_create_events: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarSource {
    pub id: String,
    pub provider_account_id: String,
    pub provider_calendar_id: String,
    pub name: String,
    pub color: Option<String>,
    pub selected: bool,
    pub read_only: bool,
    pub last_sync_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: String,
    pub provider: String,
    pub provider_event_id: String,
    pub calendar_source_id: String,
    pub title: String,
    pub starts_at: String,
    pub ends_at: String,
    pub timezone: Option<String>,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub meeting_provider: Option<String>,
    pub attendee_count: Option<i64>,
    pub attendee_names: Option<Vec<String>>,
    pub organizer_name: Option<String>,
    pub description_excerpt: Option<String>,
    pub content_hash: String,
    pub sync_status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarSettingsState {
    pub providers: Vec<CalendarProviderInfo>,
    pub accounts: Vec<CalendarProviderAccount>,
    pub sources: Vec<CalendarSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarSyncRequest {
    pub provider: Option<String>,
    pub lookback_days: Option<i64>,
    pub lookahead_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarWriteSettingsRequest {
    pub provider: Option<String>,
    pub target_calendar_name: Option<String>,
    pub auto_create_events: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventCreationRequest {
    pub meeting_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventCreationResult {
    pub meeting_id: String,
    pub calendar_event_id: String,
    pub apple_event_identifier: String,
    pub calendar_name: String,
    pub status: String,
}

struct MeetingCalendarLinkRow {
    id: String,
    apple_event_identifier: Option<String>,
}

struct MeetingNotesExportLink {
    id: String,
    note_title: String,
    folder_name: Option<String>,
    provider_note_id: Option<String>,
    status: String,
}

struct AppleCalendarWriteResult {
    apple_event_identifier: String,
    calendar_name: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarSyncResult {
    pub provider: String,
    pub status: String,
    pub synced_event_count: usize,
    pub started_at: String,
    pub completed_at: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn list_calendar_providers() -> Result<Vec<CalendarProviderInfo>, String> {
    Ok(provider_infos())
}

#[tauri::command]
pub async fn get_calendar_settings(
    state: State<'_, AppState>,
) -> Result<CalendarSettingsState, String> {
    let pool = state.db_manager.pool();
    Ok(CalendarSettingsState {
        providers: provider_infos(),
        accounts: list_accounts(pool).await?,
        sources: list_sources(pool).await?,
    })
}

#[tauri::command]
pub async fn connect_calendar_provider(
    state: State<'_, AppState>,
    provider: String,
) -> Result<CalendarProviderAccount, String> {
    let provider = normalize_provider(&provider)?;
    let pool = state.db_manager.pool();
    connect_provider_account(pool, &provider).await
}

async fn connect_provider_account(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<CalendarProviderAccount, String> {
    let now = Utc::now().to_rfc3339();
    let account_id = existing_account_id(pool, provider)
        .await?
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let status = if provider == PROVIDER_APPLE && cfg!(target_os = "macos") {
        "permission_needed"
    } else {
        "error"
    };
    let label = provider_label(provider);
    let error = if status == "error" {
        Some("This calendar provider is not available on this platform yet.".to_string())
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO calendar_provider_accounts
            (id, provider, account_label, status, last_sync_at, last_error,
             target_calendar_name, auto_create_events, created_at, updated_at)
         VALUES (?, ?, ?, ?, NULL, ?, ?, 0, ?, ?)
         ON CONFLICT(provider) DO UPDATE SET
            account_label = excluded.account_label,
            status = excluded.status,
            last_error = excluded.last_error,
            updated_at = excluded.updated_at",
    )
    .bind(&account_id)
    .bind(provider)
    .bind(label)
    .bind(status)
    .bind(error)
    .bind("Meetily")
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to connect calendar provider: {}", err))?;

    if provider == PROVIDER_APPLE {
        ensure_source(
            pool,
            &account_id,
            "apple-calendar",
            APPLE_ACCOUNT_LABEL,
            true,
            false,
        )
        .await?;
    }

    get_account(pool, provider)
        .await?
        .ok_or_else(|| "Calendar provider was not saved".to_string())
}

#[tauri::command]
pub async fn disconnect_calendar_provider(
    state: State<'_, AppState>,
    provider: String,
) -> Result<CalendarProviderAccount, String> {
    let provider = normalize_provider(&provider)?;
    let pool = state.db_manager.pool();
    let now = Utc::now().to_rfc3339();
    let account_id = existing_account_id(pool, &provider)
        .await?
        .ok_or_else(|| "Calendar provider is not connected".to_string())?;

    sqlx::query(
        "UPDATE calendar_events
         SET sync_status = 'revoked', updated_at = ?
         WHERE calendar_source_id IN (
            SELECT id FROM calendar_sources WHERE provider_account_id = ?
         )",
    )
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to revoke calendar event cache: {}", err))?;

    sqlx::query(
        "UPDATE calendar_sources SET selected = 0, updated_at = ? WHERE provider_account_id = ?",
    )
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update calendar sources: {}", err))?;

    sqlx::query(
        "UPDATE calendar_provider_accounts
         SET status = 'revoked', last_error = NULL, last_sync_at = NULL, updated_at = ?
         WHERE id = ?",
    )
    .bind(&now)
    .bind(&account_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to disconnect calendar provider: {}", err))?;

    get_account(pool, &provider)
        .await?
        .ok_or_else(|| "Calendar provider was not found after disconnect".to_string())
}

#[tauri::command]
pub async fn update_calendar_write_settings(
    state: State<'_, AppState>,
    request: CalendarWriteSettingsRequest,
) -> Result<CalendarProviderAccount, String> {
    let provider = normalize_provider(request.provider.as_deref().unwrap_or(PROVIDER_APPLE))?;
    let pool = state.db_manager.pool();
    let existing = match get_account(pool, &provider).await? {
        Some(account) => account,
        None => connect_provider_account(pool, &provider).await?,
    };
    let now = Utc::now().to_rfc3339();
    let target_calendar_name = request
        .target_calendar_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&existing.target_calendar_name);
    let auto_create_events = request
        .auto_create_events
        .unwrap_or(existing.auto_create_events);

    sqlx::query(
        "UPDATE calendar_provider_accounts
         SET target_calendar_name = ?,
             auto_create_events = ?,
             updated_at = ?
         WHERE provider = ?",
    )
    .bind(target_calendar_name)
    .bind(if auto_create_events { 1 } else { 0 })
    .bind(&now)
    .bind(&provider)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update calendar write settings: {}", err))?;

    get_account(pool, &provider)
        .await?
        .ok_or_else(|| "Calendar settings were not saved".to_string())
}

#[tauri::command]
pub async fn sync_calendar_events(
    state: State<'_, AppState>,
    request: Option<CalendarSyncRequest>,
) -> Result<CalendarSyncResult, String> {
    let request = request.unwrap_or(CalendarSyncRequest {
        provider: None,
        lookback_days: None,
        lookahead_days: None,
    });
    let provider = normalize_provider(request.provider.as_deref().unwrap_or(PROVIDER_APPLE))?;
    let started_at = Utc::now();
    let pool = state.db_manager.pool();
    let account = match get_account(pool, &provider).await? {
        Some(account) => account,
        None => connect_provider_account(pool, &provider).await?,
    };

    let result = if provider == PROVIDER_APPLE {
        sync_apple_calendar(
            pool,
            &account.id,
            request.lookback_days.unwrap_or(1),
            request.lookahead_days.unwrap_or(14),
        )
        .await
    } else {
        Err("This calendar provider is not implemented yet.".to_string())
    };

    let completed_at = Utc::now();
    match result {
        Ok(count) => {
            update_account_sync(pool, &provider, "connected", Some(completed_at), None).await?;
            Ok(CalendarSyncResult {
                provider,
                status: "connected".to_string(),
                synced_event_count: count,
                started_at: started_at.to_rfc3339(),
                completed_at: completed_at.to_rfc3339(),
                error: None,
            })
        }
        Err(error) => {
            let status = sync_error_status(&provider, &error);
            update_account_sync(pool, &provider, status, Some(completed_at), Some(&error)).await?;
            Ok(CalendarSyncResult {
                provider,
                status: status.to_string(),
                synced_event_count: 0,
                started_at: started_at.to_rfc3339(),
                completed_at: completed_at.to_rfc3339(),
                error: Some(error),
            })
        }
    }
}

#[tauri::command]
pub async fn list_upcoming_calendar_events(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<CalendarEvent>, String> {
    let pool = state.db_manager.pool();
    let now = Utc::now().to_rfc3339();
    let limit = limit.unwrap_or(25).clamp(1, 100);
    let rows = sqlx::query(
        "SELECT id, provider, provider_event_id, calendar_source_id, title, starts_at, ends_at,
                timezone, location, meeting_url, meeting_provider, attendee_count,
                attendee_names_json, organizer_name, description_excerpt, content_hash,
                sync_status, updated_at
         FROM calendar_events
         WHERE sync_status = 'active' AND ends_at >= ?
         ORDER BY starts_at ASC
         LIMIT ?",
    )
    .bind(now)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list upcoming calendar events: {}", err))?;

    rows.into_iter().map(calendar_event_from_row).collect()
}

#[tauri::command]
pub async fn link_meeting_calendar_event(
    state: State<'_, AppState>,
    meeting_id: String,
    calendar_event_id: String,
    link_source: Option<String>,
    confidence: Option<f64>,
) -> Result<(), String> {
    let pool = state.db_manager.pool();
    let now = Utc::now().to_rfc3339();
    let link_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO meeting_calendar_links
            (id, meeting_id, calendar_event_id, link_source, confidence, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(meeting_id, calendar_event_id) DO UPDATE SET
            link_source = excluded.link_source,
            confidence = excluded.confidence,
            updated_at = excluded.updated_at",
    )
    .bind(link_id)
    .bind(meeting_id)
    .bind(calendar_event_id)
    .bind(link_source.unwrap_or_else(|| "selected_before_recording".to_string()))
    .bind(confidence)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to link calendar event: {}", err))?;
    Ok(())
}

#[tauri::command]
pub async fn create_or_update_meeting_calendar_event(
    state: State<'_, AppState>,
    request: CalendarEventCreationRequest,
) -> Result<CalendarEventCreationResult, String> {
    if !cfg!(target_os = "macos") {
        return Err("Apple Calendar event creation is available only on macOS.".to_string());
    }

    let pool = state.db_manager.pool();
    let account = match get_account(pool, PROVIDER_APPLE).await? {
        Some(account) if account.status == "connected" || account.status == "permission_needed" => {
            account
        }
        Some(_) => return Err("Reconnect Apple Calendar before creating events.".to_string()),
        None => return Err("Connect Apple Calendar before creating events.".to_string()),
    };
    if !account.auto_create_events {
        return Err(
            "Enable Apple Calendar event creation in Settings before creating events.".to_string(),
        );
    }

    let meeting = MeetingsRepository::get_meeting_metadata(pool, &request.meeting_id)
        .await
        .map_err(|err| {
            format!(
                "Failed to load meeting for calendar event creation: {}",
                err
            )
        })?
        .ok_or_else(|| "Meeting was not found for calendar event creation.".to_string())?;
    let summary = SummaryProcessesRepository::get_summary_data(pool, &request.meeting_id)
        .await
        .map_err(|err| {
            format!(
                "Failed to inspect summary for calendar event creation: {}",
                err
            )
        })?;
    let transcript_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?")
            .bind(&request.meeting_id)
            .fetch_one(pool)
            .await
            .map_err(|err| {
                format!(
                    "Failed to inspect transcript for calendar event creation: {}",
                    err
                )
            })?;

    let existing_link = latest_meeting_calendar_link(pool, &request.meeting_id).await?;
    let existing_apple_event_id = existing_link
        .as_ref()
        .and_then(|row| row.apple_event_identifier.as_deref());
    let starts_at = meeting.created_at.0;
    let mut ends_at = meeting.updated_at.0;
    if ends_at <= starts_at {
        ends_at = starts_at + Duration::minutes(30);
    }
    let summary_status = summary
        .as_ref()
        .map(|summary| summary.status.as_str())
        .unwrap_or("not_generated");
    let notes_export = latest_apple_notes_export(pool, &request.meeting_id).await?;
    let notes = calendar_event_notes(
        &meeting.id,
        summary_status,
        transcript_count > 0,
        notes_export.as_ref(),
    );
    let write = write_apple_calendar_event(
        existing_apple_event_id,
        &account.target_calendar_name,
        &meeting.title,
        starts_at,
        ends_at,
        &notes,
    )
    .map_err(|error| sanitize_calendar_write_error(&error))?;

    let source_id = ensure_source(
        pool,
        &account.id,
        &format!("apple-calendar-write-{}", write.calendar_name),
        &write.calendar_name,
        true,
        false,
    )
    .await?;
    let event_id = deterministic_id(&[
        PROVIDER_APPLE,
        &source_id,
        &write.apple_event_identifier,
        &starts_at.to_rfc3339(),
    ]);
    let event = CalendarEvent {
        id: event_id.clone(),
        provider: PROVIDER_APPLE.to_string(),
        provider_event_id: write.apple_event_identifier.clone(),
        calendar_source_id: source_id,
        title: meeting.title.clone(),
        starts_at: starts_at.to_rfc3339(),
        ends_at: ends_at.to_rfc3339(),
        timezone: None,
        location: None,
        meeting_url: None,
        meeting_provider: None,
        attendee_count: None,
        attendee_names: None,
        organizer_name: None,
        description_excerpt: Some("Created by Meetily.".to_string()),
        content_hash: content_hash(&[
            &write.apple_event_identifier,
            &meeting.title,
            &starts_at.to_rfc3339(),
            &ends_at.to_rfc3339(),
            &notes,
        ]),
        sync_status: "active".to_string(),
        updated_at: Utc::now().to_rfc3339(),
    };
    upsert_event(pool, &event).await?;
    upsert_meeting_calendar_link(
        pool,
        &request.meeting_id,
        &event_id,
        &write.apple_event_identifier,
        notes_export.as_ref().map(|export| export.id.as_str()),
    )
    .await?;
    update_account_sync(pool, PROVIDER_APPLE, "connected", Some(Utc::now()), None).await?;

    Ok(CalendarEventCreationResult {
        meeting_id: request.meeting_id,
        calendar_event_id: event_id,
        apple_event_identifier: write.apple_event_identifier,
        calendar_name: write.calendar_name,
        status: write.status,
    })
}

fn provider_infos() -> Vec<CalendarProviderInfo> {
    vec![
        CalendarProviderInfo {
            provider: PROVIDER_APPLE.to_string(),
            label: APPLE_ACCOUNT_LABEL.to_string(),
            available: cfg!(target_os = "macos"),
            supports_read: cfg!(target_os = "macos"),
            supports_write: cfg!(target_os = "macos"),
            notes: Some(
                if cfg!(target_os = "macos") {
                    "Reads Apple Calendar metadata and can create Meetily-owned events through the local macOS calendar bridge after explicit opt-in."
                } else {
                    "Apple Calendar is available only on macOS."
                }
                .to_string(),
            ),
        },
        CalendarProviderInfo {
            provider: "ics".to_string(),
            label: "ICS".to_string(),
            available: false,
            supports_read: false,
            supports_write: false,
            notes: Some("Planned read-only provider.".to_string()),
        },
        CalendarProviderInfo {
            provider: "google".to_string(),
            label: "Google Calendar".to_string(),
            available: false,
            supports_read: false,
            supports_write: false,
            notes: Some("Planned OAuth provider.".to_string()),
        },
    ]
}

async fn sync_apple_calendar(
    pool: &sqlx::SqlitePool,
    account_id: &str,
    lookback_days: i64,
    lookahead_days: i64,
) -> Result<usize, String> {
    if !cfg!(target_os = "macos") {
        return Err("Apple Calendar is not available on this platform.".to_string());
    }

    let source_id = ensure_source(
        pool,
        account_id,
        "apple-calendar",
        APPLE_ACCOUNT_LABEL,
        true,
        false,
    )
    .await?;
    let started_at = Utc::now();
    let script = apple_calendar_script(lookback_days, lookahead_days);
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|err| format!("Failed to run Apple Calendar sync: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "Apple Calendar permission is required before syncing events.".to_string()
        } else {
            stderr
        };
        return Err(message);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut synced = 0usize;
    for row in stdout
        .split('\u{1d}')
        .filter(|line| !line.trim().is_empty())
    {
        match parse_apple_calendar_row(row, &source_id) {
            Ok(Some(event)) => {
                upsert_event(pool, &event).await?;
                synced += 1;
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!("Skipping malformed Apple Calendar row: {}", error);
            }
        }
    }

    let cutoff = started_at - Duration::days(lookback_days.max(30));
    sqlx::query(
        "DELETE FROM calendar_events
         WHERE calendar_source_id = ? AND starts_at < ? AND id NOT IN (
            SELECT calendar_event_id FROM meeting_calendar_links
         )",
    )
    .bind(&source_id)
    .bind(cutoff.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to prune stale calendar events: {}", err))?;

    Ok(synced)
}

fn apple_calendar_script(lookback_days: i64, lookahead_days: i64) -> String {
    format!(
        r#"set rowDelimiter to character id 29
set fieldDelimiter to character id 30
set windowStart to (current date) - ({} * days)
set windowEnd to (current date) + ({} * days)
set rows to {{}}
tell application "Calendar"
    repeat with cal in calendars
        set calName to name of cal as text
        set calId to calName
        try
            set calId to uid of cal as text
        end try
        set eventList to every event of cal whose start date is greater than or equal to windowStart and start date is less than or equal to windowEnd
        repeat with ev in eventList
            set eventId to uid of ev as text
            set eventTitle to summary of ev as text
            set eventStart to my calendarDateText(start date of ev)
            set eventEnd to my calendarDateText(end date of ev)
            set eventLocation to ""
            set eventDescription to ""
            try
                set eventLocation to location of ev as text
            end try
            try
                set eventDescription to description of ev as text
            end try
            set eventTitle to my cleanCalendarField(eventTitle)
            set eventLocation to my cleanCalendarField(eventLocation)
            set eventDescription to my cleanCalendarField(eventDescription)
            copy (calId & fieldDelimiter & calName & fieldDelimiter & eventId & fieldDelimiter & eventTitle & fieldDelimiter & eventStart & fieldDelimiter & eventEnd & fieldDelimiter & eventLocation & fieldDelimiter & eventDescription) to end of rows
        end repeat
    end repeat
end tell
set AppleScript's text item delimiters to rowDelimiter
set outputRows to rows as text
set AppleScript's text item delimiters to ""
return outputRows

on calendarDateText(rawDate)
    set yearText to year of rawDate as text
    set monthText to my padCalendarNumber(month of rawDate as integer)
    set dayText to my padCalendarNumber(day of rawDate)
    set hourText to my padCalendarNumber(hours of rawDate)
    set minuteText to my padCalendarNumber(minutes of rawDate)
    set secondText to my padCalendarNumber(seconds of rawDate)
    return yearText & "-" & monthText & "-" & dayText & "T" & hourText & ":" & minuteText & ":" & secondText
end calendarDateText

on padCalendarNumber(rawNumber)
    set numberText to rawNumber as integer as text
    if (rawNumber as integer) is less than 10 then
        return "0" & numberText
    end if
    return numberText
end padCalendarNumber

on cleanCalendarField(rawValue)
    set cleaned to rawValue as text
    set AppleScript's text item delimiters to {{character id 29, character id 30, return, linefeed, tab}}
    set cleanedParts to text items of cleaned
    set AppleScript's text item delimiters to " "
    set cleaned to cleanedParts as text
    set AppleScript's text item delimiters to ""
    return cleaned
end cleanCalendarField"#,
        lookback_days.max(0),
        lookahead_days.clamp(1, 90)
    )
}

fn write_apple_calendar_event(
    existing_event_id: Option<&str>,
    target_calendar_name: &str,
    title: &str,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    notes: &str,
) -> Result<AppleCalendarWriteResult, String> {
    let start_text = apple_calendar_date_text(starts_at);
    let end_text = apple_calendar_date_text(ends_at);
    let script = apple_calendar_write_script();
    let output = Command::new("osascript")
        .args([
            "-e",
            &script,
            "--",
            existing_event_id.unwrap_or(""),
            target_calendar_name,
            title,
            &start_text,
            &end_text,
            notes,
        ])
        .output()
        .map_err(|err| format!("Failed to run Apple Calendar event creation: {}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "Apple Calendar could not create this event.".to_string()
        } else {
            stderr
        });
    }

    parse_calendar_write_result(String::from_utf8_lossy(&output.stdout).trim())
}

fn apple_calendar_date_text(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&Local)
        .format("%A, %B %-d, %Y at %-I:%M:%S %p")
        .to_string()
}

fn apple_calendar_write_script() -> String {
    r#"on run argv
    set existingEventId to item 1 of argv
    set targetCalendarName to item 2 of argv
    set eventTitle to item 3 of argv
    set startText to item 4 of argv
    set endText to item 5 of argv
    set eventDescription to item 6 of argv
    tell application "Calendar"
        set targetCalendar to missing value
        repeat with cal in calendars
            if (name of cal as text) is targetCalendarName then
                set targetCalendar to cal
                exit repeat
            end if
        end repeat
        if targetCalendar is missing value then
            set targetCalendar to make new calendar with properties {name:targetCalendarName}
        end if
        set targetEvent to missing value
        if existingEventId is not "" then
            try
                set matches to every event of targetCalendar whose uid is existingEventId
                if (count of matches) is greater than 0 then set targetEvent to item 1 of matches
            end try
        end if
        set writeStatus to "created"
        if targetEvent is missing value then
            set targetEvent to make new event at end of events of targetCalendar with properties {summary:eventTitle, start date:date startText, end date:date endText, description:eventDescription}
        else
            set summary of targetEvent to eventTitle
            set start date of targetEvent to date startText
            set end date of targetEvent to date endText
            set description of targetEvent to eventDescription
            set writeStatus to "updated"
        end if
        return (uid of targetEvent as text) & (character id 30) & (name of targetCalendar as text) & (character id 30) & writeStatus
    end tell
end run
"#
    .to_string()
}

fn parse_calendar_write_result(row: &str) -> Result<AppleCalendarWriteResult, String> {
    let parts = row.split('\u{1e}').collect::<Vec<_>>();
    if parts.len() < 3 {
        return Err("Apple Calendar did not return event metadata.".to_string());
    }
    let apple_event_identifier = parts[0].trim();
    if apple_event_identifier.is_empty() {
        return Err("Apple Calendar did not return an event identifier.".to_string());
    }
    Ok(AppleCalendarWriteResult {
        apple_event_identifier: apple_event_identifier.to_string(),
        calendar_name: if parts[1].trim().is_empty() {
            "Meetily"
        } else {
            parts[1].trim()
        }
        .to_string(),
        status: if parts[2].trim() == "updated" {
            "updated"
        } else {
            "created"
        }
        .to_string(),
    })
}

fn calendar_event_notes(
    meeting_id: &str,
    summary_status: &str,
    has_transcript: bool,
    notes_export: Option<&MeetingNotesExportLink>,
) -> String {
    let mut notes = format!(
        "Created by Meetily\n\nMeeting ID: {}\nSummary status: {}\nTranscript: {}",
        meeting_id,
        summary_status,
        if has_transcript {
            "available locally"
        } else {
            "not available"
        }
    );

    if let Some(export) = notes_export {
        notes.push_str("\n\nApple Notes export: ");
        notes.push_str(&export.status);
        notes.push_str("\nNote title: ");
        notes.push_str(&export.note_title);
        if let Some(folder_name) = export.folder_name.as_deref() {
            notes.push_str("\nNotes folder: ");
            notes.push_str(folder_name);
        }
        if let Some(provider_note_id) = export.provider_note_id.as_deref() {
            notes.push_str("\nApple Notes ID: ");
            notes.push_str(provider_note_id);
        }
    }

    notes
}

fn sanitize_calendar_write_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("-1743")
        || lower.contains("not authorized")
        || lower.contains("not permitted")
    {
        return "Apple Calendar permission is required. Allow Meetily to control Calendar in System Settings > Privacy & Security > Automation, then retry.".to_string();
    }
    let trimmed = error.trim();
    if trimmed.is_empty() {
        "Apple Calendar event creation failed. Check Calendar permissions and try again."
            .to_string()
    } else {
        trimmed.chars().take(280).collect()
    }
}

fn parse_apple_calendar_row(row: &str, source_id: &str) -> Result<Option<CalendarEvent>, String> {
    let parts = row.split('\u{1e}').collect::<Vec<_>>();
    if parts.len() < 8 {
        return Ok(None);
    }
    let provider_event_id = parts[2].trim();
    let title = parts[3].trim();
    let starts_at = parse_apple_datetime(parts[4].trim())?;
    let ends_at = parse_apple_datetime(parts[5].trim())?;
    let location = clean_optional(parts[6]);
    let description = clean_optional(parts[7]);
    let combined = format!(
        "{} {}",
        location.as_deref().unwrap_or(""),
        description.as_deref().unwrap_or("")
    );
    let meeting_url = extract_meeting_url(&combined);
    let meeting_provider = meeting_url.as_deref().and_then(meeting_provider_for_url);
    let description_excerpt = description
        .as_deref()
        .map(sanitize_description_excerpt)
        .filter(|value| !value.is_empty());
    let id = deterministic_id(&[PROVIDER_APPLE, source_id, provider_event_id, &starts_at]);
    let content_hash = content_hash(&[
        provider_event_id,
        source_id,
        title,
        &starts_at,
        &ends_at,
        location.as_deref().unwrap_or(""),
        meeting_url.as_deref().unwrap_or(""),
        meeting_provider.as_deref().unwrap_or(""),
        description_excerpt.as_deref().unwrap_or(""),
        "active",
    ]);

    Ok(Some(CalendarEvent {
        id,
        provider: PROVIDER_APPLE.to_string(),
        provider_event_id: provider_event_id.to_string(),
        calendar_source_id: source_id.to_string(),
        title: if title.is_empty() {
            "Untitled event"
        } else {
            title
        }
        .to_string(),
        starts_at,
        ends_at,
        timezone: None,
        location,
        meeting_url,
        meeting_provider,
        attendee_count: None,
        attendee_names: None,
        organizer_name: None,
        description_excerpt,
        content_hash,
        sync_status: "active".to_string(),
        updated_at: Utc::now().to_rfc3339(),
    }))
}

fn parse_apple_datetime(value: &str) -> Result<String, String> {
    if let Ok(date) = DateTime::parse_from_rfc3339(value) {
        return Ok(date.with_timezone(&Utc).to_rfc3339());
    }

    if let Ok(date) = DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%z") {
        return Ok(date.with_timezone(&Utc).to_rfc3339());
    }

    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| format!("Failed to parse Apple Calendar date '{}'", value))?;
    let local = Local
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| format!("Failed to resolve Apple Calendar date '{}'", value))?;
    Ok(local.with_timezone(&Utc).to_rfc3339())
}

async fn list_accounts(pool: &sqlx::SqlitePool) -> Result<Vec<CalendarProviderAccount>, String> {
    let rows = sqlx::query(
        "SELECT id, provider, account_label, status, last_sync_at, last_error,
                target_calendar_name, auto_create_events, created_at, updated_at
         FROM calendar_provider_accounts
         ORDER BY provider ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list calendar accounts: {}", err))?;

    rows.into_iter().map(account_from_row).collect()
}

async fn list_sources(pool: &sqlx::SqlitePool) -> Result<Vec<CalendarSource>, String> {
    let rows = sqlx::query(
        "SELECT id, provider_account_id, provider_calendar_id, name, color, selected, read_only,
                last_sync_at, created_at, updated_at
         FROM calendar_sources
         ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to list calendar sources: {}", err))?;

    rows.into_iter().map(source_from_row).collect()
}

async fn get_account(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<Option<CalendarProviderAccount>, String> {
    let row = sqlx::query(
        "SELECT id, provider, account_label, status, last_sync_at, last_error,
                target_calendar_name, auto_create_events, created_at, updated_at
         FROM calendar_provider_accounts WHERE provider = ?",
    )
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to get calendar account: {}", err))?;
    row.map(account_from_row).transpose()
}

async fn existing_account_id(
    pool: &sqlx::SqlitePool,
    provider: &str,
) -> Result<Option<String>, String> {
    sqlx::query("SELECT id FROM calendar_provider_accounts WHERE provider = ?")
        .bind(provider)
        .fetch_optional(pool)
        .await
        .map_err(|err| format!("Failed to get calendar account: {}", err))
        .map(|row| row.map(|row| row.get::<String, _>("id")))
}

async fn ensure_source(
    pool: &sqlx::SqlitePool,
    account_id: &str,
    provider_calendar_id: &str,
    name: &str,
    selected: bool,
    read_only: bool,
) -> Result<String, String> {
    let now = Utc::now().to_rfc3339();
    let existing = sqlx::query(
        "SELECT id FROM calendar_sources WHERE provider_account_id = ? AND provider_calendar_id = ?",
    )
    .bind(account_id)
    .bind(provider_calendar_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect calendar source: {}", err))?;
    let id = existing
        .map(|row| row.get::<String, _>("id"))
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        "INSERT INTO calendar_sources
            (id, provider_account_id, provider_calendar_id, name, selected, read_only, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(provider_account_id, provider_calendar_id) DO UPDATE SET
            name = excluded.name,
            selected = excluded.selected,
            read_only = excluded.read_only,
            updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(account_id)
    .bind(provider_calendar_id)
    .bind(name)
    .bind(selected as i64)
    .bind(read_only as i64)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save calendar source: {}", err))?;
    Ok(id)
}

async fn upsert_event(pool: &sqlx::SqlitePool, event: &CalendarEvent) -> Result<(), String> {
    let attendee_names_json = event
        .attendee_names
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|err| format!("Failed to serialize attendee names: {}", err))?;
    sqlx::query(
        "INSERT INTO calendar_events
            (id, provider, provider_event_id, calendar_source_id, title, starts_at, ends_at,
             timezone, location, meeting_url, meeting_provider, attendee_count, attendee_names_json,
             organizer_name, description_excerpt, content_hash, sync_status, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(provider, provider_event_id, calendar_source_id) DO UPDATE SET
            title = excluded.title,
            starts_at = excluded.starts_at,
            ends_at = excluded.ends_at,
            timezone = excluded.timezone,
            location = excluded.location,
            meeting_url = excluded.meeting_url,
            meeting_provider = excluded.meeting_provider,
            attendee_count = excluded.attendee_count,
            attendee_names_json = excluded.attendee_names_json,
            organizer_name = excluded.organizer_name,
            description_excerpt = excluded.description_excerpt,
            content_hash = excluded.content_hash,
            sync_status = excluded.sync_status,
            updated_at = excluded.updated_at",
    )
    .bind(&event.id)
    .bind(&event.provider)
    .bind(&event.provider_event_id)
    .bind(&event.calendar_source_id)
    .bind(&event.title)
    .bind(&event.starts_at)
    .bind(&event.ends_at)
    .bind(&event.timezone)
    .bind(&event.location)
    .bind(&event.meeting_url)
    .bind(&event.meeting_provider)
    .bind(event.attendee_count)
    .bind(attendee_names_json)
    .bind(&event.organizer_name)
    .bind(&event.description_excerpt)
    .bind(&event.content_hash)
    .bind(&event.sync_status)
    .bind(&event.updated_at)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save calendar event: {}", err))?;
    Ok(())
}

async fn latest_meeting_calendar_link(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Option<MeetingCalendarLinkRow>, String> {
    let row = sqlx::query(
        "SELECT id, calendar_event_id, apple_event_identifier
         FROM meeting_calendar_links
         WHERE meeting_id = ?
         ORDER BY updated_at DESC
         LIMIT 1",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect meeting calendar link: {}", err))?;
    Ok(row.map(|row| MeetingCalendarLinkRow {
        id: row.get("id"),
        apple_event_identifier: row.get("apple_event_identifier"),
    }))
}

async fn upsert_meeting_calendar_link(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    calendar_event_id: &str,
    apple_event_identifier: &str,
    notes_export_id: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    if let Some(existing) = latest_meeting_calendar_link(pool, meeting_id).await? {
        sqlx::query(
            "UPDATE meeting_calendar_links
             SET calendar_event_id = ?,
                 link_source = 'created_by_meetily',
                 confidence = 1.0,
                 apple_event_identifier = ?,
                 notes_export_id = COALESCE(?, notes_export_id),
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(calendar_event_id)
        .bind(apple_event_identifier)
        .bind(notes_export_id)
        .bind(&now)
        .bind(existing.id)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to update meeting calendar link: {}", err))?;
        return Ok(());
    }

    let link_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO meeting_calendar_links
            (id, meeting_id, calendar_event_id, link_source, confidence,
             apple_event_identifier, notes_export_id, created_at, updated_at)
         VALUES (?, ?, ?, 'created_by_meetily', 1.0, ?, ?, ?, ?)
         ON CONFLICT(meeting_id, calendar_event_id) DO UPDATE SET
             link_source = 'created_by_meetily',
             confidence = 1.0,
             apple_event_identifier = excluded.apple_event_identifier,
             notes_export_id = COALESCE(excluded.notes_export_id, meeting_calendar_links.notes_export_id),
             updated_at = excluded.updated_at",
    )
    .bind(link_id)
    .bind(meeting_id)
    .bind(calendar_event_id)
    .bind(apple_event_identifier)
    .bind(notes_export_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to save meeting calendar link: {}", err))?;
    Ok(())
}

async fn latest_apple_notes_export(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Option<MeetingNotesExportLink>, String> {
    let row = sqlx::query(
        "SELECT id, note_title, folder_name, provider_note_id, status
         FROM apple_notes_exports
         WHERE meeting_id = ? AND provider = 'apple_notes'
         ORDER BY COALESCE(exported_at, updated_at) DESC
         LIMIT 1",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to inspect Apple Notes export link: {}", err))?;

    Ok(row.map(|row| MeetingNotesExportLink {
        id: row.get("id"),
        note_title: row.get("note_title"),
        folder_name: row.get("folder_name"),
        provider_note_id: row.get("provider_note_id"),
        status: row.get("status"),
    }))
}

async fn update_account_sync(
    pool: &sqlx::SqlitePool,
    provider: &str,
    status: &str,
    synced_at: Option<DateTime<Utc>>,
    error: Option<&str>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE calendar_provider_accounts
         SET status = ?, last_sync_at = ?, last_error = ?, updated_at = ?
         WHERE provider = ?",
    )
    .bind(status)
    .bind(synced_at.map(|date| date.to_rfc3339()))
    .bind(error)
    .bind(Utc::now().to_rfc3339())
    .bind(provider)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to update calendar sync status: {}", err))?;
    Ok(())
}

fn account_from_row(row: sqlx::sqlite::SqliteRow) -> Result<CalendarProviderAccount, String> {
    let auto_create_events: i64 = row.get("auto_create_events");
    Ok(CalendarProviderAccount {
        id: row.get("id"),
        provider: row.get("provider"),
        account_label: row.get("account_label"),
        status: row.get("status"),
        last_sync_at: row.get("last_sync_at"),
        last_error: row.get("last_error"),
        target_calendar_name: row.get("target_calendar_name"),
        auto_create_events: auto_create_events != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn source_from_row(row: sqlx::sqlite::SqliteRow) -> Result<CalendarSource, String> {
    Ok(CalendarSource {
        id: row.get("id"),
        provider_account_id: row.get("provider_account_id"),
        provider_calendar_id: row.get("provider_calendar_id"),
        name: row.get("name"),
        color: row.get("color"),
        selected: row.get::<i64, _>("selected") != 0,
        read_only: row.get::<i64, _>("read_only") != 0,
        last_sync_at: row.get("last_sync_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn calendar_event_from_row(row: sqlx::sqlite::SqliteRow) -> Result<CalendarEvent, String> {
    let attendee_names_json: Option<String> = row.get("attendee_names_json");
    let attendee_names = attendee_names_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok());
    Ok(CalendarEvent {
        id: row.get("id"),
        provider: row.get("provider"),
        provider_event_id: row.get("provider_event_id"),
        calendar_source_id: row.get("calendar_source_id"),
        title: row.get("title"),
        starts_at: row.get("starts_at"),
        ends_at: row.get("ends_at"),
        timezone: row.get("timezone"),
        location: row.get("location"),
        meeting_url: row.get("meeting_url"),
        meeting_provider: row.get("meeting_provider"),
        attendee_count: row.get("attendee_count"),
        attendee_names,
        organizer_name: row.get("organizer_name"),
        description_excerpt: row.get("description_excerpt"),
        content_hash: row.get("content_hash"),
        sync_status: row.get("sync_status"),
        updated_at: row.get("updated_at"),
    })
}

fn normalize_provider(provider: &str) -> Result<String, String> {
    let provider = provider.trim().to_lowercase();
    match provider.as_str() {
        "apple" | "ics" | "google" => Ok(provider),
        _ => Err("Unsupported calendar provider".to_string()),
    }
}

fn sync_error_status(provider: &str, error: &str) -> &'static str {
    if provider != PROVIDER_APPLE {
        return "error";
    }

    let error = error.to_lowercase();
    if error.contains("not authorized")
        || error.contains("not allowed")
        || error.contains("permission")
        || error.contains("automation")
    {
        "permission_needed"
    } else {
        "error"
    }
}

fn provider_label(provider: &str) -> &'static str {
    match provider {
        PROVIDER_APPLE => APPLE_ACCOUNT_LABEL,
        "ics" => "ICS",
        "google" => "Google Calendar",
        _ => "Calendar",
    }
}

fn clean_optional(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value == "missing value" {
        None
    } else {
        Some(value.chars().take(500).collect())
    }
}

fn extract_meeting_url(text: &str) -> Option<String> {
    let re = Regex::new(r#"https?://[^\s<>()"']+"#).ok()?;
    let matched = re
        .find_iter(text)
        .filter_map(|candidate| sanitize_meeting_url(candidate.as_str()))
        .find(|url| meeting_provider_for_url(url).is_some());
    matched
}

fn sanitize_meeting_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim_end_matches(&['.', ',', ';', ')', ']'][..]);
    let mut url = Url::parse(trimmed).ok()?;
    url.set_fragment(None);
    let sensitive = [
        "pwd",
        "passcode",
        "password",
        "pin",
        "token",
        "tk",
        "signature",
    ];
    let filtered = url
        .query_pairs()
        .filter(|(key, _)| {
            let key = key.to_lowercase();
            !sensitive.iter().any(|term| key.contains(term))
        })
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<_>>();
    url.set_query(None);
    if !filtered.is_empty() {
        url.query_pairs_mut()
            .extend_pairs(filtered.iter().map(|(k, v)| (&**k, &**v)));
    }
    Some(url.to_string())
}

fn meeting_provider_for_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_lowercase();
    if host == "meet.google.com" {
        Some("google_meet".to_string())
    } else if host == "zoom.us" || host.ends_with(".zoom.us") {
        Some("zoom".to_string())
    } else if host == "teams.microsoft.com" {
        Some("teams".to_string())
    } else {
        None
    }
}

fn sanitize_description_excerpt(value: &str) -> String {
    let without_urls = Regex::new(r#"https?://[^\s<>()"']+"#)
        .map(|re| re.replace_all(value, "[link]").to_string())
        .unwrap_or_else(|_| value.to_string());
    without_urls
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}

fn deterministic_id(parts: &[&str]) -> String {
    format!("cal_{}", content_hash(parts))
}

fn content_hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_sanitizes_supported_meeting_url() {
        let text = "Join https://company.zoom.us/j/123456789?pwd=secret&from=addon";
        let url = extract_meeting_url(text).unwrap();
        assert_eq!(url, "https://company.zoom.us/j/123456789?from=addon");
        assert_eq!(meeting_provider_for_url(&url).as_deref(), Some("zoom"));
    }

    #[test]
    fn ignores_unsupported_urls() {
        assert!(extract_meeting_url("https://example.com/private").is_none());
    }

    #[test]
    fn description_excerpt_removes_raw_links() {
        let excerpt =
            sanitize_description_excerpt("Agenda https://meet.google.com/abc-defg-hij token");
        assert_eq!(excerpt, "Agenda [link] token");
    }

    #[test]
    fn apple_calendar_script_formats_dates_as_text() {
        let script = apple_calendar_script(0, 7);
        assert!(script.contains("calendarDateText(start date of ev)"));
        assert!(script.contains("calendarDateText(end date of ev)"));
        assert!(script.contains("on padCalendarNumber(rawNumber)"));
        assert!(!script.contains("class isot"));
    }

    #[test]
    fn deterministic_ids_are_stable() {
        assert_eq!(
            deterministic_id(&["apple", "source", "event"]),
            deterministic_id(&["apple", "source", "event"])
        );
        assert_ne!(
            deterministic_id(&["apple", "source", "event"]),
            deterministic_id(&["apple", "source", "other"])
        );
    }

    #[test]
    fn calendar_event_notes_include_apple_notes_export_metadata() {
        let notes_export = MeetingNotesExportLink {
            id: "export-1".to_string(),
            note_title: "2026-06-19 - Planning".to_string(),
            folder_name: Some("Meetily".to_string()),
            provider_note_id: Some("x-coredata://note/123".to_string()),
            status: "exported".to_string(),
        };
        let notes = calendar_event_notes("meeting-1", "completed", true, Some(&notes_export));

        assert!(notes.contains("Meeting ID: meeting-1"));
        assert!(notes.contains("Apple Notes export: exported"));
        assert!(notes.contains("Note title: 2026-06-19 - Planning"));
        assert!(notes.contains("Notes folder: Meetily"));
        assert!(notes.contains("Apple Notes ID: x-coredata://note/123"));
    }
}
