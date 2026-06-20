use crate::state::AppState;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

const SCREENSHOT_STORE: &str = "screenshot_preferences.json";
const SCREENSHOT_STORE_KEY: &str = "preferences";
const MIN_INTERVAL_SECONDS: u64 = 30;
const MAX_INTERVAL_SECONDS: u64 = 900;
const DEFAULT_INTERVAL_SECONDS: u64 = 60;
const DEFAULT_RETENTION_DAYS: u32 = 30;
const SCREENSHOT_RELEVANCE_THRESHOLD: f64 = 0.55;
const DEFAULT_CAPTURE_MODE: &str = "interval";
const EVENT_TRIGGER_MIN_GAP_SECONDS: i64 = 45;
const MAX_SCREENSHOTS_PER_MEETING: u32 = 240;

static CAPTURE_TASKS: Lazy<Mutex<HashMap<String, JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CAPTURE_RUNTIME: Lazy<Mutex<HashMap<String, Arc<Mutex<CaptureRuntimeState>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotPreferences {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub capture_target: String,
    pub capture_mode: String,
    pub retention_days: u32,
}

impl Default for ScreenshotPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: DEFAULT_INTERVAL_SECONDS,
            capture_target: default_capture_target(),
            capture_mode: DEFAULT_CAPTURE_MODE.to_string(),
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct CaptureRuntimeState {
    paused: bool,
    stopped: bool,
    last_capture_at: Option<DateTime<Utc>>,
    capture_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingScreenshot {
    pub id: String,
    pub meeting_id: String,
    pub captured_at: String,
    pub recording_time: Option<f64>,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub display_label: Option<String>,
    pub status: String,
    pub redaction_status: String,
    pub source: String,
    pub provider: Option<String>,
    pub relevance_confidence: Option<f64>,
    pub relevance_status: Option<String>,
    pub capture_trigger: Option<String>,
    pub speaker_evidence: bool,
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotCaptureStatus {
    pub meeting_id: String,
    pub active: bool,
    pub enabled: bool,
    pub interval_seconds: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ScreenshotAnalysis {
    is_relevant: bool,
    confidence: f64,
    provider: Option<String>,
    visible_names: Vec<String>,
    text_snippets: Vec<String>,
    relevance_status: String,
    skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ScreenshotWindowBounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CallWindowCaptureTarget {
    provider: String,
    app_name: Option<String>,
    window_title: Option<String>,
    window_id: Option<u32>,
    bounds: ScreenshotWindowBounds,
    checked_at: String,
    permission_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ScreenshotCapturePlan {
    capture_target: String,
    call_window: Option<CallWindowCaptureTarget>,
}

#[tauri::command]
pub async fn get_screenshot_preferences<R: Runtime>(
    app: AppHandle<R>,
) -> Result<ScreenshotPreferences, String> {
    load_screenshot_preferences(&app)
        .await
        .map_err(|err| format!("Failed to load screenshot preferences: {}", err))
}

#[tauri::command]
pub async fn set_screenshot_preferences<R: Runtime>(
    app: AppHandle<R>,
    preferences: ScreenshotPreferences,
) -> Result<ScreenshotPreferences, String> {
    let preferences = normalize_preferences(preferences);
    save_screenshot_preferences(&app, &preferences)
        .await
        .map_err(|err| format!("Failed to save screenshot preferences: {}", err))?;
    Ok(preferences)
}

#[tauri::command]
pub async fn start_meeting_screenshot_capture<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    recording_started_at: Option<String>,
) -> Result<ScreenshotCaptureStatus, String> {
    let preferences = load_screenshot_preferences(&app)
        .await
        .map_err(|err| format!("Failed to load screenshot preferences: {}", err))?;
    prune_expired_meeting_screenshots(state.db_manager.pool(), preferences.retention_days).await?;

    if !preferences.enabled {
        return Ok(ScreenshotCaptureStatus {
            meeting_id,
            active: false,
            enabled: false,
            interval_seconds: preferences.interval_seconds,
            last_error: None,
        });
    }

    stop_capture_task(&meeting_id);

    let runtime_state = Arc::new(Mutex::new(CaptureRuntimeState::default()));
    CAPTURE_RUNTIME
        .lock()
        .map_err(|_| "Failed to access screenshot scheduler registry".to_string())?
        .insert(meeting_id.clone(), runtime_state.clone());

    if preferences.capture_mode == "manualOnly" {
        return Ok(ScreenshotCaptureStatus {
            meeting_id,
            active: false,
            enabled: true,
            interval_seconds: preferences.interval_seconds,
            last_error: None,
        });
    }

    let app_for_task = app.clone();
    let db_manager = state.db_manager.clone();
    let meeting_for_task = meeting_id.clone();
    let recording_started_at = parse_recording_started_at(recording_started_at.as_deref());
    let fallback_preferences = preferences.clone();

    let handle = tokio::spawn(async move {
        loop {
            if is_capture_stopped(&runtime_state) {
                break;
            }
            let next_preferences = match load_screenshot_preferences(&app_for_task).await {
                Ok(value) => value,
                Err(err) => {
                    log::warn!("Failed to reload screenshot preferences: {}", err);
                    fallback_preferences.clone()
                }
            };
            if next_preferences.enabled
                && next_preferences.capture_mode != "manualOnly"
                && reserve_capture_slot(&runtime_state, Utc::now(), 0).unwrap_or(false)
            {
                if let Err(err) = capture_and_store_screenshot(
                    &app_for_task,
                    db_manager.pool(),
                    &meeting_for_task,
                    recording_started_at,
                    "interval",
                )
                .await
                {
                    log::warn!(
                        "Periodic screenshot capture failed for meeting {}: {}",
                        meeting_for_task,
                        err
                    );
                } else if let Err(err) = mark_capture_completed(&runtime_state) {
                    log::warn!("Failed to update screenshot scheduler count: {}", err);
                }
            }

            sleep(Duration::from_secs(next_preferences.interval_seconds)).await;
        }
    });

    CAPTURE_TASKS
        .lock()
        .map_err(|_| "Failed to access screenshot task registry".to_string())?
        .insert(meeting_id.clone(), handle);

    Ok(ScreenshotCaptureStatus {
        meeting_id,
        active: true,
        enabled: true,
        interval_seconds: preferences.interval_seconds,
        last_error: None,
    })
}

#[tauri::command]
pub async fn stop_meeting_screenshot_capture(
    meeting_id: String,
) -> Result<ScreenshotCaptureStatus, String> {
    let active = stop_capture_task(&meeting_id);
    Ok(ScreenshotCaptureStatus {
        meeting_id,
        active: false,
        enabled: active,
        interval_seconds: DEFAULT_INTERVAL_SECONDS,
        last_error: None,
    })
}

#[tauri::command]
pub async fn pause_meeting_screenshot_capture(
    meeting_id: String,
) -> Result<ScreenshotCaptureStatus, String> {
    let active = set_capture_paused(&meeting_id, true)?;
    Ok(ScreenshotCaptureStatus {
        meeting_id,
        active: false,
        enabled: active,
        interval_seconds: DEFAULT_INTERVAL_SECONDS,
        last_error: None,
    })
}

#[tauri::command]
pub async fn resume_meeting_screenshot_capture(
    meeting_id: String,
) -> Result<ScreenshotCaptureStatus, String> {
    let active = set_capture_paused(&meeting_id, false)?;
    Ok(ScreenshotCaptureStatus {
        meeting_id,
        active,
        enabled: active,
        interval_seconds: DEFAULT_INTERVAL_SECONDS,
        last_error: None,
    })
}

#[tauri::command]
pub async fn trigger_meeting_screenshot_capture<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    recording_started_at: Option<String>,
    trigger_reason: Option<String>,
) -> Result<ScreenshotCaptureStatus, String> {
    let preferences = load_screenshot_preferences(&app)
        .await
        .map_err(|err| format!("Failed to load screenshot preferences: {}", err))?;
    if !preferences.enabled || preferences.capture_mode != "speechEvent" {
        return Ok(ScreenshotCaptureStatus {
            meeting_id,
            active: false,
            enabled: preferences.enabled,
            interval_seconds: preferences.interval_seconds,
            last_error: None,
        });
    }

    let runtime_state = {
        let states = CAPTURE_RUNTIME
            .lock()
            .map_err(|_| "Failed to access screenshot scheduler registry".to_string())?;
        states.get(&meeting_id).cloned()
    };

    let Some(runtime_state) = runtime_state else {
        return Ok(ScreenshotCaptureStatus {
            meeting_id,
            active: false,
            enabled: true,
            interval_seconds: preferences.interval_seconds,
            last_error: None,
        });
    };

    if !reserve_capture_slot(&runtime_state, Utc::now(), EVENT_TRIGGER_MIN_GAP_SECONDS)? {
        return Ok(ScreenshotCaptureStatus {
            meeting_id,
            active: true,
            enabled: true,
            interval_seconds: preferences.interval_seconds,
            last_error: None,
        });
    }

    let trigger_reason = normalize_trigger_reason(trigger_reason.as_deref());
    let recording_started_at = parse_recording_started_at(recording_started_at.as_deref());
    let last_error = match capture_and_store_screenshot(
        &app,
        state.db_manager.pool(),
        &meeting_id,
        recording_started_at,
        &trigger_reason,
    )
    .await
    {
        Ok(_) => {
            mark_capture_completed(&runtime_state)?;
            None
        }
        Err(err) => Some(err),
    };

    Ok(ScreenshotCaptureStatus {
        meeting_id,
        active: true,
        enabled: true,
        interval_seconds: preferences.interval_seconds,
        last_error,
    })
}

#[tauri::command]
pub async fn capture_meeting_screenshot_now<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    recording_started_at: Option<String>,
) -> Result<MeetingScreenshot, String> {
    let recording_started_at = parse_recording_started_at(recording_started_at.as_deref());
    capture_and_store_screenshot(
        &app,
        state.db_manager.pool(),
        &meeting_id,
        recording_started_at,
        "manual",
    )
    .await
    .and_then(|screenshot| {
        screenshot.ok_or_else(|| {
            "Captured screen did not appear to contain an active meeting window".to_string()
        })
    })
}

#[tauri::command]
pub async fn list_meeting_screenshots(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<MeetingScreenshot>, String> {
    load_meeting_screenshots(state.db_manager.pool(), &meeting_id).await
}

#[tauri::command]
pub async fn delete_meeting_screenshot(
    state: tauri::State<'_, AppState>,
    screenshot_id: String,
    delete_file: Option<bool>,
    remove_metadata: Option<bool>,
) -> Result<(), String> {
    let should_delete_file = delete_file.unwrap_or(true);
    let should_remove_metadata = remove_metadata.unwrap_or(true);
    if !should_delete_file {
        return Err(
            "Screenshot deletion must remove the image file to avoid orphaned meeting artifacts"
                .to_string(),
        );
    }

    let pool = state.db_manager.pool();
    let row = sqlx::query(
        r#"
        SELECT file_path, thumbnail_path, metadata_json
        FROM meeting_screenshots
        WHERE id = ?
        "#,
    )
    .bind(&screenshot_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load screenshot: {}", err))?;

    let Some(row) = row else {
        return Ok(());
    };

    let file_path: Option<String> = row.try_get("file_path").ok();
    remove_optional_file(file_path.as_deref())
        .map_err(|err| format!("Failed to remove screenshot file: {}", err))?;
    let thumbnail_path: Option<String> = row.try_get("thumbnail_path").ok();
    remove_optional_file(thumbnail_path.as_deref())
        .map_err(|err| format!("Failed to remove screenshot thumbnail: {}", err))?;

    let now = Utc::now().to_rfc3339();
    if should_remove_metadata {
        sqlx::query(
            r#"
            DELETE FROM meeting_screenshots
            WHERE id = ?
            "#,
        )
        .bind(&screenshot_id)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to delete screenshot metadata: {}", err))?;
    } else {
        sqlx::query(
            r#"
            UPDATE meeting_screenshots
            SET status = 'deleted',
                file_path = NULL,
                thumbnail_path = NULL,
                redaction_status = 'image_removed',
                metadata_json = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(screenshot_metadata_after_image_removal(
            row.try_get("metadata_json").ok(),
            &now,
        ))
        .bind(&now)
        .bind(&screenshot_id)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to update screenshot metadata: {}", err))?;
    }

    Ok(())
}

fn remove_optional_file(file_path: Option<&str>) -> std::io::Result<()> {
    let Some(file_path) = file_path.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };
    match std::fs::remove_file(file_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn screenshot_metadata_after_image_removal(
    metadata_json: Option<String>,
    removed_at: &str,
) -> String {
    let metadata = metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .unwrap_or_else(|| json!({}));
    let analysis = metadata.get("analysis");
    let provider = metadata
        .get("provider")
        .cloned()
        .or_else(|| analysis.and_then(|value| value.get("provider")).cloned());
    let confidence = analysis
        .and_then(|value| value.get("confidence"))
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let relevance_status = analysis
        .and_then(|value| value.get("relevanceStatus"))
        .and_then(|value| value.as_str())
        .unwrap_or("imageRemoved");
    let removal_reason = "Screenshot image was removed by the user";

    json!({
        "analysis": {
            "isRelevant": false,
            "confidence": confidence,
            "provider": provider.clone().unwrap_or(serde_json::Value::Null),
            "visibleNames": [],
            "textSnippets": [],
            "relevanceStatus": relevance_status,
            "skipReason": removal_reason,
        },
        "captureTarget": metadata.get("captureTarget").cloned().unwrap_or(serde_json::Value::Null),
        "provider": provider.unwrap_or(serde_json::Value::Null),
        "sourceTrigger": metadata.get("sourceTrigger").cloned().unwrap_or(serde_json::Value::Null),
        "recordingTime": metadata.get("recordingTime").cloned().unwrap_or(serde_json::Value::Null),
        "imageRemovedAt": removed_at,
        "imageRemovedByUser": true,
        "skipReason": removal_reason,
    })
    .to_string()
}

#[tauri::command]
pub async fn attach_meeting_screenshots(
    state: tauri::State<'_, AppState>,
    from_meeting_id: String,
    to_meeting_id: String,
) -> Result<u64, String> {
    if from_meeting_id.trim().is_empty() || to_meeting_id.trim().is_empty() {
        return Err("Meeting IDs are required".to_string());
    }

    if from_meeting_id == to_meeting_id {
        return Ok(0);
    }

    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"
        UPDATE meeting_screenshots
        SET meeting_id = ?, updated_at = ?
        WHERE meeting_id = ? AND deleted_at IS NULL
        "#,
    )
    .bind(&to_meeting_id)
    .bind(&now)
    .bind(&from_meeting_id)
    .execute(state.db_manager.pool())
    .await
    .map_err(|err| format!("Failed to attach screenshots to saved meeting: {}", err))?;

    Ok(result.rows_affected())
}

pub async fn load_screenshot_preferences<R: Runtime>(
    app: &AppHandle<R>,
) -> anyhow::Result<ScreenshotPreferences> {
    let store = app.store(SCREENSHOT_STORE)?;
    let Some(value) = store.get(SCREENSHOT_STORE_KEY) else {
        return Ok(ScreenshotPreferences::default());
    };
    let preferences = serde_json::from_value::<ScreenshotPreferences>(value.clone())?;
    Ok(normalize_preferences(preferences))
}

pub async fn save_screenshot_preferences<R: Runtime>(
    app: &AppHandle<R>,
    preferences: &ScreenshotPreferences,
) -> anyhow::Result<()> {
    let store = app.store(SCREENSHOT_STORE)?;
    store.set(SCREENSHOT_STORE_KEY, serde_json::to_value(preferences)?);
    store.save()?;
    Ok(())
}

async fn capture_and_store_screenshot<R: Runtime>(
    app: &AppHandle<R>,
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    recording_started_at: Option<DateTime<Utc>>,
    source_trigger: &str,
) -> Result<Option<MeetingScreenshot>, String> {
    let preferences = load_screenshot_preferences(app)
        .await
        .map_err(|err| format!("Failed to load screenshot preferences: {}", err))?;
    let screenshot_id = Uuid::new_v4().to_string();
    let captured_at = Utc::now();
    let recording_time = recording_started_at
        .map(|started_at| (captured_at - started_at).num_milliseconds() as f64 / 1000.0)
        .filter(|seconds| *seconds >= 0.0);
    let display_label = recording_time.map(|seconds| format_time_label(seconds));
    let captured_at_string = captured_at.to_rfc3339();

    let capture_plan = match build_capture_plan(&preferences).await {
        Ok(plan) => plan,
        Err(err) => {
            store_skipped_screenshot(
                pool,
                SkippedScreenshotRecord {
                    screenshot_id,
                    meeting_id,
                    captured_at: &captured_at_string,
                    recording_time,
                    display_label: display_label.as_deref(),
                    source_trigger,
                    capture_target: &preferences.capture_target,
                    provider: None,
                    window_title: None,
                    window_id: None,
                    window_bounds: None,
                    confidence: None,
                    status: "skipped",
                    relevance_status: "skipped",
                    skip_reason: &err,
                },
            )
            .await?;
            return Ok(None);
        }
    };
    let file_path = screenshot_file_path(app, meeting_id, &screenshot_id, captured_at)
        .map_err(|err| format!("Failed to prepare screenshot folder: {}", err))?;

    if let Err(err) = capture_screen_to_file(&file_path, &capture_plan).await {
        if let Err(remove_err) = std::fs::remove_file(&file_path) {
            if remove_err.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "Failed to remove failed screenshot file {}: {}",
                    file_path.display(),
                    remove_err
                );
            }
        }
        store_skipped_screenshot(
            pool,
            SkippedScreenshotRecord {
                screenshot_id,
                meeting_id,
                captured_at: &captured_at_string,
                recording_time,
                display_label: display_label.as_deref(),
                source_trigger,
                capture_target: &capture_plan.capture_target,
                provider: capture_plan
                    .call_window
                    .as_ref()
                    .map(|target| target.provider.as_str()),
                window_title: capture_plan
                    .call_window
                    .as_ref()
                    .and_then(|target| target.window_title.as_deref()),
                window_id: capture_plan
                    .call_window
                    .as_ref()
                    .and_then(|target| target.window_id),
                window_bounds: capture_plan
                    .call_window
                    .as_ref()
                    .map(|target| &target.bounds),
                confidence: None,
                status: "failed",
                relevance_status: "failed",
                skip_reason: &err,
            },
        )
        .await?;
        return Ok(None);
    }

    let analysis = analyze_screenshot(&file_path).await;
    if !analysis.is_relevant {
        let skip_reason = analysis
            .skip_reason
            .clone()
            .unwrap_or_else(|| "Screenshot did not look like a supported meeting UI".to_string());
        if let Err(err) = std::fs::remove_file(&file_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "Failed to remove irrelevant screenshot {}: {}",
                    file_path.display(),
                    err
                );
            }
        }
        store_skipped_screenshot(
            pool,
            SkippedScreenshotRecord {
                screenshot_id,
                meeting_id,
                captured_at: &captured_at_string,
                recording_time,
                display_label: display_label.as_deref(),
                source_trigger,
                capture_target: &capture_plan.capture_target,
                provider: analysis.provider.as_deref().or_else(|| {
                    capture_plan
                        .call_window
                        .as_ref()
                        .map(|target| target.provider.as_str())
                }),
                window_title: capture_plan
                    .call_window
                    .as_ref()
                    .and_then(|target| target.window_title.as_deref()),
                window_id: capture_plan
                    .call_window
                    .as_ref()
                    .and_then(|target| target.window_id),
                window_bounds: capture_plan
                    .call_window
                    .as_ref()
                    .map(|target| &target.bounds),
                confidence: Some(analysis.confidence),
                status: "skipped",
                relevance_status: &analysis.relevance_status,
                skip_reason: &skip_reason,
            },
        )
        .await?;
        return Ok(None);
    }

    let now = Utc::now().to_rfc3339();
    let file_path_string = file_path.to_string_lossy().to_string();

    let metadata_json = json!({
        "analysis": analysis,
        "captureTarget": capture_plan.capture_target,
        "provider": capture_plan
            .call_window
            .as_ref()
            .map(|target| target.provider.clone()),
        "windowTitle": capture_plan
            .call_window
            .as_ref()
            .and_then(|target| target.window_title.clone()),
        "windowId": capture_plan
            .call_window
            .as_ref()
            .and_then(|target| target.window_id),
        "windowBounds": capture_plan
            .call_window
            .as_ref()
            .map(|target| target.bounds.clone()),
        "sourceTrigger": source_trigger,
        "recordingTime": recording_time,
    })
    .to_string();

    sqlx::query(
        r#"
        INSERT INTO meeting_screenshots
            (id, meeting_id, captured_at, recording_time, file_path, display_label, status, redaction_status, source, created_at, updated_at, metadata_json)
        VALUES (?, ?, ?, ?, ?, ?, 'captured', 'not_available', ?, ?, ?, ?)
        "#,
    )
    .bind(&screenshot_id)
    .bind(meeting_id)
    .bind(&captured_at_string)
    .bind(recording_time)
    .bind(&file_path_string)
    .bind(&display_label)
    .bind(source_trigger)
    .bind(&now)
    .bind(&now)
    .bind(&metadata_json)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to store screenshot metadata: {}", err))?;

    Ok(Some(MeetingScreenshot {
        id: screenshot_id,
        meeting_id: meeting_id.to_string(),
        captured_at: captured_at_string,
        recording_time,
        file_path: Some(file_path_string),
        thumbnail_path: None,
        display_label,
        status: "captured".to_string(),
        redaction_status: "not_available".to_string(),
        source: source_trigger.to_string(),
        provider: analysis.provider,
        relevance_confidence: Some(analysis.confidence),
        relevance_status: Some(analysis.relevance_status),
        capture_trigger: Some(source_trigger.to_string()),
        speaker_evidence: !analysis.visible_names.is_empty(),
        skip_reason: None,
    }))
}

struct SkippedScreenshotRecord<'a> {
    screenshot_id: String,
    meeting_id: &'a str,
    captured_at: &'a str,
    recording_time: Option<f64>,
    display_label: Option<&'a str>,
    source_trigger: &'a str,
    capture_target: &'a str,
    provider: Option<&'a str>,
    window_title: Option<&'a str>,
    window_id: Option<u32>,
    window_bounds: Option<&'a ScreenshotWindowBounds>,
    confidence: Option<f64>,
    status: &'a str,
    relevance_status: &'a str,
    skip_reason: &'a str,
}

