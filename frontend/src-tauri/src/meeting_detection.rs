use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserMeetingSignal {
    browser: String,
    provider: String,
    title: Option<String>,
    url: Option<String>,
    is_active: bool,
    permission_status: String,
    checked_at: String,
    freshness_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMeetingActivitySignals {
    active_app_name: Option<String>,
    active_window_title: Option<String>,
    active_window_bounds: Option<WindowBounds>,
    running_apps: Vec<String>,
    browser_tabs: Vec<BrowserMeetingSignal>,
    checked_at: String,
    missing_permissions: Vec<String>,
    permission_status: std::collections::BTreeMap<String, String>,
    signal_freshness_ms: u64,
    degraded_mode: bool,
    error: Option<String>,
}

#[tauri::command]
pub async fn get_meeting_activity_signals() -> Result<NativeMeetingActivitySignals, String> {
    let running_apps = get_running_app_names();
    let mut errors: Vec<String> = Vec::new();
    let mut missing_permissions: BTreeSet<String> = BTreeSet::new();
    let mut permission_status = std::collections::BTreeMap::new();
    let checked_at = chrono::Utc::now().to_rfc3339();
    let collection_started = Instant::now();

    #[cfg(target_os = "macos")]
    let (active_app_name, active_window_title, active_window_bounds, browser_tabs) = {
        let active_app = match macos_active_app() {
            Ok(value) => {
                permission_status.insert("activeApp".to_string(), "available".to_string());
                value
            }
            Err(error) => {
                missing_permissions.insert("accessibility".to_string());
                permission_status.insert(
                    "activeApp".to_string(),
                    permission_status_from_error(&error),
                );
                errors.push(error);
                None
            }
        };
        let active_title = match macos_active_window_title() {
            Ok(value) => {
                permission_status.insert("activeWindow".to_string(), "available".to_string());
                value
            }
            Err(error) => {
                missing_permissions.insert("accessibility".to_string());
                permission_status.insert(
                    "activeWindow".to_string(),
                    permission_status_from_error(&error),
                );
                errors.push(error);
                None
            }
        };
        let bounds = match macos_active_window_bounds() {
            Ok(value) => {
                permission_status.insert("activeWindowBounds".to_string(), "available".to_string());
                value
            }
            Err(error) => {
                missing_permissions.insert("accessibility".to_string());
                permission_status.insert(
                    "activeWindowBounds".to_string(),
                    permission_status_from_error(&error),
                );
                errors.push(error);
                None
            }
        };
        let (tabs, browser_permission) = macos_browser_tabs(
            &running_apps,
            &checked_at,
            collection_started,
            &mut errors,
            &mut missing_permissions,
        );
        permission_status.insert("browserAutomation".to_string(), browser_permission);
        (active_app, active_title, bounds, tabs)
    };

    #[cfg(not(target_os = "macos"))]
    let (active_app_name, active_window_title, active_window_bounds, browser_tabs) =
        (None, None, None, Vec::new());

    #[cfg(not(target_os = "macos"))]
    {
        permission_status.insert("activeApp".to_string(), "limited".to_string());
        permission_status.insert("activeWindow".to_string(), "limited".to_string());
        permission_status.insert("browserAutomation".to_string(), "limited".to_string());
    }

    let missing_permissions = missing_permissions.into_iter().collect::<Vec<_>>();

    Ok(NativeMeetingActivitySignals {
        active_app_name,
        active_window_title,
        active_window_bounds,
        running_apps,
        browser_tabs,
        checked_at,
        missing_permissions: missing_permissions.clone(),
        degraded_mode: !missing_permissions.is_empty(),
        permission_status,
        signal_freshness_ms: elapsed_ms(collection_started),
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    })
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn get_running_app_names() -> Vec<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("tasklist").output()
    } else {
        Command::new("ps").args(["-axo", "comm="]).output()
    };

    let Ok(output) = output else {
        return Vec::new();
    };

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut names = BTreeSet::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let name = Path::new(trimmed)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(trimmed)
            .trim_end_matches(".exe")
            .to_string();
        if is_meeting_related_app(&name) {
            names.insert(name);
        }
    }

    names.into_iter().collect()
}

fn is_meeting_related_app(name: &str) -> bool {
    let normalized = name.to_lowercase();
    [
        "teams",
        "microsoft teams",
        "zoom",
        "google chrome",
        "chrome",
        "arc",
        "safari",
        "microsoft edge",
        "msedge",
        "firefox",
        "slack",
    ]
    .iter()
    .any(|term| normalized.contains(term))
}

fn detect_provider_from_text(text: &str) -> &'static str {
    let normalized = text.to_lowercase();
    if normalized.contains("meet.google.com") || normalized.contains("google meet") {
        "google-meet"
    } else if normalized.contains("zoom.us")
        || normalized.contains("zoom.com")
        || normalized.contains("zoom meeting")
    {
        "zoom"
    } else if normalized.contains("teams.microsoft.com")
        || normalized.contains("teams.live.com")
        || normalized.contains("microsoft teams")
        || normalized.contains("teams meeting")
    {
        "teams"
    } else if normalized.contains("slack.com/huddle")
        || normalized.contains("slack huddle")
        || normalized.contains("slack call")
        || (normalized.contains("slack")
            && (normalized.contains("huddle") || normalized.contains("call")))
    {
        "slack"
    } else {
        "unknown"
    }
}

fn permission_status_from_error(error: &str) -> String {
    let normalized = error.to_lowercase();
    if normalized.contains("not authorized")
        || normalized.contains("not permitted")
        || normalized.contains("privacy")
        || normalized.contains("accessibility")
        || normalized.contains("automation")
        || normalized.contains("-1743")
    {
        "denied".to_string()
    } else {
        "limited".to_string()
    }
}

