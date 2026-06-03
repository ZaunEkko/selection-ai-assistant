use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiProviderKind {
    OpenAiCompatible,
    Anthropic,
    Gemini,
}

impl Default for AiProviderKind {
    fn default() -> Self {
        Self::OpenAiCompatible
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AiProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub provider_kind: AiProviderKind,
    pub api_key: String,
    pub api_key_ref: String,
    pub headers: Vec<(String, String)>,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            base_url: String::new(),
            model: String::new(),
            provider_kind: AiProviderKind::OpenAiCompatible,
            api_key: String::new(),
            api_key_ref: String::new(),
            headers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CloseButtonBehavior {
    Ask,
    MinimizeToTray,
    ExitApp,
}

impl Default for CloseButtonBehavior {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBehaviorConfig {
    pub start_minimized_to_tray: bool,
    pub close_button_behavior: CloseButtonBehavior,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub default_provider_id: Option<String>,
    pub providers: Vec<AiProviderConfig>,
    pub hover_radius: f64,
    pub hover_delay_ms: u64,
    pub candidate_timeout_ms: u64,
    pub min_drag_distance: f64,
    pub hotkey: String,
    pub clipboard_fallback_enabled: bool,
    pub show_clipboard_privacy_warning_on_first_use: bool,
    pub disable_in_elevated_windows: bool,
    pub manual_hotkey_always_enabled: bool,
    pub start_minimized_to_tray: bool,
    pub close_button_behavior: CloseButtonBehavior,
    pub disabled_apps: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_provider_id: None,
            providers: Vec::new(),
            hover_radius: 90.0,
            hover_delay_ms: 1_000,
            candidate_timeout_ms: 4_000,
            min_drag_distance: 6.0,
            hotkey: "Ctrl+Alt+A".to_string(),
            clipboard_fallback_enabled: true,
            show_clipboard_privacy_warning_on_first_use: true,
            disable_in_elevated_windows: true,
            manual_hotkey_always_enabled: true,
            start_minimized_to_tray: false,
            close_button_behavior: CloseButtonBehavior::Ask,
            disabled_apps: vec![
                "1Password.exe".to_string(),
                "KeePassXC.exe".to_string(),
                "Bitwarden.exe".to_string(),
                "mstsc.exe".to_string(),
                "AnyDesk.exe".to_string(),
                "TeamViewer.exe".to_string(),
            ],
        }
    }
}

impl AppConfig {
    pub fn settings_path() -> io::Result<PathBuf> {
        let config_dir = dirs::config_local_dir()
            .or_else(dirs::data_local_dir)
            .or_else(dirs::config_dir)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "OS config directory was not found")
            })?;
        Ok(config_dir
            .join("selection-ai-assistant")
            .join("settings.json"))
    }

    pub fn load_from_default_path_or_default() -> io::Result<Self> {
        let path = Self::settings_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load_from_path(path)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }

    pub fn save_to_default_path(&self) -> io::Result<()> {
        self.save_to_path(Self::settings_path()?)
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        durable_write(path, contents.as_bytes())
    }

    pub fn is_disabled_process(&self, process_name: &str) -> bool {
        self.disabled_apps
            .iter()
            .any(|name| name.eq_ignore_ascii_case(process_name))
    }
}

fn durable_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings.json");
    let temp_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));

    let write_result = (|| {
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        temp_file.write_all(contents)?;
        temp_file.flush()?;
        temp_file.sync_all()?;
        drop(temp_file);

        replace_file(&temp_path, path)?;
        sync_parent_directory(parent);
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    write_result
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide: Vec<u16> = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let target_wide: Vec<u16> = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let replaced = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            target_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> io::Result<()> {
    fs::rename(source, target)
}

#[cfg(not(windows))]
fn sync_parent_directory(parent: &Path) {
    if let Ok(directory) = fs::File::open(parent) {
        let _ = directory.sync_all();
    }
}

#[cfg(windows)]
fn sync_parent_directory(_parent: &Path) {}
