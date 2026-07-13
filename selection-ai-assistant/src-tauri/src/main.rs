#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(all(target_os = "windows", debug_assertions))]
fn attach_parent_console_for_debug_session() {
    if selection_ai_assistant_lib::app_lifecycle::is_autostart_launch(std::env::args()) {
        return;
    }

    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

fn main() {
    #[cfg(all(target_os = "windows", debug_assertions))]
    attach_parent_console_for_debug_session();

    selection_ai_assistant_lib::run();
}