async fn store_skipped_screenshot(
    pool: &sqlx::SqlitePool,
    record: SkippedScreenshotRecord<'_>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let metadata_json = json!({
        "analysis": {
            "isRelevant": false,
            "confidence": record.confidence.unwrap_or(0.0),
            "provider": record.provider,
            "visibleNames": [],
            "textSnippets": [],
            "relevanceStatus": record.relevance_status,
            "skipReason": record.skip_reason,
        },
        "captureTarget": record.capture_target,
        "provider": record.provider,
        "windowTitle": record.window_title,
        "windowId": record.window_id,
        "windowBounds": record.window_bounds,
        "sourceTrigger": record.source_trigger,
        "recordingTime": record.recording_time,
        "skipReason": record.skip_reason,
    })
    .to_string();

    sqlx::query(
        r#"
        INSERT INTO meeting_screenshots
            (id, meeting_id, captured_at, recording_time, file_path, display_label, status, redaction_status, source, created_at, updated_at, metadata_json)
        VALUES (?, ?, ?, ?, NULL, ?, ?, 'not_available', ?, ?, ?, ?)
        "#,
    )
    .bind(&record.screenshot_id)
    .bind(record.meeting_id)
    .bind(record.captured_at)
    .bind(record.recording_time)
    .bind(record.display_label)
    .bind(record.status)
    .bind(record.source_trigger)
    .bind(&now)
    .bind(&now)
    .bind(&metadata_json)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to store skipped screenshot metadata: {}", err))?;

    Ok(())
}

