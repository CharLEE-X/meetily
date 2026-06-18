use crate::state::AppState;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

static CAPTURE_TASKS: Lazy<Mutex<HashMap<String, JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotPreferences {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub capture_target: String,
    pub retention_days: u32,
}

impl Default for ScreenshotPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: DEFAULT_INTERVAL_SECONDS,
            capture_target: "fullScreen".to_string(),
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingScreenshot {
    pub id: String,
    pub meeting_id: String,
    pub captured_at: String,
    pub recording_time: Option<f64>,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub display_label: Option<String>,
    pub status: String,
    pub redaction_status: String,
    pub source: String,
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

    let app_for_task = app.clone();
    let db_manager = state.db_manager.clone();
    let meeting_for_task = meeting_id.clone();
    let recording_started_at = parse_recording_started_at(recording_started_at.as_deref());
    let interval_seconds = preferences.interval_seconds;

    let handle = tokio::spawn(async move {
        loop {
            if let Err(err) = capture_and_store_screenshot(
                &app_for_task,
                db_manager.pool(),
                &meeting_for_task,
                recording_started_at,
            )
            .await
            {
                log::warn!(
                    "Periodic screenshot capture failed for meeting {}: {}",
                    meeting_for_task,
                    err
                );
            }

            sleep(Duration::from_secs(interval_seconds)).await;
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
) -> Result<(), String> {
    let pool = state.db_manager.pool();
    let row = sqlx::query(
        r#"
        SELECT file_path
        FROM meeting_screenshots
        WHERE id = ? AND deleted_at IS NULL
        "#,
    )
    .bind(&screenshot_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load screenshot: {}", err))?;

    let Some(row) = row else {
        return Ok(());
    };

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE meeting_screenshots
        SET status = 'deleted', deleted_at = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&now)
    .bind(&now)
    .bind(&screenshot_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to delete screenshot metadata: {}", err))?;

    if delete_file.unwrap_or(true) {
        let file_path: String = row.get("file_path");
        if let Err(err) = std::fs::remove_file(&file_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!("Failed to remove screenshot file {}: {}", file_path, err);
            }
        }
    }

    Ok(())
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
) -> Result<Option<MeetingScreenshot>, String> {
    let screenshot_id = Uuid::new_v4().to_string();
    let captured_at = Utc::now();
    let file_path = screenshot_file_path(app, meeting_id, &screenshot_id, captured_at)
        .map_err(|err| format!("Failed to prepare screenshot folder: {}", err))?;

    capture_screen_to_file(&file_path).await?;

    let analysis = analyze_screenshot(&file_path).await;
    if !analysis.is_relevant {
        if let Err(err) = std::fs::remove_file(&file_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "Failed to remove irrelevant screenshot {}: {}",
                    file_path.display(),
                    err
                );
            }
        }
        return Ok(None);
    }

    let recording_time = recording_started_at
        .map(|started_at| (captured_at - started_at).num_milliseconds() as f64 / 1000.0)
        .filter(|seconds| *seconds >= 0.0);
    let display_label = recording_time.map(|seconds| format_time_label(seconds));
    let captured_at_string = captured_at.to_rfc3339();
    let now = Utc::now().to_rfc3339();
    let file_path_string = file_path.to_string_lossy().to_string();

    let metadata_json = json!({
        "analysis": analysis,
    })
    .to_string();

    sqlx::query(
        r#"
        INSERT INTO meeting_screenshots
            (id, meeting_id, captured_at, recording_time, file_path, display_label, status, redaction_status, source, created_at, updated_at, metadata_json)
        VALUES (?, ?, ?, ?, ?, ?, 'captured', 'not_available', 'periodic', ?, ?, ?)
        "#,
    )
    .bind(&screenshot_id)
    .bind(meeting_id)
    .bind(&captured_at_string)
    .bind(recording_time)
    .bind(&file_path_string)
    .bind(&display_label)
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
        file_path: file_path_string,
        thumbnail_path: None,
        display_label,
        status: "captured".to_string(),
        redaction_status: "not_available".to_string(),
        source: "periodic".to_string(),
    }))
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
    let has_provider = provider.is_some();
    let has_visible_name = !visible_names.is_empty();
    let has_call_controls = recognized_text.iter().any(|text| {
        contains_any(
            &text.to_lowercase(),
            &["mute", "camera", "captions", "present", "leave call"],
        )
    });

    let confidence = match (has_provider, has_visible_name, has_call_controls) {
        (true, true, _) => 0.92,
        (true, false, true) => 0.78,
        (true, false, false) => 0.62,
        (false, true, true) => 0.68,
        _ => 0.0,
    };

    ScreenshotAnalysis {
        is_relevant: confidence >= SCREENSHOT_RELEVANCE_THRESHOLD,
        confidence,
        provider,
        visible_names,
        text_snippets: recognized_text
            .iter()
            .filter(|text| !text.trim().is_empty())
            .take(20)
            .cloned()
            .collect(),
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
               display_label, status, redaction_status, source
        FROM meeting_screenshots
        WHERE meeting_id = ? AND deleted_at IS NULL
        ORDER BY COALESCE(recording_time, 0), captured_at
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load screenshots: {}", err))?;

    Ok(rows
        .into_iter()
        .map(|row| MeetingScreenshot {
            id: row.get("id"),
            meeting_id: row.get("meeting_id"),
            captured_at: row.get("captured_at"),
            recording_time: row.try_get("recording_time").ok(),
            file_path: row.get("file_path"),
            thumbnail_path: row.try_get("thumbnail_path").ok(),
            display_label: row.try_get("display_label").ok(),
            status: row.get("status"),
            redaction_status: row.get("redaction_status"),
            source: row.get("source"),
        })
        .collect())
}

