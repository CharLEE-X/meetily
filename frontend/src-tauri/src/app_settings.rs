use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};
use tauri::{AppHandle, Manager, Runtime};

const SETTINGS_FILE_NAME: &str = "app-settings.json";
const LOGIN_AGENT_LABEL: &str = "com.meetily.ai.login";
const LOGIN_AGENT_FILE_NAME: &str = "com.meetily.ai.login.plist";
static SETTINGS_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub launch_at_login: bool,
    pub start_minimized: bool,
    pub startup_supported: bool,
    pub login_item_installed: bool,
    pub login_item_path: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            start_minimized: false,
            startup_supported: startup_supported(),
            login_item_installed: false,
            login_item_path: login_agent_path().map(|path| path.to_string_lossy().to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsUpdate {
    pub launch_at_login: bool,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StoredAppSettings {
    launch_at_login: bool,
    start_minimized: bool,
}

#[tauri::command]
pub async fn get_app_settings<R: Runtime>(app: AppHandle<R>) -> Result<AppSettings, String> {
    load_app_settings(&app)
}

#[tauri::command]
pub async fn update_app_settings<R: Runtime>(
    app: AppHandle<R>,
    settings: AppSettingsUpdate,
) -> Result<AppSettings, String> {
    let _guard = SETTINGS_LOCK
        .lock()
        .map_err(|err| format!("Failed to lock app settings: {}", err))?;

    if settings.launch_at_login {
        install_login_item().map_err(|err| {
            format!(
                "Failed to enable launch at login. Check that RecallX can write to LaunchAgents: {}",
                err
            )
        })?;
    } else {
        remove_login_item().map_err(|err| {
            format!(
                "Failed to disable launch at login. Check LaunchAgents permissions: {}",
                err
            )
        })?;
    }

    let stored = StoredAppSettings {
        launch_at_login: settings.launch_at_login,
        start_minimized: settings.start_minimized,
    };
    save_stored_settings_locked(&app, &stored)?;

    load_app_settings_locked(&app)
}

pub fn should_start_minimized<R: Runtime>(app: &AppHandle<R>) -> bool {
    load_stored_settings(app)
        .map(|settings| settings.start_minimized)
        .unwrap_or(false)
}

fn load_app_settings<R: Runtime>(app: &AppHandle<R>) -> Result<AppSettings, String> {
    let _guard = SETTINGS_LOCK
        .lock()
        .map_err(|err| format!("Failed to lock app settings: {}", err))?;

    load_app_settings_locked(app)
}

fn load_app_settings_locked<R: Runtime>(app: &AppHandle<R>) -> Result<AppSettings, String> {
    let stored = load_stored_settings(app).unwrap_or_default();
    let login_item_installed = is_login_item_installed();

    Ok(AppSettings {
        launch_at_login: stored.launch_at_login && login_item_installed,
        start_minimized: stored.start_minimized,
        startup_supported: startup_supported(),
        login_item_installed,
        login_item_path: login_agent_path().map(|path| path.to_string_lossy().to_string()),
    })
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config directory: {}", err))?;
    Ok(dir.join(SETTINGS_FILE_NAME))
}

fn load_stored_settings<R: Runtime>(app: &AppHandle<R>) -> Result<StoredAppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(StoredAppSettings::default());
    }

    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read app settings from {:?}: {}", path, err))?;
    serde_json::from_str(&content)
        .map_err(|err| format!("Failed to parse app settings from {:?}: {}", path, err))
}

fn save_stored_settings_locked<R: Runtime>(
    app: &AppHandle<R>,
    settings: &StoredAppSettings,
) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create app settings directory: {}", err))?;
    }

    let content = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("Failed to serialize app settings: {}", err))?;
    write_atomic(&path, content.as_bytes())
}