async fn analyze_screenshot(path: &Path) -> ScreenshotAnalysis {
    let path = path.to_path_buf();
    let recognized_text =
        match tokio::task::spawn_blocking(move || recognize_text_in_image(&path)).await {
            Ok(Ok(text)) => text,
            Ok(Err(err)) => {
                log::warn!("Screenshot OCR failed: {}", err);
                Vec::new()
            }
            Err(err) => {
                log::warn!("Screenshot OCR task failed: {}", err);
                Vec::new()
            }
        };

    analyze_recognized_text(&recognized_text)
}

fn analyze_recognized_text(recognized_text: &[String]) -> ScreenshotAnalysis {
    let provider = detect_meeting_provider(recognized_text);
    let visible_names = extract_visible_names(recognized_text);
    let sensitive_reason = detect_sensitive_frame_reason(recognized_text);
    let has_provider = provider.is_some();
    let has_visible_name = !visible_names.is_empty();
    let has_call_controls = recognized_text.iter().any(|text| {
        contains_any(
            &text.to_lowercase(),
            &["mute", "camera", "captions", "present", "leave call"],
        )
    });

    let mut confidence: f64 = match (has_provider, has_visible_name, has_call_controls) {
        (true, true, _) => 0.92,
        (true, false, true) => 0.78,
        (true, false, false) => 0.62,
        // Name + controls without a supported provider is ambiguous enough to record as skipped metadata.
        (false, true, true) => 0.48,
        _ => 0.0,
    };
    if sensitive_reason.is_some() {
        confidence = confidence.min(0.35);
    }
    let is_relevant = confidence >= SCREENSHOT_RELEVANCE_THRESHOLD && sensitive_reason.is_none();
    let relevance_status = if is_relevant {
        "kept"
    } else if confidence > 0.0 && sensitive_reason.is_none() {
        "needsReview"
    } else {
        "skipped"
    }
    .to_string();
    let skip_reason = if is_relevant {
        None
    } else {
        Some(sensitive_reason.unwrap_or_else(|| {
            if confidence > 0.0 {
                "Meeting UI confidence was too low; skipped for review".to_string()
            } else {
                "No supported meeting UI was detected".to_string()
            }
        }))
    };

    ScreenshotAnalysis {
        is_relevant,
        confidence,
        provider,
        visible_names,
        text_snippets: recognized_text
            .iter()
            .filter(|text| !text.trim().is_empty())
            .take(20)
            .cloned()
            .collect(),
        relevance_status,
        skip_reason,
    }
}

