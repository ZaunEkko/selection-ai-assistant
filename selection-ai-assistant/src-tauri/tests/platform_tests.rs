use selection_ai_assistant_lib::platform::{
    stub::{LinuxPlatformBackend, MacosPlatformBackend},
    PlatformBackend, PlatformFeatureStatus, PlatformId,
};

#[cfg(windows)]
use selection_ai_assistant_lib::platform::windows::WindowsPlatformBackend;

#[cfg(windows)]
#[test]
fn windows_backend_declares_current_mvp_platform_features() {
    let capabilities = WindowsPlatformBackend::default().capabilities();

    assert_eq!(capabilities.platform, PlatformId::Windows);
    assert_eq!(
        capabilities.automatic_selection,
        PlatformFeatureStatus::Supported
    );
    assert_eq!(
        capabilities.global_input_monitor,
        PlatformFeatureStatus::Supported
    );
    assert_eq!(
        capabilities.selection_reader,
        PlatformFeatureStatus::Supported
    );
    assert_eq!(
        capabilities.selection_anchor_reader,
        PlatformFeatureStatus::Supported
    );
    assert_eq!(
        capabilities.clipboard_fallback,
        PlatformFeatureStatus::Supported
    );
    assert_eq!(capabilities.manual_hotkey, PlatformFeatureStatus::Supported);
}

#[test]
fn macos_backend_is_explicit_stub_without_claiming_selection_support() {
    let capabilities = MacosPlatformBackend::default().capabilities();

    assert_eq!(capabilities.platform, PlatformId::Macos);
    assert_eq!(
        capabilities.automatic_selection,
        PlatformFeatureStatus::PermissionRequired
    );
    assert_eq!(
        capabilities.global_input_monitor,
        PlatformFeatureStatus::PermissionRequired
    );
    assert_eq!(
        capabilities.selection_reader,
        PlatformFeatureStatus::Unavailable
    );
    assert_eq!(
        capabilities.selection_anchor_reader,
        PlatformFeatureStatus::Unavailable
    );
    assert!(capabilities
        .permission_note
        .as_deref()
        .unwrap_or_default()
        .contains("macOS"));
}

#[test]
fn linux_backend_is_explicit_stub_without_wayland_selection_claims() {
    let capabilities = LinuxPlatformBackend::default().capabilities();

    assert_eq!(capabilities.platform, PlatformId::Linux);
    assert_eq!(
        capabilities.automatic_selection,
        PlatformFeatureStatus::Unavailable
    );
    assert_eq!(
        capabilities.global_input_monitor,
        PlatformFeatureStatus::Unsupported
    );
    assert_eq!(
        capabilities.selection_reader,
        PlatformFeatureStatus::Unavailable
    );
    assert_eq!(
        capabilities.selection_anchor_reader,
        PlatformFeatureStatus::Unavailable
    );
    assert!(capabilities
        .permission_note
        .as_deref()
        .unwrap_or_default()
        .contains("Wayland"));
}

#[test]
fn current_platform_capabilities_match_compile_target() {
    let capabilities = selection_ai_assistant_lib::platform::current_platform_capabilities();

    #[cfg(windows)]
    assert_eq!(capabilities.platform, PlatformId::Windows);
    #[cfg(target_os = "macos")]
    assert_eq!(capabilities.platform, PlatformId::Macos);
    #[cfg(target_os = "linux")]
    assert_eq!(capabilities.platform, PlatformId::Linux);
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    assert_eq!(capabilities.platform, PlatformId::Unknown);
}