fn write_atomic(path: &Path, content: &[u8]) -> Result<(), String> {
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, content).map_err(|err| {
        format!(
            "Failed to write temporary app settings to {:?}: {}",
            temp_path, err
        )
    })?;
    fs::rename(&temp_path, path)
        .map_err(|err| format!("Failed to replace app settings at {:?}: {}", path, err))
}

#[cfg(target_os = "macos")]
fn startup_supported() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
fn startup_supported() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn login_agent_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| {
        home.join("Library")
            .join("LaunchAgents")
            .join(LOGIN_AGENT_FILE_NAME)
    })
}

#[cfg(not(target_os = "macos"))]
fn login_agent_path() -> Option<PathBuf> {
    None
}

#[cfg(target_os = "macos")]
fn is_login_item_installed() -> bool {
    login_agent_path()
        .map(|path| path.exists())
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn is_login_item_installed() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn install_login_item() -> Result<(), String> {
    let path =
        login_agent_path().ok_or_else(|| "Could not resolve LaunchAgents path".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create LaunchAgents directory: {}", err))?;
    }

    let executable = resolve_login_executable_path()?;
    let plist = render_launch_agent_plist(&executable.to_string_lossy());
    fs::write(&path, plist).map_err(|err| format!("Failed to write {:?}: {}", path, err))
}

#[cfg(not(target_os = "macos"))]
fn install_login_item() -> Result<(), String> {
    Err("Launch at login is currently supported on macOS only.".to_string())
}

#[cfg(target_os = "macos")]
fn remove_login_item() -> Result<(), String> {
    let Some(path) = login_agent_path() else {
        return Ok(());
    };

    if path.exists() {
        fs::remove_file(&path).map_err(|err| format!("Failed to remove {:?}: {}", path, err))?;
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn resolve_login_executable_path() -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|err| format!("Failed to resolve current executable: {}", err))?;

    if is_bundled_app_executable(&executable) {
        return Ok(executable);
    }

    Err(format!(
        "Launch at login can only be enabled from an installed RecallX.app bundle. Current executable is {:?}.",
        executable
    ))
}

#[cfg(target_os = "macos")]
fn is_bundled_app_executable(path: &Path) -> bool {
    let components: Vec<_> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect();

    components.windows(3).any(|window| {
        window[0].ends_with(".app") && window[1] == "Contents" && window[2] == "MacOS"
    })
}

#[cfg(not(target_os = "macos"))]
fn remove_login_item() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn render_launch_agent_plist(executable_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
"#,
        xml_escape(LOGIN_AGENT_LABEL),
        xml_escape(executable_path)
    )
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::write_atomic;
    #[cfg(target_os = "macos")]
    use super::{
        is_bundled_app_executable, render_launch_agent_plist, xml_escape, LOGIN_AGENT_LABEL,
    };
    use std::fs;
    #[cfg(target_os = "macos")]
    use std::path::Path;

    #[cfg(target_os = "macos")]
    #[test]
    fn launch_agent_plist_contains_escaped_executable() {
        let plist =
            render_launch_agent_plist("/Applications/Meetily & Friends.app/Contents/MacOS/meetily");

        assert!(plist.contains(LOGIN_AGENT_LABEL));
        assert!(plist.contains("/Applications/Meetily &amp; Friends.app/Contents/MacOS/meetily"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn xml_escape_handles_reserved_characters() {
        assert_eq!(
            xml_escape("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn bundled_app_executable_detection_rejects_dev_binary() {
        assert!(is_bundled_app_executable(Path::new(
            "/Applications/meetily.app/Contents/MacOS/meetily"
        )));
        assert!(!is_bundled_app_executable(Path::new(
            "/Users/dev/meetily/target/debug/meetily"
        )));
    }

    #[test]
    fn write_atomic_replaces_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");

        write_atomic(&path, br#"{"startMinimized":false}"#).expect("initial write");
        write_atomic(&path, br#"{"startMinimized":true}"#).expect("replacement write");

        let content = fs::read_to_string(path).expect("settings content");
        assert_eq!(content, r#"{"startMinimized":true}"#);
    }
}