fn detect_sensitive_frame_reason(recognized_text: &[String]) -> Option<String> {
    let joined = recognized_text.join(" ").to_lowercase();
    if contains_any(
        &joined,
        &[
            "one-time code",
            "verification code",
            "api key",
            "secret key",
            "private key",
            "recovery code",
            "credit card",
        ],
    ) || joined
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .collect::<Vec<_>>()
        .windows(2)
        .any(|tokens| matches!(tokens, ["password", "field" | "reset" | "login"]))
    {
        Some("Sensitive private content was detected in the call window".to_string())
    } else {
        None
    }
}

fn detect_meeting_provider(recognized_text: &[String]) -> Option<String> {
    let joined = recognized_text.join(" ").to_lowercase();
    if contains_any(&joined, &["meet.google.com", "google meet"]) {
        Some("Google Meet".to_string())
    } else if contains_any(&joined, &["teams.microsoft.com", "microsoft teams"]) {
        Some("Microsoft Teams".to_string())
    } else if contains_any(&joined, &["zoom.us", "zoom meeting"]) {
        Some("Zoom".to_string())
    } else if contains_any(&joined, &["facetime"]) {
        Some("FaceTime".to_string())
    } else if contains_any(&joined, &["webex.com", "webex"]) {
        Some("Webex".to_string())
    } else {
        None
    }
}

fn extract_visible_names(recognized_text: &[String]) -> Vec<String> {
    let mut names = BTreeSet::new();
    for text in recognized_text {
        if let Some(name) = normalize_visible_name_candidate(text) {
            names.insert(name);
        }
    }
    names.into_iter().collect()
}

fn normalize_visible_name_candidate(value: &str) -> Option<String> {
    let cleaned = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphabetic() || ch.is_ascii_whitespace() || ch == '-' || ch == '\'' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if !(2..=4).contains(&tokens.len()) {
        return None;
    }

    let lower = cleaned.to_lowercase();
    if contains_any(
        &lower,
        &[
            "google meet",
            "microsoft teams",
            "new tab",
            "recording",
            "apple developer",
            "create new certificate",
            "application support",
            "captions",
            "connected",
            "mobility",
            "repository",
            "knowledge base",
            "certificates identifiers profiles",
        ],
    ) {
        return None;
    }

    let has_titlecase_token = tokens.iter().all(|token| {
        token
            .chars()
            .next()
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false)
    });
    if !has_titlecase_token {
        return None;
    }

    Some(cleaned.chars().take(64).collect())
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

#[cfg(target_os = "macos")]
fn recognize_text_in_image(path: &Path) -> Result<Vec<String>, String> {
    macos_vision_ocr::recognize_text(path)
}

#[cfg(not(target_os = "macos"))]
fn recognize_text_in_image(_path: &Path) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

async fn load_meeting_screenshots(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Vec<MeetingScreenshot>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, meeting_id, captured_at, recording_time, file_path, thumbnail_path,
               display_label, status, redaction_status, source, metadata_json
        FROM meeting_screenshots
        WHERE meeting_id = ?
        ORDER BY COALESCE(recording_time, 0), captured_at
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load screenshots: {}", err))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let metadata_json: Option<String> = row.try_get("metadata_json").ok();
            let metadata = metadata_json
                .as_deref()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
            let analysis = metadata.as_ref().and_then(|value| value.get("analysis"));

            MeetingScreenshot {
                id: row.get("id"),
                meeting_id: row.get("meeting_id"),
                captured_at: row.get("captured_at"),
                recording_time: row.try_get("recording_time").ok(),
                file_path: row.try_get("file_path").ok(),
                thumbnail_path: row.try_get("thumbnail_path").ok(),
                display_label: row.try_get("display_label").ok(),
                status: row.get("status"),
                redaction_status: row.get("redaction_status"),
                source: row.get("source"),
                provider: metadata
                    .as_ref()
                    .and_then(|value| value.get("provider"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        analysis
                            .and_then(|value| value.get("provider"))
                            .and_then(|value| value.as_str())
                            .map(str::to_string)
                    }),
                relevance_confidence: analysis
                    .and_then(|value| value.get("confidence"))
                    .and_then(|value| value.as_f64()),
                relevance_status: analysis
                    .and_then(|value| value.get("relevanceStatus"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                capture_trigger: metadata
                    .as_ref()
                    .and_then(|value| value.get("sourceTrigger"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| Some(row.get("source"))),
                speaker_evidence: row.get::<String, _>("status") == "captured"
                    && analysis
                        .and_then(|value| value.get("visibleNames"))
                        .and_then(|value| value.as_array())
                        .map(|names| !names.is_empty())
                        .unwrap_or(false),
                skip_reason: metadata
                    .as_ref()
                    .and_then(|value| value.get("skipReason"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        analysis
                            .and_then(|value| value.get("skipReason"))
                            .and_then(|value| value.as_str())
                            .map(str::to_string)
                    }),
            }
        })
        .collect())
}

