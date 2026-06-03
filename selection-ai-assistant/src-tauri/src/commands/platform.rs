use crate::platform::{current_platform_capabilities, PlatformCapabilities};

#[tauri::command]
pub fn get_platform_capabilities() -> PlatformCapabilities {
    current_platform_capabilities()
}