fn record_browser_permission_error(error: &str, missing_permissions: &mut BTreeSet<String>) {
    let normalized = error.to_lowercase();
    if normalized.contains("not authorized")
        || normalized.contains("not permitted")
        || normalized.contains("automation")
        || normalized.contains("-1743")
    {
        missing_permissions.insert("browserAutomation".to_string());
    }
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<Option<String>, String> {
    let output = Command::new("osascript")
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

#[cfg(target_os = "macos")]
fn macos_active_app() -> Result<Option<String>, String> {
    run_osascript(
        r#"tell application "System Events" to get name of first application process whose frontmost is true"#,
    )
}

#[cfg(target_os = "macos")]
fn macos_active_window_title() -> Result<Option<String>, String> {
    run_osascript(
        r#"tell application "System Events" to get name of front window of first application process whose frontmost is true"#,
    )
}

#[cfg(target_os = "macos")]
fn macos_active_window_bounds() -> Result<Option<WindowBounds>, String> {
    let output = run_osascript(
        r#"tell application "System Events"
set frontProcess to first application process whose frontmost is true
set frontWindow to front window of frontProcess
set windowPosition to position of frontWindow
set windowSize to size of frontWindow
return (item 1 of windowPosition as text) & "," & (item 2 of windowPosition as text) & "," & (item 1 of windowSize as text) & "," & (item 2 of windowSize as text)
end tell"#,
    )?;
    let Some(output) = output else {
        return Ok(None);
    };
    let values = output
        .split(',')
        .filter_map(|part| part.trim().parse::<i32>().ok())
        .collect::<Vec<_>>();
    if values.len() != 4 {
        return Ok(None);
    }
    Ok(Some(WindowBounds {
        x: values[0],
        y: values[1],
        width: values[2],
        height: values[3],
    }))
}

#[cfg(target_os = "macos")]
fn macos_browser_tabs(
    running_apps: &[String],
    checked_at: &str,
    collection_started: Instant,
    errors: &mut Vec<String>,
    missing_permissions: &mut BTreeSet<String>,
) -> (Vec<BrowserMeetingSignal>, String) {
    let browsers = [
        (
            "Google Chrome",
            r#"tell application "Google Chrome"
if (count of windows) is 0 then return ""
set activeTab to active tab of front window
return (title of activeTab) & linefeed & (URL of activeTab)
end tell"#,
        ),
        (
            "Arc",
            r#"tell application "Arc"
if (count of windows) is 0 then return ""
set activeTab to active tab of front window
return (title of activeTab) & linefeed & (URL of activeTab)
end tell"#,
        ),
        (
            "Microsoft Edge",
            r#"tell application "Microsoft Edge"
if (count of windows) is 0 then return ""
set activeTab to active tab of front window
return (title of activeTab) & linefeed & (URL of activeTab)
end tell"#,
        ),
        (
            "Safari",
            r#"tell application "Safari"
if (count of windows) is 0 then return ""
set activeTab to current tab of front window
return (name of activeTab) & linefeed & (URL of activeTab)
end tell"#,
        ),
    ];

    let mut browser_permission = "unknown".to_string();
    let tabs = browsers
        .iter()
        .filter(|(browser, _)| {
            running_apps.iter().any(|app| {
                app.eq_ignore_ascii_case(browser)
                    || app.to_lowercase().contains(&browser.to_lowercase())
            })
        })
        .filter_map(|(browser, script)| {
            let output = match run_osascript(script) {
                Ok(value) => {
                    browser_permission = "available".to_string();
                    value?
                }
                Err(error) => {
                    record_browser_permission_error(&error, missing_permissions);
                    browser_permission = permission_status_from_error(&error);
                    errors.push(format!("{} tab inspection: {}", browser, error));
                    return None;
                }
            };
            let mut lines = output.lines();
            let title = lines
                .next()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let url = lines
                .next()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let provider = detect_provider_from_text(&format!(
                "{} {}",
                title.as_deref().unwrap_or(""),
                url.as_deref().unwrap_or("")
            ));
            Some(BrowserMeetingSignal {
                browser: (*browser).to_string(),
                provider: provider.to_string(),
                title,
                url,
                is_active: true,
                permission_status: "available".to_string(),
                checked_at: checked_at.to_string(),
                freshness_ms: elapsed_ms(collection_started),
            })
        })
        .collect();
    (tabs, browser_permission)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_provider_metadata() {
        assert_eq!(
            detect_provider_from_text("Daily - Google Meet https://meet.google.com/abc-defg-hij"),
            "google-meet"
        );
        assert_eq!(
            detect_provider_from_text("Zoom Meeting https://zoom.us/j/123"),
            "zoom"
        );
        assert_eq!(
            detect_provider_from_text("Join Microsoft Teams Meeting"),
            "teams"
        );
        assert_eq!(
            detect_provider_from_text("Slack huddle with platform"),
            "slack"
        );
        assert_eq!(
            detect_provider_from_text("Engineering huddle planning doc"),
            "unknown"
        );
        assert_eq!(detect_provider_from_text("Project notes"), "unknown");
    }

    #[test]
    fn maps_permission_limited_errors_to_missing_permissions() {
        let mut missing_permissions = BTreeSet::new();
        record_browser_permission_error(
            "Not authorized to send Apple events to Google Chrome. (-1743)",
            &mut missing_permissions,
        );

        assert_eq!(
            permission_status_from_error(
                "Not authorized to send Apple events to Google Chrome. (-1743)"
            ),
            "denied"
        );
        assert!(missing_permissions.contains("browserAutomation"));
    }
}
