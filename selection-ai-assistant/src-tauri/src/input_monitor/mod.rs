pub mod events;

#[cfg(windows)]
mod windows_monitor {
    use std::{
        ffi::c_void,
        path::Path,
        ptr::null_mut,
        sync::{mpsc, Mutex, OnceLock},
        thread,
        time::Duration,
    };

    use tauri::Manager;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GlobalFree, BOOL, LPARAM, LRESULT, POINT, WPARAM},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::{
            DataExchange::{
                CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData,
                GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
                SetClipboardData,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            Ole::CF_UNICODETEXT,
            Threading::{
                OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
                KEYEVENTF_KEYUP, VK_CONTROL, VK_MENU,
            },
            WindowsAndMessaging::{
                CallNextHookEx, DispatchMessageW, GetCursorPos, GetForegroundWindow, GetMessageW,
                GetWindowTextW, GetWindowThreadProcessId, SetWindowsHookExW, TranslateMessage,
                UnhookWindowsHookEx, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN,
                WM_LBUTTONUP, WM_MOUSEMOVE,
            },
        },
    };

    use crate::{
        app_state::AppState,
        commands::{
            panel::{hide_floating_button, show_floating_button},
            selection::{create_panel_context_for_selection, open_panel_for_context},
        },
        config::AppConfig,
        input_monitor::events::{
            apply_mouse_up_action_to_pending_selection, consume_pending_selection,
            handle_hotkey_state, handle_mouse_button_event,
            hover_action_for_pending_selection_when_idle, HotkeyAction, HotkeyKeyState,
            MouseButtonEvent, PendingHotkeyAction, PendingSelection, PendingSelectionHoverAction,
            SelectionMouseUpEffect,
        },
        selection::{
            clipboard_reader::{
                clipboard_restore_attempt_sequence, should_accept_selected_text_after_restore,
                should_prepare_conservative_clipboard_capture, should_use_clipboard_fallback,
                ClipboardFallbackContext, ClipboardRestorePlan,
            },
            types::SelectionCandidate,
        },
        types::{AppWindowInfo, Point, Rect},
    };

    const VK_A: i32 = 0x41;
    const KEY_DOWN: i16 = 0x8000u16 as i16;
    const CLIPBOARD_RESTORE_RETRY_COUNT: usize = 2;
    const CLIPBOARD_RESTORE_RETRY_DELAY: Duration = Duration::from_millis(30);

    pub fn start(app: tauri::AppHandle) {
        let (mouse_tx, mouse_rx) = mpsc::channel();
        start_low_level_mouse_hook(mouse_tx.clone());

        thread::spawn(move || {
            let _mouse_tx = mouse_tx;
            let mut drag_start: Option<Point> = None;
            let mut pending_selection: Option<PendingSelection> = None;
            let mut pending_hotkey = PendingHotkeyAction::default();

            loop {
                while let Ok(event) = mouse_rx.try_recv() {
                    handle_mouse_event(&app, &mut drag_start, &mut pending_selection, event);
                }

                let cursor = cursor_point().unwrap_or(Point { x: 0.0, y: 0.0 });
                let keys = HotkeyKeyState {
                    ctrl: key_down(VK_CONTROL as i32),
                    alt: key_down(VK_MENU as i32),
                    a: key_down(VK_A),
                };
                match handle_hotkey_state(&mut pending_hotkey, keys) {
                    HotkeyAction::Armed => {
                        consume_pending_selection(&mut pending_selection);
                    }
                    HotkeyAction::CaptureAndOpen => {
                        capture_store_and_open_panel(&app, cursor);
                        consume_pending_selection(&mut pending_selection);
                    }
                    HotkeyAction::AlreadyArmed | HotkeyAction::Idle => {}
                }

                match mouse_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(event) => {
                        handle_mouse_event(&app, &mut drag_start, &mut pending_selection, event)
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });
    }

    fn handle_mouse_event(
        app: &tauri::AppHandle,
        drag_start: &mut Option<Point>,
        pending_selection: &mut Option<PendingSelection>,
        event: MouseButtonEvent,
    ) {
        if let MouseButtonEvent::Move(position) = event {
            let hover_radius = current_config(app)
                .map(|config| config.hover_radius)
                .unwrap_or(90.0);
            match hover_action_for_pending_selection_when_idle(
                pending_selection.as_ref(),
                drag_start.as_ref(),
                position,
                hover_radius,
            ) {
                PendingSelectionHoverAction::CaptureAndShowButton { anchor } => {
                    if capture_store_and_show_floating_button(app, anchor) {
                        *pending_selection = None;
                    } else {
                        clear_selection_and_hide_button(app);
                        *pending_selection = None;
                    }
                }
                PendingSelectionHoverAction::KeepPending
                | PendingSelectionHoverAction::NoPendingSelection => {}
            }
            return;
        }

        let min_drag_distance = current_config(app)
            .map(|config| config.min_drag_distance)
            .unwrap_or(6.0);
        let Some(action) = handle_mouse_button_event(
            drag_start,
            pending_selection,
            event,
            min_drag_distance,
            &assistant_window_rects(app),
        ) else {
            return;
        };

        match apply_mouse_up_action_to_pending_selection(pending_selection, action) {
            SelectionMouseUpEffect::PendingAnchorArmedAndClearSelectionAndHide => {
                clear_selection_and_hide_button(app);
            }
            SelectionMouseUpEffect::ClearSelectionAndHide => {
                clear_selection_and_hide_button(app);
            }
            SelectionMouseUpEffect::PreserveSelection => {}
        }
    }

    static MOUSE_EVENT_SENDER: OnceLock<Mutex<Option<mpsc::Sender<MouseButtonEvent>>>> =
        OnceLock::new();

    fn start_low_level_mouse_hook(sender: mpsc::Sender<MouseButtonEvent>) {
        thread::spawn(move || {
            let sender_slot = MOUSE_EVENT_SENDER.get_or_init(|| Mutex::new(None));
            if let Ok(mut slot) = sender_slot.lock() {
                *slot = Some(sender);
            } else {
                return;
            }

            let hook =
                unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), null_mut(), 0) };
            if hook.is_null() {
                if let Ok(mut slot) = sender_slot.lock() {
                    *slot = None;
                }
                return;
            }

            let mut message: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            unsafe {
                UnhookWindowsHookEx(hook);
            }
            if let Ok(mut slot) = sender_slot.lock() {
                *slot = None;
            }
        });
    }

    unsafe extern "system" fn mouse_hook_proc(
        code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if code >= 0 {
            let hook_event = match w_param as u32 {
                WM_LBUTTONDOWN => Some(MouseButtonEvent::Down(mouse_hook_point(l_param))),
                WM_LBUTTONUP => Some(MouseButtonEvent::Up(mouse_hook_point(l_param))),
                WM_MOUSEMOVE => Some(MouseButtonEvent::Move(mouse_hook_point(l_param))),
                _ => None,
            };

            if let Some(event) = hook_event {
                if let Some(sender_slot) = MOUSE_EVENT_SENDER.get() {
                    if let Ok(slot) = sender_slot.lock() {
                        if let Some(sender) = slot.as_ref() {
                            let _ = sender.send(event);
                        }
                    }
                }
            }
        }

        CallNextHookEx(null_mut(), code, w_param, l_param)
    }

    unsafe fn mouse_hook_point(l_param: LPARAM) -> Point {
        let hook = &*(l_param as *const MSLLHOOKSTRUCT);
        Point {
            x: hook.pt.x as f64,
            y: hook.pt.y as f64,
        }
    }

    fn capture_store_and_show_floating_button(app: &tauri::AppHandle, anchor: Point) -> bool {
        let Some(config) = current_config(app) else {
            return false;
        };

        if let Some(context) = read_current_selection_context(anchor, &config) {
            app.state::<AppState>().store_latest_selection(context);
            show_floating_button(app.clone(), anchor).is_ok()
        } else {
            false
        }
    }

    fn capture_store_and_open_panel(app: &tauri::AppHandle, fallback_point: Point) {
        let Some(config) = current_config(app) else {
            clear_selection_and_hide_button(app);
            return;
        };

        if let Some(context) = read_current_selection_context(fallback_point, &config) {
            app.state::<AppState>()
                .store_latest_selection(context.clone());
            if let Ok(opened) = open_panel_for_context(app, context) {
                app.state::<AppState>().store_latest_selection(opened);
            }
        } else {
            clear_selection_and_hide_button(app);
        }
    }

    fn clear_selection_and_hide_button(app: &tauri::AppHandle) {
        app.state::<AppState>().clear_latest_selection();
        let _ = hide_floating_button(app.clone());
    }

    fn assistant_window_rects(app: &tauri::AppHandle) -> Vec<Rect> {
        ["floating-button", "ai-panel"]
            .into_iter()
            .filter_map(|label| app.get_webview_window(label))
            .filter(|window| window.is_visible().unwrap_or(false))
            .filter_map(|window| {
                let position = window.outer_position().ok()?;
                let size = window.outer_size().ok()?;
                Some(Rect {
                    x: position.x as f64,
                    y: position.y as f64,
                    width: size.width as f64,
                    height: size.height as f64,
                })
            })
            .collect()
    }

    fn current_config(app: &tauri::AppHandle) -> Option<AppConfig> {
        app.state::<AppState>()
            .config
            .lock()
            .ok()
            .map(|config| config.clone())
    }

    fn read_current_selection_context(
        fallback_point: Point,
        config: &AppConfig,
    ) -> Option<crate::commands::selection::PanelContext> {
        let window = foreground_window_info(config)?;
        let fallback_context = ClipboardFallbackContext {
            clipboard_fallback_enabled: config.clipboard_fallback_enabled,
            process_name: window.process_name.clone(),
            disabled_apps: config.disabled_apps.clone(),
            is_password_control: false,
            is_elevated_window: window.elevated,
            disable_in_elevated_windows: config.disable_in_elevated_windows,
        };

        if !should_use_clipboard_fallback(&fallback_context) {
            return None;
        }

        let text = copy_selection_with_clipboard_restore()?;
        let selection = SelectionCandidate::from_clipboard_text(
            text,
            window.process_name,
            window.window_title,
            fallback_point,
        );
        create_panel_context_for_selection(selection, false).ok()
    }

    fn key_down(vk: i32) -> bool {
        unsafe { (GetAsyncKeyState(vk) & KEY_DOWN) != 0 }
    }

    fn cursor_point() -> Option<Point> {
        let mut point = POINT { x: 0, y: 0 };
        let ok = unsafe { GetCursorPos(&mut point) };
        if ok == 0 {
            None
        } else {
            Some(Point {
                x: point.x as f64,
                y: point.y as f64,
            })
        }
    }

    fn foreground_window_info(config: &AppConfig) -> Option<AppWindowInfo> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return None;
        }

        let mut process_id = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut process_id);
        }
        if process_id == 0 {
            return None;
        }

        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return None;
        }

        let process_name = process_name(process);
        let elevated = is_process_elevated(process);
        unsafe {
            CloseHandle(process);
        }

        let process_name = process_name?;
        let elevated = match elevated {
            Some(elevated) => elevated,
            None if config.disable_in_elevated_windows => return None,
            None => false,
        };

        Some(AppWindowInfo {
            process_name,
            window_title: window_title(hwnd).unwrap_or_else(|| "Unknown window".to_string()),
            elevated,
        })
    }

    fn process_name(process: *mut c_void) -> Option<String> {
        if process.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 260];
        let mut size = buffer.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size) };
        if ok == 0 || size == 0 {
            return None;
        }
        buffer.truncate(size as usize);
        let path = String::from_utf16_lossy(&buffer);
        Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .filter(|name| !name.trim().is_empty())
    }

    fn window_title(hwnd: *mut c_void) -> Option<String> {
        let mut buffer = [0u16; 512];
        let len = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if len <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..len as usize]))
            .filter(|title| !title.trim().is_empty())
    }

    fn is_process_elevated(process: *mut c_void) -> Option<bool> {
        if process.is_null() {
            return None;
        }

        let mut token = null_mut();
        let opened = unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) };
        if opened == 0 || token.is_null() {
            return None;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length = 0u32;
        let ok = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut TOKEN_ELEVATION as *mut c_void,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            )
        };
        unsafe {
            CloseHandle(token);
        }

        if ok == 0 {
            None
        } else {
            Some(elevation.TokenIsElevated != 0)
        }
    }

    fn copy_selection_with_clipboard_restore() -> Option<String> {
        let before_sequence = clipboard_sequence_number();
        let restore_plan = clipboard_restore_plan()?;

        send_ctrl_c();
        thread::sleep(Duration::from_millis(120));

        let sequence_changed = clipboard_sequence_number() != before_sequence;
        let selected = if sequence_changed {
            read_clipboard_unicode().map(|text| text.trim().to_string())
        } else {
            None
        };

        let restored_clipboard = restore_clipboard_with_retry(restore_plan);

        should_accept_selected_text_after_restore(selected.as_deref(), restored_clipboard)
    }

    fn clipboard_sequence_number() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    fn clipboard_restore_plan() -> Option<ClipboardRestorePlan> {
        with_open_clipboard(|| unsafe {
            let format_count = CountClipboardFormats();
            if format_count < 0 {
                return None;
            }

            let unicode_text_available = IsClipboardFormatAvailable(CF_UNICODETEXT.into()) != 0;
            if !should_prepare_conservative_clipboard_capture(
                format_count as u32,
                unicode_text_available,
            ) {
                return None;
            }

            if unicode_text_available {
                read_clipboard_unicode_from_open().map(ClipboardRestorePlan::Text)
            } else {
                Some(ClipboardRestorePlan::Empty)
            }
        })
        .flatten()
    }

    fn restore_clipboard_with_retry(plan: ClipboardRestorePlan) -> bool {
        let attempts = clipboard_restore_attempt_sequence(plan, CLIPBOARD_RESTORE_RETRY_COUNT);
        let last_index = attempts.len().saturating_sub(1);

        for (index, attempt) in attempts.into_iter().enumerate() {
            if restore_clipboard(attempt) {
                return index != last_index;
            }

            if index < last_index {
                thread::sleep(CLIPBOARD_RESTORE_RETRY_DELAY);
            }
        }

        false
    }

    fn restore_clipboard(plan: ClipboardRestorePlan) -> bool {
        match plan {
            ClipboardRestorePlan::Text(text) => write_clipboard_unicode(&text),
            ClipboardRestorePlan::Empty => empty_clipboard(),
        }
    }

    fn send_ctrl_c() {
        let mut inputs = [
            keyboard_input(VK_CONTROL, 0),
            keyboard_input(0x43, 0),
            keyboard_input(0x43, KEYEVENTF_KEYUP),
            keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_mut_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            );
        }
    }

    fn keyboard_input(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn read_clipboard_unicode() -> Option<String> {
        with_open_clipboard(|| unsafe { read_clipboard_unicode_from_open() }).flatten()
    }

    unsafe fn read_clipboard_unicode_from_open() -> Option<String> {
        let handle = GetClipboardData(CF_UNICODETEXT.into());
        if handle.is_null() {
            return None;
        }
        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            return None;
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        GlobalUnlock(handle);
        Some(text)
    }

    fn write_clipboard_unicode(text: &str) -> bool {
        with_open_clipboard(|| unsafe {
            let _ = EmptyClipboard();
            let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = utf16.len() * std::mem::size_of::<u16>();
            let handle = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if handle.is_null() {
                return false;
            }
            let ptr = GlobalLock(handle) as *mut u8;
            if ptr.is_null() {
                let _ = GlobalFree(handle);
                return false;
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr() as *const u8, ptr, bytes);
            GlobalUnlock(handle);
            if SetClipboardData(CF_UNICODETEXT.into(), handle).is_null() {
                let _ = GlobalFree(handle);
                return false;
            }
            true
        })
        .unwrap_or(false)
    }

    fn empty_clipboard() -> bool {
        with_open_clipboard(|| unsafe { EmptyClipboard() != 0 }).unwrap_or(false)
    }

    fn with_open_clipboard<T>(f: impl FnOnce() -> T) -> Option<T> {
        let opened: BOOL = unsafe { OpenClipboard(null_mut()) };
        if opened == 0 {
            return None;
        }
        let result = f();
        unsafe {
            CloseClipboard();
        }
        Some(result)
    }
}

#[cfg(windows)]
pub fn start_background_monitor(app: tauri::AppHandle) {
    windows_monitor::start(app);
}

#[cfg(not(windows))]
pub fn start_background_monitor(_app: tauri::AppHandle) {}
