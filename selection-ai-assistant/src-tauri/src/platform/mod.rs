use serde::{Deserialize, Serialize};

use crate::types::Rect;

pub mod stub;

#[cfg(windows)]
pub mod windows;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformId {
    Windows,
    Macos,
    Linux,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformFeatureStatus {
    Supported,
    Unsupported,
    PermissionRequired,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub platform: PlatformId,
    pub automatic_selection: PlatformFeatureStatus,
    pub global_input_monitor: PlatformFeatureStatus,
    pub selection_reader: PlatformFeatureStatus,
    pub selection_anchor_reader: PlatformFeatureStatus,
    pub clipboard_fallback: PlatformFeatureStatus,
    pub manual_hotkey: PlatformFeatureStatus,
    pub permission_check: PlatformFeatureStatus,
    pub permission_note: Option<String>,
}

pub trait SelectionReader {
    fn selection_reader_status(&self) -> PlatformFeatureStatus;
}

pub trait SelectionAnchorReader {
    fn selection_anchor_reader_status(&self) -> PlatformFeatureStatus;
}

pub trait ClipboardBackend {
    fn clipboard_fallback_status(&self) -> PlatformFeatureStatus;
}

pub trait PermissionChecker {
    fn permission_check_status(&self) -> PlatformFeatureStatus;

    fn permission_note(&self) -> Option<String> {
        None
    }
}

pub trait InputMonitor {
    fn global_input_monitor_status(&self) -> PlatformFeatureStatus;

    fn start_background_monitor(&self, app: tauri::AppHandle);

    fn notify_ai_panel_closed_by_user(&self, assistant_rects: Vec<Rect>);
}

pub trait PlatformBackend:
    InputMonitor + SelectionReader + SelectionAnchorReader + ClipboardBackend + PermissionChecker
{
    fn platform_id(&self) -> PlatformId;

    fn automatic_selection_status(&self) -> PlatformFeatureStatus;

    fn manual_hotkey_status(&self) -> PlatformFeatureStatus;

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            platform: self.platform_id(),
            automatic_selection: self.automatic_selection_status(),
            global_input_monitor: self.global_input_monitor_status(),
            selection_reader: self.selection_reader_status(),
            selection_anchor_reader: self.selection_anchor_reader_status(),
            clipboard_fallback: self.clipboard_fallback_status(),
            manual_hotkey: self.manual_hotkey_status(),
            permission_check: self.permission_check_status(),
            permission_note: self.permission_note(),
        }
    }
}

#[cfg(windows)]
pub type CurrentPlatformBackend = windows::WindowsPlatformBackend;

#[cfg(target_os = "macos")]
pub type CurrentPlatformBackend = stub::MacosPlatformBackend;

#[cfg(all(not(windows), not(target_os = "macos"), target_os = "linux"))]
pub type CurrentPlatformBackend = stub::LinuxPlatformBackend;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub type CurrentPlatformBackend = stub::UnsupportedPlatformBackend;

pub fn current_platform_backend() -> CurrentPlatformBackend {
    CurrentPlatformBackend::default()
}

pub fn current_platform_capabilities() -> PlatformCapabilities {
    current_platform_backend().capabilities()
}

pub fn start_background_monitor(app: tauri::AppHandle) {
    current_platform_backend().start_background_monitor(app);
}

pub fn notify_ai_panel_closed_by_user(assistant_rects: Vec<Rect>) {
    current_platform_backend().notify_ai_panel_closed_by_user(assistant_rects);
}
