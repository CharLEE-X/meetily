use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserMeetingSignal {
    browser: String,
    title: Option<String>,
    url: Option<String>,
    is_active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMeetingActivitySignals {
    active_app_name: Option<String>,
    active_window_title: Option<String>,
    running_apps: Vec<String>,
    browser_tabs: Vec<BrowserMeetingSignal>,
    checked_at: String,
    error: Option<String>,
}

#[tauri::command]
pub async fn get_meeting_activity_signals() -> Result<NativeMeetingActivitySignals, String> {
    let running_apps = get_running_app_names();
    let mut errors: Vec<String> = Vec::new();

    #[cfg(target_os = "macos")]
    let (active_app_name, active_window_title, browser_tabs) = {
        let active_app = macos_active_app().inspect_err(|error| errors.push(error.clone())).ok().flatten();
        let active_title = macos_active_window_title().inspect_err(|error| errors.push(error.clone())).ok().flatten();
        let tabs = macos_browser_tabs(&running_apps);
        (active_app, active_title, tabs)
    };

    #[cfg(not(target_os = "macos"))]
    let (active_app_name, active_window_title, browser_tabs) = (None, None, Vec::new());

    Ok(NativeMeetingActivitySignals {
        active_app_name,
        active_window_title,
        running_apps,
        browser_tabs,
        checked_at: chrono::Utc::now().to_rfc3339(),
        error: if errors.is_empty() { None } else { Some(errors.join("; ")) },
    })
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
    Ok(if value.is_empty() || value == "missing value" { None } else { Some(value) })
}

#[cfg(target_os = "macos")]
fn macos_active_app() -> Result<Option<String>, String> {
    run_osascript(r#"tell application "System Events" to get name of first application process whose frontmost is true"#)
}

#[cfg(target_os = "macos")]
fn macos_active_window_title() -> Result<Option<String>, String> {
    run_osascript(r#"tell application "System Events" to get name of front window of first application process whose frontmost is true"#)
}

#[cfg(target_os = "macos")]
fn macos_browser_tabs(running_apps: &[String]) -> Vec<BrowserMeetingSignal> {
    let browsers = [
        ("Google Chrome", r#"tell application "Google Chrome"
if (count of windows) is 0 then return ""
set activeTab to active tab of front window
return (title of activeTab) & linefeed & (URL of activeTab)
end tell"#),
        ("Arc", r#"tell application "Arc"
if (count of windows) is 0 then return ""
set activeTab to active tab of front window
return (title of activeTab) & linefeed & (URL of activeTab)
end tell"#),
        ("Microsoft Edge", r#"tell application "Microsoft Edge"
if (count of windows) is 0 then return ""
set activeTab to active tab of front window
return (title of activeTab) & linefeed & (URL of activeTab)
end tell"#),
        ("Safari", r#"tell application "Safari"
if (count of windows) is 0 then return ""
set activeTab to current tab of front window
return (name of activeTab) & linefeed & (URL of activeTab)
end tell"#),
    ];

    browsers
        .iter()
        .filter(|(browser, _)| running_apps.iter().any(|app| app.eq_ignore_ascii_case(browser) || app.to_lowercase().contains(&browser.to_lowercase())))
        .filter_map(|(browser, script)| {
            let output = run_osascript(script).ok().flatten()?;
            let mut lines = output.lines();
            let title = lines.next().map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
            let url = lines.next().map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
            Some(BrowserMeetingSignal {
                browser: (*browser).to_string(),
                title,
                url,
                is_active: true,
            })
        })
        .collect()
}