async fn prune_expired_meeting_screenshots(
    pool: &sqlx::SqlitePool,
    retention_days: u32,
) -> Result<(), String> {
    let retention_days = retention_days.max(1);
    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff = cutoff.to_rfc3339();
    let rows = sqlx::query(
        r#"
        SELECT id, file_path, thumbnail_path
        FROM meeting_screenshots
        WHERE captured_at < ?
        "#,
    )
    .bind(&cutoff)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load expired screenshots: {}", err))?;

    for row in rows {
        let file_path: Option<String> = row.try_get("file_path").ok();
        remove_optional_file(file_path.as_deref())
            .map_err(|err| format!("Failed to remove expired screenshot file: {}", err))?;
        let thumbnail_path: Option<String> = row.try_get("thumbnail_path").ok();
        remove_optional_file(thumbnail_path.as_deref())
            .map_err(|err| format!("Failed to remove expired screenshot thumbnail: {}", err))?;
        let id: String = row.get("id");
        sqlx::query("DELETE FROM meeting_screenshots WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .map_err(|err| format!("Failed to delete expired screenshot metadata: {}", err))?;
    }

    Ok(())
}

fn stop_capture_task(meeting_id: &str) -> bool {
    let Ok(mut tasks) = CAPTURE_TASKS.lock() else {
        return false;
    };
    if let Ok(mut states) = CAPTURE_RUNTIME.lock() {
        if let Some(runtime_state) = states.remove(meeting_id) {
            if let Ok(mut state) = runtime_state.lock() {
                state.stopped = true;
            }
        }
    }
    tasks.remove(meeting_id).is_some()
}

fn is_capture_stopped(runtime_state: &Arc<Mutex<CaptureRuntimeState>>) -> bool {
    runtime_state
        .lock()
        .map(|state| state.stopped)
        .unwrap_or(true)
}

fn set_capture_paused(meeting_id: &str, paused: bool) -> Result<bool, String> {
    let states = CAPTURE_RUNTIME
        .lock()
        .map_err(|_| "Failed to access screenshot scheduler registry".to_string())?;
    let Some(runtime_state) = states.get(meeting_id) else {
        return Ok(false);
    };
    let mut state = runtime_state
        .lock()
        .map_err(|_| "Failed to update screenshot scheduler state".to_string())?;
    state.paused = paused;
    Ok(true)
}

fn reserve_capture_slot(
    runtime_state: &Arc<Mutex<CaptureRuntimeState>>,
    now: DateTime<Utc>,
    min_gap_seconds: i64,
) -> Result<bool, String> {
    let mut state = runtime_state
        .lock()
        .map_err(|_| "Failed to update screenshot scheduler state".to_string())?;
    Ok(should_reserve_capture_slot(
        &mut state,
        now,
        min_gap_seconds,
    ))
}

fn should_reserve_capture_slot(
    state: &mut CaptureRuntimeState,
    now: DateTime<Utc>,
    min_gap_seconds: i64,
) -> bool {
    if state.paused || state.capture_count >= MAX_SCREENSHOTS_PER_MEETING {
        return false;
    }
    if let Some(last_capture_at) = state.last_capture_at {
        if (now - last_capture_at).num_seconds() < min_gap_seconds {
            return false;
        }
    }
    state.last_capture_at = Some(now);
    true
}

fn mark_capture_completed(runtime_state: &Arc<Mutex<CaptureRuntimeState>>) -> Result<(), String> {
    let mut state = runtime_state
        .lock()
        .map_err(|_| "Failed to update screenshot scheduler state".to_string())?;
    state.capture_count = state.capture_count.saturating_add(1);
    Ok(())
}

fn normalize_trigger_reason(value: Option<&str>) -> String {
    match value {
        Some(reason @ ("speechEvent" | "speakerChange")) => reason.to_string(),
        Some(_) => "unknown".to_string(),
        _ => "unknown".to_string(),
    }
}

fn normalize_preferences(mut preferences: ScreenshotPreferences) -> ScreenshotPreferences {
    preferences.interval_seconds = preferences
        .interval_seconds
        .clamp(MIN_INTERVAL_SECONDS, MAX_INTERVAL_SECONDS);
    if preferences.capture_target != "fullScreen" && preferences.capture_target != "callWindow" {
        preferences.capture_target = "callWindow".to_string();
    }
    if !matches!(
        preferences.capture_mode.as_str(),
        "interval" | "speechEvent" | "manualOnly"
    ) {
        preferences.capture_mode = DEFAULT_CAPTURE_MODE.to_string();
    }
    if preferences.retention_days == 0 {
        preferences.retention_days = DEFAULT_RETENTION_DAYS;
    }
    preferences
}

fn default_capture_target() -> String {
    if cfg!(target_os = "macos") {
        "callWindow".to_string()
    } else {
        "fullScreen".to_string()
    }
}

fn parse_recording_started_at(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn screenshot_file_path<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
    screenshot_id: &str,
    captured_at: DateTime<Utc>,
) -> anyhow::Result<PathBuf> {
    let folder = app
        .path()
        .app_data_dir()?
        .join("artifacts")
        .join("meetings")
        .join(sanitize_path_segment(meeting_id))
        .join("screenshots");
    std::fs::create_dir_all(&folder)?;
    let filename = format!(
        "{}_{}.png",
        captured_at.format("%Y%m%d_%H%M%S"),
        screenshot_id
    );
    Ok(folder.join(filename))
}

async fn build_capture_plan(
    preferences: &ScreenshotPreferences,
) -> Result<ScreenshotCapturePlan, String> {
    if preferences.capture_target == "fullScreen" {
        return Ok(ScreenshotCapturePlan {
            capture_target: "fullScreen".to_string(),
            call_window: None,
        });
    }

    let call_window = detect_call_window_capture_target().await?;
    Ok(ScreenshotCapturePlan {
        capture_target: "callWindow".to_string(),
        call_window: Some(call_window),
    })
}

#[cfg(target_os = "macos")]
async fn detect_call_window_capture_target() -> Result<CallWindowCaptureTarget, String> {
    tokio::task::spawn_blocking(macos_detect_call_window_capture_target)
        .await
        .map_err(|err| format!("Call-window detection task failed: {}", err))?
}

#[cfg(not(target_os = "macos"))]
async fn detect_call_window_capture_target() -> Result<CallWindowCaptureTarget, String> {
    Err("Call-window screenshots are currently available on macOS only".to_string())
}

#[cfg(target_os = "macos")]
fn macos_detect_call_window_capture_target() -> Result<CallWindowCaptureTarget, String> {
    let checked_at = Utc::now().to_rfc3339();
    let output = run_osascript(
        r#"tell application "System Events"
set frontProcess to first application process whose frontmost is true
set frontWindow to front window of frontProcess
set windowPosition to position of frontWindow
set windowSize to size of frontWindow
return (name of frontProcess as text) & linefeed & (name of frontWindow as text) & linefeed & (item 1 of windowPosition as text) & "," & (item 2 of windowPosition as text) & "," & (item 1 of windowSize as text) & "," & (item 2 of windowSize as text)
end tell"#,
    )
    .map_err(|err| user_facing_call_window_error(&err))?
    .ok_or_else(|| "No active window is available for call-window capture".to_string())?;

    let mut lines = output.lines();
    let app_name = lines
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let window_title = lines
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let bounds_line = lines.next().ok_or_else(|| {
        "Active window bounds were unavailable; skipped call-window screenshot".to_string()
    })?;
    let bounds = parse_window_bounds(bounds_line).ok_or_else(|| {
        "Active window bounds were invalid; skipped call-window screenshot".to_string()
    })?;
    if !is_usable_window_bounds(&bounds) {
        return Err("Active meeting window bounds were too small or invalid; skipped call-window screenshot".to_string());
    }

    let active_tab_url = if app_name.map(is_supported_browser_app).unwrap_or(false) {
        Some(resolve_browser_active_tab_url(app_name.ok_or_else(|| {
            "Browser window metadata was unavailable; skipped call-window screenshot".to_string()
        })?)?)
    } else {
        None
    };
    let provider = detect_call_window_provider(app_name, window_title, active_tab_url.as_deref())
        .ok_or_else(|| {
        "No active supported meeting window was detected; skipped call-window screenshot"
            .to_string()
    })?;
    let window_id = resolve_cg_window_id(app_name, window_title).ok_or_else(|| {
        "Could not resolve active meeting window id; skipped call-window screenshot".to_string()
    })?;

    Ok(CallWindowCaptureTarget {
        provider,
        app_name: app_name.map(str::to_string),
        window_title: window_title.map(str::to_string),
        window_id: Some(window_id),
        bounds,
        checked_at,
        permission_status: "available".to_string(),
    })
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<Option<String>, String> {
    let output = std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|error| format!("osascript failed: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Ok(None);
        }
        return Err(stderr);
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if value.is_empty() || value == "missing value" {
        None
    } else {
        Some(value)
    })
}

fn parse_window_bounds(value: &str) -> Option<ScreenshotWindowBounds> {
    let values = value
        .split(',')
        .filter_map(|part| part.trim().parse::<i32>().ok())
        .collect::<Vec<_>>();
    if values.len() != 4 {
        return None;
    }
    Some(ScreenshotWindowBounds {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
    })
}

fn is_usable_window_bounds(bounds: &ScreenshotWindowBounds) -> bool {
    bounds.width >= 320 && bounds.height >= 180
}

fn detect_call_window_provider(
    app_name: Option<&str>,
    window_title: Option<&str>,
    active_tab_url: Option<&str>,
) -> Option<String> {
    let app = app_name.unwrap_or_default().to_lowercase();
    let title = window_title.unwrap_or_default().to_lowercase();
    let active_tab_url = active_tab_url.unwrap_or_default().to_lowercase();
    let is_browser = is_supported_browser_app(&app);

    if is_browser && is_browser_meeting_url(&active_tab_url, "googleMeet") {
        Some("googleMeet".to_string())
    } else if is_browser && is_browser_meeting_url(&active_tab_url, "zoom") {
        Some("zoom".to_string())
    } else if is_browser && is_browser_meeting_url(&active_tab_url, "teams") {
        Some("teams".to_string())
    } else if is_browser && is_browser_meeting_url(&active_tab_url, "webex") {
        Some("webex".to_string())
    } else if is_browser && is_browser_meeting_url(&active_tab_url, "slack") {
        Some("slack".to_string())
    } else if equals_any(&app, &["zoom.us", "zoom workplace", "zoom"])
        || contains_any(&title, &["zoom.us", "zoom meeting", "zoom workplace"])
    {
        Some("zoom".to_string())
    } else if equals_any(
        &app,
        &[
            "microsoft teams",
            "microsoft teams (work or school)",
            "teams",
        ],
    ) || contains_any(
        &title,
        &[
            "teams.microsoft.com",
            "teams.live.com",
            "microsoft teams",
            "teams meeting",
        ],
    ) {
        Some("teams".to_string())
    } else if equals_any(&app, &["facetime"]) {
        Some("facetime".to_string())
    } else if equals_any(&app, &["webex", "cisco webex"])
        || contains_any(&title, &["webex.com", "webex meeting"])
    {
        Some("webex".to_string())
    } else if equals_any(&app, &["slack"])
        && contains_any(&title, &["huddle", "call", "slack huddle", "slack call"])
    {
        Some("slack".to_string())
    } else {
        None
    }
}

fn is_browser_meeting_url(url: &str, provider: &str) -> bool {
    match provider {
        "googleMeet" => url.contains("meet.google.com/"),
        "zoom" => {
            (url.contains("zoom.us/j/") || url.contains("zoom.com/j/"))
                || url.contains("zoom.us/wc/")
                || url.contains("zoom.com/wc/")
        }
        "teams" => {
            contains_any(url, &["teams.microsoft.com/l/meetup-join", "teams.live.com/meet"])
        }
        "webex" => contains_any(url, &["webex.com/meet/", "webex.com/join/", ".webex.com/"]),
        "slack" => url.contains("slack.com/huddle"),
        _ => false,
    }
}

fn is_supported_browser_app(app_name: &str) -> bool {
    let app = app_name.to_lowercase();
    contains_any(
        &app,
        &[
            "google chrome",
            "chrome",
            "arc",
            "safari",
            "microsoft edge",
            "firefox",
        ],
    )
}

#[cfg(target_os = "macos")]
fn resolve_browser_active_tab_url(app_name: &str) -> Result<String, String> {
    let app_name = app_name.trim();
    let script = if app_name.eq_ignore_ascii_case("safari") {
        r#"tell application "Safari"
return URL of front document as text
end tell"#
        .to_string()
    } else {
        format!(
            r#"tell application "{}"
return URL of active tab of front window as text
end tell"#,
            app_name.replace('"', "")
        )
    };
    let url = run_osascript(&script)
        .map_err(|err| user_facing_call_window_error(&err))?
        .ok_or_else(|| "Active browser tab URL was unavailable; skipped call-window screenshot".to_string())?;
    if url.trim().is_empty() {
        return Err("Active browser tab URL was empty; skipped call-window screenshot".to_string());
    }
    Ok(url)
}

fn equals_any(value: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| value == *candidate)
}

