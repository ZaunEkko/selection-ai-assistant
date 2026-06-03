use crate::{
    platform::{
        ClipboardBackend, InputMonitor, PermissionChecker, PlatformBackend, PlatformFeatureStatus,
        PlatformId, SelectionAnchorReader, SelectionReader,
    },
    types::Rect,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct MacosPlatformBackend;

#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPlatformBackend;

#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedPlatformBackend;

impl SelectionReader for MacosPlatformBackend {
    fn selection_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl SelectionAnchorReader for MacosPlatformBackend {
    fn selection_anchor_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl ClipboardBackend for MacosPlatformBackend {
    fn clipboard_fallback_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl PermissionChecker for MacosPlatformBackend {
    fn permission_check_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::PermissionRequired
    }

    fn permission_note(&self) -> Option<String> {
        Some(
            "macOS backend 已预留；自动划词需要实现 Accessibility/Input Monitoring 权限检查与原生 selection reader。"
                .to_string(),
        )
    }
}

impl InputMonitor for MacosPlatformBackend {
    fn global_input_monitor_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::PermissionRequired
    }

    fn start_background_monitor(&self, _app: tauri::AppHandle) {}

    fn notify_ai_panel_closed_by_user(&self, _assistant_rects: Vec<Rect>) {}
}

impl PlatformBackend for MacosPlatformBackend {
    fn platform_id(&self) -> PlatformId {
        PlatformId::Macos
    }

    fn automatic_selection_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::PermissionRequired
    }

    fn manual_hotkey_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl SelectionReader for LinuxPlatformBackend {
    fn selection_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl SelectionAnchorReader for LinuxPlatformBackend {
    fn selection_anchor_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl ClipboardBackend for LinuxPlatformBackend {
    fn clipboard_fallback_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl PermissionChecker for LinuxPlatformBackend {
    fn permission_check_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }

    fn permission_note(&self) -> Option<String> {
        Some(
            "Linux backend 已预留；X11/Wayland 的全局输入监听与其他窗口选区读取需要分别实现，Wayland 默认限制更强。"
                .to_string(),
        )
    }
}

impl InputMonitor for LinuxPlatformBackend {
    fn global_input_monitor_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }

    fn start_background_monitor(&self, _app: tauri::AppHandle) {}

    fn notify_ai_panel_closed_by_user(&self, _assistant_rects: Vec<Rect>) {}
}

impl PlatformBackend for LinuxPlatformBackend {
    fn platform_id(&self) -> PlatformId {
        PlatformId::Linux
    }

    fn automatic_selection_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }

    fn manual_hotkey_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unavailable
    }
}

impl SelectionReader for UnsupportedPlatformBackend {
    fn selection_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }
}

impl SelectionAnchorReader for UnsupportedPlatformBackend {
    fn selection_anchor_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }
}

impl ClipboardBackend for UnsupportedPlatformBackend {
    fn clipboard_fallback_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }
}

impl PermissionChecker for UnsupportedPlatformBackend {
    fn permission_check_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }

    fn permission_note(&self) -> Option<String> {
        Some("当前平台尚未预留系统层 backend。".to_string())
    }
}

impl InputMonitor for UnsupportedPlatformBackend {
    fn global_input_monitor_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }

    fn start_background_monitor(&self, _app: tauri::AppHandle) {}

    fn notify_ai_panel_closed_by_user(&self, _assistant_rects: Vec<Rect>) {}
}

impl PlatformBackend for UnsupportedPlatformBackend {
    fn platform_id(&self) -> PlatformId {
        PlatformId::Unknown
    }

    fn automatic_selection_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }

    fn manual_hotkey_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Unsupported
    }
}