fn stop_capture_task(meeting_id: &str) -> bool {
    let Ok(mut tasks) = CAPTURE_TASKS.lock() else {
        return false;
    };
    if let Some(handle) = tasks.remove(meeting_id) {
        handle.abort();
        return true;
    }
    false
}

fn normalize_preferences(mut preferences: ScreenshotPreferences) -> ScreenshotPreferences {
    preferences.interval_seconds = preferences
        .interval_seconds
        .clamp(MIN_INTERVAL_SECONDS, MAX_INTERVAL_SECONDS);
    if preferences.capture_target != "fullScreen" {
        preferences.capture_target = "fullScreen".to_string();
    }
    if preferences.retention_days == 0 {
        preferences.retention_days = DEFAULT_RETENTION_DAYS;
    }
    preferences
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

#[cfg(target_os = "macos")]
async fn capture_screen_to_file(path: &Path) -> Result<(), String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let status = std::process::Command::new("/usr/sbin/screencapture")
            .arg("-x")
            .arg(&path)
            .status()
            .map_err(|err| format!("Failed to start macOS screenshot capture: {}", err))?;

        if status.success() {
            Ok(())
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
async fn capture_screen_to_file(_path: &Path) -> Result<(), String> {
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
        assert_eq!(preferences.capture_target, "fullScreen");
    }

    #[test]
    fn normalizes_preferences_to_supported_bounds() {
        let preferences = normalize_preferences(ScreenshotPreferences {
            enabled: true,
            interval_seconds: 5,
            capture_target: "activeWindow".to_string(),
            retention_days: 0,
        });

        assert!(preferences.enabled);
        assert_eq!(preferences.interval_seconds, MIN_INTERVAL_SECONDS);
        assert_eq!(preferences.capture_target, "fullScreen");
        assert_eq!(preferences.retention_days, DEFAULT_RETENTION_DAYS);
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
    fn screenshot_analysis_keeps_google_meet_with_visible_name() {
        let analysis = analyze_recognized_text(&[
            "meet.google.com/trv-nxib-ftd".to_string(),
            "Adrian Witaszak".to_string(),
            "Captions".to_string(),
        ]);

        assert!(analysis.is_relevant);
        assert_eq!(analysis.provider, Some("Google Meet".to_string()));
        assert_eq!(analysis.visible_names, vec!["Adrian Witaszak".to_string()]);
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