#[cfg(target_os = "macos")]
fn resolve_cg_window_id(app_name: Option<&str>, window_title: Option<&str>) -> Option<u32> {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
    };

    let app_name = app_name.unwrap_or_default();
    let window_title = window_title.unwrap_or_default();
    let windows = copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)?;

    let number_key = CFString::from_static_string("kCGWindowNumber");
    let owner_key = CFString::from_static_string("kCGWindowOwnerName");
    let name_key = CFString::from_static_string("kCGWindowName");
    let layer_key = CFString::from_static_string("kCGWindowLayer");
    let onscreen_key = CFString::from_static_string("kCGWindowIsOnscreen");

    for raw_window in windows.get_all_values() {
        let dictionary =
            unsafe { CFDictionary::<CFString, CFType>::wrap_under_get_rule(raw_window as _) };
        let layer = dictionary
            .find(&layer_key)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|number| number.to_i32())
            .unwrap_or(0);
        if layer != 0 {
            continue;
        }
        let is_onscreen = dictionary
            .find(&onscreen_key)
            .and_then(|value| value.downcast::<CFBoolean>())
            .map(|value| value.into())
            .unwrap_or(true);
        if !is_onscreen {
            continue;
        }
        let owner = dictionary
            .find(&owner_key)
            .and_then(|value| value.downcast::<CFString>())
            .map(|value| value.to_string())
            .unwrap_or_default();
        let name = dictionary
            .find(&name_key)
            .and_then(|value| value.downcast::<CFString>())
            .map(|value| value.to_string())
            .unwrap_or_default();
        if !window_identity_matches(app_name, window_title, &owner, &name) {
            continue;
        }
        let Some(window_id) = dictionary
            .find(&number_key)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|number| number.to_i32())
            .and_then(|value| u32::try_from(value).ok())
        else {
            continue;
        };
        return Some(window_id);
    }

    None
}

#[cfg(not(target_os = "macos"))]
fn resolve_cg_window_id(_app_name: Option<&str>, _window_title: Option<&str>) -> Option<u32> {
    None
}

fn window_identity_matches(
    active_app: &str,
    active_title: &str,
    candidate_owner: &str,
    candidate_title: &str,
) -> bool {
    if !normalized_text_matches(active_app, candidate_owner) {
        return false;
    }
    if active_title.trim().is_empty() {
        return true;
    }
    normalized_text_matches(active_title, candidate_title)
}

fn normalized_text_matches(left: &str, right: &str) -> bool {
    let left = left.trim().to_lowercase();
    let right = right.trim().to_lowercase();
    !left.is_empty() && !right.is_empty() && left == right
}

fn user_facing_call_window_error(error: &str) -> String {
    let normalized = error.to_lowercase();
    if normalized.contains("not authorized")
        || normalized.contains("not permitted")
        || normalized.contains("privacy")
        || normalized.contains("accessibility")
        || normalized.contains("automation")
        || normalized.contains("-1743")
    {
        "Meetily needs macOS Accessibility permission to detect meeting window bounds before capturing call-window screenshots".to_string()
    } else {
        format!("Could not detect active meeting window bounds: {}", error)
    }
}

#[cfg(target_os = "macos")]
async fn capture_screen_to_file(
    path: &Path,
    capture_plan: &ScreenshotCapturePlan,
) -> Result<(), String> {
    let path = path.to_path_buf();
    let capture_target = capture_plan.capture_target.clone();
    let window_id = capture_plan
        .call_window
        .as_ref()
        .and_then(|target| target.window_id);
    tokio::task::spawn_blocking(move || {
        let mut command = std::process::Command::new("/usr/sbin/screencapture");
        command.arg("-x");
        if let Some(window_id) = window_id {
            command.arg("-o").arg("-l").arg(window_id.to_string());
        }
        let status = command
            .arg(&path)
            .status()
            .map_err(|err| format!("Failed to start macOS screenshot capture: {}", err))?;

        if status.success() {
            Ok(())
        } else if capture_target == "callWindow" {
            Err(format!(
                "Call-window screenshot failed with status {}. Screen Recording permission may be missing, or the meeting window may no longer be available.",
                status
            ))
        } else {
            Err(format!(
                "macOS screenshot capture failed with status {}",
                status
            ))
        }
    })
    .await
    .map_err(|err| format!("Screenshot capture task failed: {}", err))?
}

#[cfg(not(target_os = "macos"))]
async fn capture_screen_to_file(
    _path: &Path,
    _capture_plan: &ScreenshotCapturePlan,
) -> Result<(), String> {
    Err("Periodic screenshots are currently available on macOS only".to_string())
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect();
    if sanitized.is_empty() {
        "meeting".to_string()
    } else {
        sanitized
    }
}

fn format_time_label(seconds: f64) -> String {
    let total_seconds = seconds.round().max(0.0) as u64;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

#[cfg(target_os = "macos")]
mod macos_vision_ocr {
    use objc::runtime::{Object, BOOL, YES};
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::path::Path;

    type Id = *mut Object;

    #[link(name = "Foundation", kind = "framework")]
    extern "C" {}

    #[link(name = "Vision", kind = "framework")]
    extern "C" {}

    pub fn recognize_text(path: &Path) -> Result<Vec<String>, String> {
        let path = path
            .to_str()
            .ok_or_else(|| "Screenshot path is not valid UTF-8".to_string())?;
        let path =
            CString::new(path).map_err(|_| "Screenshot path contains NUL byte".to_string())?;

        unsafe {
            let pool: Id = msg_send![class!(NSAutoreleasePool), new];
            let result = recognize_text_inner(path.as_ptr());
            let _: () = msg_send![pool, drain];
            result
        }
    }

    unsafe fn recognize_text_inner(path: *const c_char) -> Result<Vec<String>, String> {
        let ns_path: Id = msg_send![class!(NSString), stringWithUTF8String: path];
        if ns_path.is_null() {
            return Err("Failed to create NSString for screenshot path".to_string());
        }

        let url: Id = msg_send![class!(NSURL), fileURLWithPath: ns_path];
        if url.is_null() {
            return Err("Failed to create file URL for screenshot".to_string());
        }

        let handler: Id = msg_send![class!(VNImageRequestHandler), alloc];
        let options: Id = msg_send![class!(NSDictionary), dictionary];
        let handler: Id = msg_send![handler, initWithURL: url options: options];
        if handler.is_null() {
            return Err("Failed to initialize Vision image handler".to_string());
        }

        let request: Id = msg_send![class!(VNRecognizeTextRequest), new];
        if request.is_null() {
            return Err("Failed to initialize Vision text request".to_string());
        }
        let _: () = msg_send![request, setUsesLanguageCorrection: YES];

        let requests: Id = msg_send![class!(NSArray), arrayWithObject: request];
        let mut error: Id = std::ptr::null_mut();
        let ok: BOOL = msg_send![handler, performRequests: requests error: &mut error];
        if !ok {
            return Err("Vision failed to recognize text in screenshot".to_string());
        }

        let observations: Id = msg_send![request, results];
        if observations.is_null() {
            return Ok(Vec::new());
        }

        let count: usize = msg_send![observations, count];
        let mut lines = Vec::new();
        for index in 0..count {
            let observation: Id = msg_send![observations, objectAtIndex: index];
            let candidates: Id = msg_send![observation, topCandidates: 1usize];
            if candidates.is_null() {
                continue;
            }
            let candidate_count: usize = msg_send![candidates, count];
            if candidate_count == 0 {
                continue;
            }
            let candidate: Id = msg_send![candidates, objectAtIndex: 0usize];
            let confidence: f32 = msg_send![candidate, confidence];
            if confidence < 0.35 {
                continue;
            }
            let string: Id = msg_send![candidate, string];
            if let Some(text) = nsstring_to_string(string) {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    lines.push(text);
                }
            }
        }

        Ok(lines)
    }

    unsafe fn nsstring_to_string(value: Id) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let bytes: *const c_char = msg_send![value, UTF8String];
        if bytes.is_null() {
            return None;
        }
        CStr::from_ptr(bytes).to_str().ok().map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preferences_are_opt_in() {
        let preferences = ScreenshotPreferences::default();
        assert!(!preferences.enabled);
        assert_eq!(preferences.interval_seconds, DEFAULT_INTERVAL_SECONDS);
        assert_eq!(preferences.capture_target, "callWindow");
        assert_eq!(preferences.capture_mode, DEFAULT_CAPTURE_MODE);
    }

    #[test]
    fn normalizes_preferences_to_supported_bounds() {
        let preferences = normalize_preferences(ScreenshotPreferences {
            enabled: true,
            interval_seconds: 5,
            capture_target: "activeWindow".to_string(),
            capture_mode: "unknown".to_string(),
            retention_days: 0,
        });

        assert!(preferences.enabled);
        assert_eq!(preferences.interval_seconds, MIN_INTERVAL_SECONDS);
        assert_eq!(preferences.capture_target, "callWindow");
        assert_eq!(preferences.capture_mode, DEFAULT_CAPTURE_MODE);
        assert_eq!(preferences.retention_days, DEFAULT_RETENTION_DAYS);
    }

    #[test]
    fn detects_supported_call_window_provider() {
        assert_eq!(
            detect_call_window_provider(
                Some("Google Chrome"),
                Some("Google Meet - standup"),
                Some("https://meet.google.com/abc-defg-hij")
            ),
            Some("googleMeet".to_string())
        );
        assert_eq!(
            detect_call_window_provider(
                Some("Google Chrome"),
                Some("Google Meet - standup"),
                Some("https://bank.example.com/login")
            ),
            None
        );
        assert_eq!(
            detect_call_window_provider(Some("Microsoft Teams"), Some("Weekly Sync"), None),
            Some("teams".to_string())
        );
        assert_eq!(
            detect_call_window_provider(Some("Zoom Workplace"), Some("Zoom Meeting"), None),
            Some("zoom".to_string())
        );
        assert_eq!(
            detect_call_window_provider(
                Some("Google Chrome"),
                Some("Zoom pricing page"),
                Some("https://zoom.us/pricing")
            ),
            None
        );
        assert_eq!(
            detect_call_window_provider(Some("Finder"), Some("Downloads"), None),
            None
        );
    }

    #[test]
    fn window_identity_requires_matching_owner_and_title_when_available() {
        assert!(window_identity_matches(
            "Google Chrome",
            "Google Meet - standup",
            "Google Chrome",
            "Google Meet - standup"
        ));
        assert!(!window_identity_matches(
            "Google Chrome",
            "Google Meet - standup",
            "Google Chrome",
            "Inbox"
        ));
        assert!(!window_identity_matches(
            "Google Chrome",
            "Google Meet - standup",
            "Google Chrome",
            ""
        ));
    }

    #[test]
    fn parses_and_validates_window_bounds() {
        let bounds = parse_window_bounds("24,48,1280,720").expect("bounds");
        assert_eq!(bounds.x, 24);
        assert_eq!(bounds.y, 48);
        assert_eq!(bounds.width, 1280);
        assert_eq!(bounds.height, 720);
        assert!(is_usable_window_bounds(&bounds));
        assert!(!is_usable_window_bounds(&ScreenshotWindowBounds {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        }));
        assert!(parse_window_bounds("not,bounds").is_none());
    }

    #[test]
    fn creates_safe_artifact_path_segment() {
        assert_eq!(
            sanitize_path_segment("../meeting id!"),
            "meetingid".to_string()
        );
        assert_eq!(sanitize_path_segment(""), "meeting".to_string());
    }

    #[test]
    fn formats_recording_time_label() {
        assert_eq!(format_time_label(0.0), "00:00");
        assert_eq!(format_time_label(65.2), "01:05");
    }

    #[test]
    fn scheduler_respects_pause_state_and_rate_limits() {
        let mut state = CaptureRuntimeState::default();
        let now = Utc::now();

        assert!(should_reserve_capture_slot(&mut state, now, 45));
        assert!(!should_reserve_capture_slot(
            &mut state,
            now + chrono::Duration::seconds(20),
            45
        ));
        assert!(should_reserve_capture_slot(
            &mut state,
            now + chrono::Duration::seconds(45),
            45
        ));

        state.paused = true;
        assert!(!should_reserve_capture_slot(
            &mut state,
            now + chrono::Duration::seconds(120),
            45
        ));
    }

    #[test]
    fn scheduler_stops_after_max_capture_count() {
        let mut state = CaptureRuntimeState {
            paused: false,
            stopped: false,
            last_capture_at: None,
            capture_count: MAX_SCREENSHOTS_PER_MEETING,
        };

        assert!(!should_reserve_capture_slot(&mut state, Utc::now(), 0));
    }

    #[test]
    fn screenshot_analysis_keeps_google_meet_with_visible_name() {
        let analysis = analyze_recognized_text(&[
            "meet.google.com/trv-nxib-ftd".to_string(),
            "Adrian Witaszak".to_string(),
            "Captions".to_string(),
        ]);

        assert!(analysis.is_relevant);
        assert_eq!(analysis.provider, Some("Google Meet".to_string()));
        assert_eq!(analysis.visible_names, vec!["Adrian Witaszak".to_string()]);
        assert_eq!(analysis.relevance_status, "kept");
        assert!(analysis.skip_reason.is_none());
    }

    #[test]
    fn screenshot_analysis_filters_non_call_screens() {
        let analysis = analyze_recognized_text(&[
            "Create a New Certificate".to_string(),
            "Apple Developer".to_string(),
            "Application Support".to_string(),
        ]);

        assert!(!analysis.is_relevant);
        assert!(analysis.visible_names.is_empty());
        assert_eq!(analysis.relevance_status, "skipped");
        assert_eq!(
            analysis.skip_reason.as_deref(),
            Some("No supported meeting UI was detected")
        );
    }

    #[test]
    fn screenshot_analysis_marks_low_confidence_meeting_frames_for_review() {
        let analysis = analyze_recognized_text(&[
            "Adrian Witaszak".to_string(),
            "Mute".to_string(),
            "Camera".to_string(),
        ]);

        assert!(!analysis.is_relevant);
        assert_eq!(analysis.relevance_status, "needsReview");
        assert_eq!(
            analysis.skip_reason.as_deref(),
            Some("Meeting UI confidence was too low; skipped for review")
        );
    }

    #[test]
    fn screenshot_analysis_skips_sensitive_frames_even_with_meeting_signals() {
        let analysis = analyze_recognized_text(&[
            "meet.google.com/trv-nxib-ftd".to_string(),
            "Adrian Witaszak".to_string(),
            "API key sk-live-123".to_string(),
        ]);

        assert!(!analysis.is_relevant);
        assert_eq!(analysis.relevance_status, "skipped");
        assert_eq!(
            analysis.skip_reason.as_deref(),
            Some("Sensitive private content was detected in the call window")
        );
    }

    #[test]
    fn image_removal_metadata_scrubs_ocr_names_and_text() {
        let metadata = json!({
            "analysis": {
                "isRelevant": true,
                "visibleNames": ["Adrian Witaszak"],
                "textSnippets": ["Adrian Witaszak", "Google Meet"],
                "relevanceStatus": "kept"
            },
            "provider": "Google Meet",
            "windowTitle": "Zoom Meeting - Adrian Witaszak / Acme Q3 Review"
        })
        .to_string();

        let sanitized =
            screenshot_metadata_after_image_removal(Some(metadata), "2026-06-20T09:00:00Z");
        let value: serde_json::Value = serde_json::from_str(&sanitized).expect("metadata json");

        assert_eq!(
            value
                .get("analysis")
                .and_then(|analysis| analysis.get("visibleNames"))
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(0)
        );
        assert_eq!(
            value
                .get("analysis")
                .and_then(|analysis| analysis.get("textSnippets"))
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(0)
        );
        assert_eq!(
            value
                .get("imageRemovedByUser")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert!(value.get("windowTitle").is_none());
    }

    #[test]
    fn visible_name_extraction_ignores_controls_and_project_text() {
        let names = extract_visible_names(&[
            "Recording 02:30".to_string(),
            "Connected Mobility Repository".to_string(),
            "Adrian Witaszak".to_string(),
        ]);

        assert_eq!(names, vec!["Adrian Witaszak".to_string()]);
    }
}
