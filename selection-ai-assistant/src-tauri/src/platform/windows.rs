use std::{
    collections::HashMap,
    ffi::c_void,
    path::Path,
    ptr::null_mut,
    sync::{mpsc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use base64::Engine;
use tauri::{Emitter, Manager};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GlobalFree, BOOL, LPARAM, LRESULT, POINT, RECT as WinRect, WPARAM},
    Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        RGBQUAD, SRCCOPY,
    },
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::{
        DataExchange::{
            CloseClipboard, CountClipboardFormats, EmptyClipboard, EnumClipboardFormats,
            GetClipboardData, GetClipboardSequenceNumber, IsClipboardFormatAvailable,
            OpenClipboard, SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
        Ole::CF_UNICODETEXT,
        Threading::{
            OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, VK_CONTROL, VK_MENU, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetAncestor, GetCursorPos, GetForegroundWindow,
            GetMessageW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
            SetWindowsHookExW, SystemParametersInfoW, TranslateMessage, UnhookWindowsHookEx,
            WindowFromPoint, GA_ROOT, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN,
            WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
        },
    },
};

use crate::{
    app_state::{AppState, SelectionVisualState},
    commands::{
        panel::{
            floating_button_position_for_selection, hide_floating_button,
            hide_replacement_preset_panel_for_app, show_floating_button_at_position,
            show_floating_button_for_selection,
        },
        screenshot::show_screenshot_overlay_for_point,
        selection::{
            create_panel_context_for_selection, emit_panel_context,
            panel_context_for_visible_refresh,
        },
    },
    config::AppConfig,
    input_monitor::events::{
        consume_pending_selection, handle_hotkey_state,
        hover_action_for_pending_selection_when_idle, manual_hotkey_trigger_key,
        predicted_scroll_offset, scroll_tracker_action, selection_geometry_matches_drag_gesture,
        selection_still_trackable_on_monitors, settled_measurement_is_stable,
        should_follow_scroll_for_source, update_scroll_ratio,
        visible_floating_button_action_when_idle, HotkeyAction, HotkeyKeyState, MouseButtonEvent,
        PendingHotkeyAction, PendingSelection, PendingSelectionHoverAction, ScrollBurst,
        ScrollPace, ScrollRatioEstimate, ScrollTrackerAction, VisibleFloatingButton,
        VisibleFloatingButtonAction, SCROLL_SETTLE_IDLE_MS,
    },
    platform::{
        ClipboardBackend, InputMonitor, PermissionChecker, PlatformBackend, PlatformFeatureStatus,
        PlatformId, SelectionAnchorReader, SelectionReader,
    },
    selection::{
        clipboard_reader::{
            clipboard_restore_attempt_sequence, should_accept_selected_text_after_capture,
            should_block_clipboard_fallback_after_uia_result,
            should_prepare_conservative_clipboard_capture, should_use_clipboard_fallback,
            ClipboardFallbackContext, ClipboardFormatSnapshot, ClipboardRestorePlan,
            ClipboardRestoreStatus,
        },
        types::SelectionCandidate,
        uia_reader::{
            read_current_uia_selection_from_hwnd, read_current_uia_selection_from_hwnd_with_points,
        },
    },
    types::{AppWindowInfo, Point, Rect},
};

const KEY_DOWN: i16 = 0x8000u16 as i16;
const TARGET_CONTROLS_CLOSE_DELAY: Duration = Duration::from_millis(180);
const CLIPBOARD_RESTORE_RETRY_COUNT: usize = 2;
const CLIPBOARD_RESTORE_RETRY_DELAY: Duration = Duration::from_millis(30);
const SCROLL_FOLLOW_MAX_PLACEMENT_HEIGHT: f64 = 36.0;
/// 跟随线程的采样间隔。一次采样只做一次 BitBlt + 内存扫描，成本在毫秒级，
/// 因此可以按这个节奏持续测量真实选区位置，而不是靠预测累积误差。
const SCROLL_TRACK_SAMPLE_INTERVAL: Duration = Duration::from_millis(20);
/// 跟随会话的兜底上限，避免任何异常情况下线程常驻。
const SCROLL_TRACK_MAX_SESSION: Duration = Duration::from_secs(20);
/// 跟踪时以预测位置为中心的搜索带宽度/高度余量。
const SCROLL_TRACK_SEARCH_PADDING_X: f64 = 80.0;
const SCROLL_TRACK_SEARCH_PADDING_Y: f64 = 160.0;
/// UIA 回退的采样间隔（以采样次数计）。跨进程 COM 调用比像素测量贵得多，
/// 只在视觉测量失败且到达该间隔时才尝试。
const UIA_MEASURE_SAMPLE_STRIDE: u64 = 5;
/// 单次像素捕获的上限，避免异常矩形导致巨额分配。
const MAX_CAPTURE_WIDTH: i32 = 6000;
const MAX_CAPTURE_HEIGHT: i32 = 4000;

#[cfg(debug_assertions)]
fn trace_selection_monitor(args: std::fmt::Arguments<'_>) {
    eprintln!("[selection-monitor] {args}");
}

#[cfg(not(debug_assertions))]
fn trace_selection_monitor(_args: std::fmt::Arguments<'_>) {}

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPlatformBackend;

impl SelectionReader for WindowsPlatformBackend {
    fn selection_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Supported
    }
}

impl SelectionAnchorReader for WindowsPlatformBackend {
    fn selection_anchor_reader_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Supported
    }
}

impl ClipboardBackend for WindowsPlatformBackend {
    fn clipboard_fallback_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Supported
    }
}

impl PermissionChecker for WindowsPlatformBackend {
    fn permission_check_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Supported
    }
}

impl InputMonitor for WindowsPlatformBackend {
    fn global_input_monitor_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Supported
    }

    fn start_background_monitor(&self, app: tauri::AppHandle) {
        start(app);
    }

    fn notify_ai_panel_closed_by_user(&self, _assistant_rects: Vec<Rect>) {}
}

impl PlatformBackend for WindowsPlatformBackend {
    fn platform_id(&self) -> PlatformId {
        PlatformId::Windows
    }

    fn automatic_selection_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Supported
    }

    fn manual_hotkey_status(&self) -> PlatformFeatureStatus {
        PlatformFeatureStatus::Supported
    }
}

fn start(app: tauri::AppHandle) {
    let (mouse_tx, mouse_rx) = mpsc::channel();
    start_low_level_mouse_hook(mouse_tx.clone());

    thread::spawn(move || {
        let _mouse_tx = mouse_tx;
        let mut drag_start: Option<Point> = None;
        let mut pending_selection: Option<PendingSelection> = None;
        let mut visible_floating_button: Option<VisibleFloatingButton> = None;
        let mut pointer_inside_target_controls = false;
        let mut pending_hotkey = PendingHotkeyAction::default();
        let mut scroll_burst = ScrollBurst::default();
        let monitor_started_at = Instant::now();

        loop {
            while let Ok(event) = mouse_rx.try_recv() {
                handle_mouse_event(
                    &app,
                    &mut drag_start,
                    &mut pending_selection,
                    &mut visible_floating_button,
                    &mut pointer_inside_target_controls,
                    &mut scroll_burst,
                    event,
                    elapsed_ms(monitor_started_at),
                );
            }

            let cursor = cursor_point().unwrap_or(Point { x: 0.0, y: 0.0 });
            let trigger_key = current_config(&app)
                .and_then(|config| manual_hotkey_trigger_key(&config.hotkey))
                .unwrap_or('A');
            let keys = HotkeyKeyState {
                ctrl: key_down(VK_CONTROL as i32),
                alt: key_down(VK_MENU as i32),
                a: key_down(trigger_key as i32),
            };
            match handle_hotkey_state(&mut pending_hotkey, keys) {
                HotkeyAction::Armed => {
                    consume_pending_selection(&mut pending_selection);
                    visible_floating_button = None;
                }
                HotkeyAction::CaptureAndOpen => {
                    if let Err(error) = show_screenshot_overlay_for_point(&app, cursor) {
                        trace_selection_monitor(format_args!(
                            "screenshot overlay failed after hotkey: {error:?}"
                        ));
                    }
                    consume_pending_selection(&mut pending_selection);
                    visible_floating_button = None;
                }
                HotkeyAction::AlreadyArmed | HotkeyAction::Idle => {}
            }

            match mouse_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => handle_mouse_event(
                    &app,
                    &mut drag_start,
                    &mut pending_selection,
                    &mut visible_floating_button,
                    &mut pointer_inside_target_controls,
                    &mut scroll_burst,
                    event,
                    elapsed_ms(monitor_started_at),
                ),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn emit_floating_button_pointer_position(app: &tauri::AppHandle, position: Point) {
    let Some(window) = app.get_webview_window("floating-button") else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        return;
    }
    let Ok(window_position) = window.outer_position() else {
        return;
    };
    let Ok(window_size) = window.outer_size() else {
        return;
    };
    let rect = Rect {
        x: window_position.x as f64,
        y: window_position.y as f64,
        width: window_size.width as f64,
        height: window_size.height as f64,
    };
    if !crate::input_monitor::events::rect_contains(rect, position) {
        return;
    }

    let _ = window.emit(
        "floating_button_pointer_position",
        serde_json::json!({
            "x": position.x - rect.x,
            "y": position.y - rect.y,
            "width": rect.width,
            "height": rect.height,
        }),
    );
}

fn update_target_controls_pointer_state(
    app: &tauri::AppHandle,
    position: Point,
    was_inside: &mut bool,
) {
    let target_rects = target_control_window_rects(app);
    let is_inside = target_rects
        .iter()
        .any(|rect| crate::input_monitor::events::rect_contains(*rect, position));
    if is_inside == *was_inside {
        return;
    }
    *was_inside = is_inside;

    if is_inside
        || !app
            .get_webview_window("replacement-preset")
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false)
    {
        return;
    }

    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(TARGET_CONTROLS_CLOSE_DELAY);
        let Some(position) = cursor_point() else {
            return;
        };
        let target_rects = target_control_window_rects(&app);
        let is_inside = target_rects
            .iter()
            .any(|rect| crate::input_monitor::events::rect_contains(*rect, position));
        if is_inside {
            return;
        }

        let _ = hide_replacement_preset_panel_for_app(app);
    });
}

fn target_control_window_rects(app: &tauri::AppHandle) -> Vec<Rect> {
    ["floating-button", "replacement-preset"]
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

fn handle_mouse_event(
    app: &tauri::AppHandle,
    drag_start: &mut Option<Point>,
    pending_selection: &mut Option<PendingSelection>,
    visible_floating_button: &mut Option<VisibleFloatingButton>,
    pointer_inside_target_controls: &mut bool,
    scroll_burst: &mut ScrollBurst,
    event: MouseButtonEvent,
    now_ms: u64,
) {
    trace_selection_monitor(format_args!("mouse event: {event:?}"));
    if let MouseButtonEvent::Wheel { position, delta } = event {
        let in_assistant_window = assistant_window_rects(app)
            .iter()
            .any(|window| crate::input_monitor::events::rect_contains(*window, position));
        if !in_assistant_window {
            follow_visible_floating_button_after_scroll(
                app,
                visible_floating_button,
                scroll_burst,
                position,
                delta,
                now_ms,
            );
        }
        return;
    }

    if let MouseButtonEvent::Move(position) = event {
        emit_floating_button_pointer_position(app, position);
        update_target_controls_pointer_state(app, position, pointer_inside_target_controls);
        let config = current_config(app).unwrap_or_default();
        match hover_action_for_pending_selection_when_idle(
            pending_selection,
            drag_start.as_ref(),
            position,
            config.hover_radius,
            now_ms,
            config.hover_delay_ms,
        ) {
            PendingSelectionHoverAction::CaptureAndShowButton { anchor } => {
                if let Some(button) = capture_store_and_show_floating_button(app, anchor, &[], None)
                {
                    *pending_selection = None;
                    *visible_floating_button = Some(button);
                } else {
                    clear_selection_and_hide_button(app);
                    *pending_selection = None;
                    *visible_floating_button = None;
                }
                return;
            }
            PendingSelectionHoverAction::KeepPending => return,
            PendingSelectionHoverAction::NoPendingSelection => {}
        }

        if let VisibleFloatingButtonAction::HideAndRearmSelection { anchor } =
            visible_floating_button_action_when_idle(
                visible_floating_button,
                drag_start.as_ref(),
                position,
                config.hover_radius,
                &assistant_window_rects(app),
            )
        {
            *pending_selection = Some(PendingSelection {
                anchor,
                toolbar_anchor: anchor,
                hover_started_at_ms: None,
            });
            let _ = hide_floating_button(app.clone());
        }
        return;
    }

    let min_drag_distance = current_config(app)
        .map(|config| config.min_drag_distance)
        .unwrap_or(6.0);

    // 处理 mouse button 事件
    match event {
        MouseButtonEvent::Down(point) => {
            *drag_start = Some(point);
            // 新的一次交互开始，之前的滚轮节奏统计不应继续影响快慢判定。
            scroll_burst.reset();
            consume_pending_selection(pending_selection);
        }
        MouseButtonEvent::Up(up_point) => {
            if let Some(down_point) = drag_start.take() {
                // 检查是否满足 drag 距离
                let is_drag_met = crate::input_monitor::events::is_drag_distance_met(
                    down_point,
                    up_point,
                    min_drag_distance,
                );

                // 检查是否在助手窗口内
                let assistant_rects = assistant_window_rects(app);
                let in_assistant_window = assistant_rects
                    .iter()
                    .any(|window| crate::input_monitor::events::rect_contains(*window, up_point));
                trace_selection_monitor(format_args!(
                    "mouse up: down={down_point:?}, up={up_point:?}, min_drag_distance={min_drag_distance}, is_drag_met={is_drag_met}, in_assistant_window={in_assistant_window}, assistant_rects={assistant_rects:?}"
                ));

                if is_drag_met && !in_assistant_window {
                    let selection_hint_rects = drag_selection_hint_rects(down_point, up_point);
                    let toolbar_anchor = Point {
                        x: down_point.x.min(up_point.x),
                        y: down_point.y.min(up_point.y).max(0.0),
                    };
                    trace_selection_monitor(format_args!(
                        "drag selection released; capture after 60ms at anchor={toolbar_anchor:?}"
                    ));
                    thread::sleep(Duration::from_millis(60));
                    if let Some(button) = capture_store_and_show_floating_button(
                        app,
                        toolbar_anchor,
                        &selection_hint_rects,
                        Some((down_point, up_point)),
                    ) {
                        *pending_selection = None;
                        *visible_floating_button = Some(button);
                    } else {
                        clear_selection_and_hide_button(app);
                        *pending_selection = None;
                        *visible_floating_button = None;
                    }
                } else if !in_assistant_window {
                    // 不是有效 drag，清除选区
                    clear_selection_and_hide_button(app);
                    *pending_selection = None;
                    *visible_floating_button = None;
                }
                // 在助手窗口内时，保持选区不变
            }
        }
        MouseButtonEvent::Move(_) | MouseButtonEvent::Wheel { .. } => {} // Move/Wheel 事件在上面已处理
    }
}

static MOUSE_EVENT_SENDER: OnceLock<Mutex<Option<mpsc::Sender<MouseButtonEvent>>>> =
    OnceLock::new();

fn drag_selection_hint_rects(down_point: Point, up_point: Point) -> Vec<Rect> {
    const HINT_LINE_HEIGHT: f64 = 36.0;
    const DRAG_Y_TO_TEXT_TOP_OFFSET: f64 = 34.0;
    let x = down_point.x.min(up_point.x);
    let y = (down_point.y.min(up_point.y) - DRAG_Y_TO_TEXT_TOP_OFFSET).max(0.0);
    let width = (down_point.x - up_point.x).abs().max(1.0);

    vec![Rect {
        x,
        y,
        width,
        height: HINT_LINE_HEIGHT,
    }]
}

fn uia_probe_points_for_drag((down_point, up_point): (Point, Point)) -> Vec<Point> {
    let point_at = |ratio: f64| Point {
        x: down_point.x + (up_point.x - down_point.x) * ratio,
        y: down_point.y + (up_point.y - down_point.y) * ratio,
    };

    vec![
        down_point,
        up_point,
        point_at(0.25),
        point_at(0.5),
        point_at(0.75),
    ]
}

fn should_use_selection_hint_rects(hint_rects: &[Rect]) -> bool {
    hint_rects.iter().any(is_valid_rect)
}

fn is_valid_rect(rect: &Rect) -> bool {
    rect.width > 0.0 && rect.height > 0.0
}

#[derive(Debug, Default, Clone, Copy)]
struct ColorBucket {
    count: u32,
    red_sum: u32,
    green_sum: u32,
    blue_sum: u32,
}

#[derive(Debug, Clone, Copy)]
struct RowMatch {
    y: i32,
    min_x: i32,
    max_x: i32,
    count: i32,
}

#[derive(Debug, Clone, Copy)]
struct SelectionCaptureOptions {
    drag_points: Option<(Point, Point)>,
    require_drag_rect_match: bool,
    allow_clipboard_fallback: bool,
}

#[derive(Debug)]
struct SelectionCaptureResult {
    context: crate::commands::selection::PanelContext,
    source_window_handle: isize,
    visual_selection: Option<SelectionVisualState>,
}

impl SelectionCaptureOptions {
    fn from_mouse_drag(down_point: Point, up_point: Point) -> Self {
        Self {
            drag_points: Some((down_point, up_point)),
            require_drag_rect_match: true,
            allow_clipboard_fallback: false,
        }
    }

    fn from_explicit_hotkey() -> Self {
        Self {
            drag_points: None,
            require_drag_rect_match: false,
            allow_clipboard_fallback: true,
        }
    }
}

/// 一块已经读到内存里的屏幕像素。
///
/// 之前的实现对搜索区域内的**每个像素**调用一次 `GetPixel`：一次滚动修正的
/// 搜索带约 930x660，即约 61 万次 GDI 往返，实测量级在数百毫秒到数秒之间，
/// 远超跟随逻辑给它的时间预算，因此修正几乎从来没有真正生效过。
///
/// 改成一次 `BitBlt` + `GetDIBits` 把区域读进内存后按数组扫描，
/// 同样的区域只需要一次 GDI 调用，成本降到毫秒级。
struct PixelBuffer {
    origin_x: i32,
    origin_y: i32,
    width: i32,
    height: i32,
    bgra: Vec<u8>,
}

impl PixelBuffer {
    /// 从屏幕 DC 抓取指定区域。
    ///
    /// 使用屏幕 DC 而不是 `GetWindowDC(hwnd)`：GPU 合成的窗口
    /// （Electron / Chromium，例如 VS Code）经常无法通过窗口 DC 读回内容。
    fn capture(rect: Rect) -> Option<Self> {
        let origin_x = rect.x.floor() as i32;
        let origin_y = rect.y.floor() as i32;
        let width = rect.width.ceil() as i32;
        let height = rect.height.ceil() as i32;
        if width <= 0 || height <= 0 || width > MAX_CAPTURE_WIDTH || height > MAX_CAPTURE_HEIGHT {
            return None;
        }

        let screen_dc = unsafe { GetDC(null_mut()) };
        if screen_dc.is_null() {
            return None;
        }
        let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
        if memory_dc.is_null() {
            unsafe { ReleaseDC(null_mut(), screen_dc) };
            return None;
        }
        let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(null_mut(), screen_dc);
            }
            return None;
        }

        let previous = unsafe { SelectObject(memory_dc, bitmap) };
        let copied = unsafe {
            BitBlt(
                memory_dc, 0, 0, width, height, screen_dc, origin_x, origin_y, SRCCOPY,
            )
        };

        // GetDIBits 要求目标位图**不能**处于选中状态（MSDN 明确规定），
        // 严格的显卡驱动会直接返回 0，导致整个视觉检测静默失效。
        // 所以在读取像素之前先把原位图选回去。
        unsafe { SelectObject(memory_dc, previous) };

        let mut bgra = vec![0_u8; (width as usize) * (height as usize) * 4];
        let scanlines = if copied == 0 {
            0
        } else {
            let mut bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: (width * height * 4) as u32,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };
            unsafe {
                GetDIBits(
                    memory_dc,
                    bitmap,
                    0,
                    height as u32,
                    bgra.as_mut_ptr() as *mut c_void,
                    &mut bitmap_info,
                    DIB_RGB_COLORS,
                )
            }
        };

        unsafe {
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }

        (scanlines != 0).then_some(Self {
            origin_x,
            origin_y,
            width,
            height,
            bgra,
        })
    }

    fn pixel(&self, screen_x: i32, screen_y: i32) -> Option<(u8, u8, u8)> {
        let local_x = screen_x - self.origin_x;
        let local_y = screen_y - self.origin_y;
        if local_x < 0 || local_y < 0 || local_x >= self.width || local_y >= self.height {
            return None;
        }

        let index = ((local_y as usize) * (self.width as usize) + local_x as usize) * 4;
        let pixel = self.bgra.get(index..index + 4)?;
        Some((pixel[2], pixel[1], pixel[0]))
    }
}

/// 首次拖拽选字后，用像素扫描定位选区高亮。
///
/// `excluded_rects` 不能省：`PixelBuffer` 抓的是**合成后的屏幕**而不是窗口 DC，
/// 助手自己的置顶窗口（上一次残留的操作条、结果面板等）只要与搜索带相交，
/// 其像素就会一起参与高亮配色扫描，可能被当成选区本身——之后滚动跟随的就是
/// 那个悬浮窗，而不是文字。跟踪路径一直传了排除矩形，首次捕获同样需要。
fn visual_selection_from_drag(
    source_window_handle: isize,
    down_point: Point,
    up_point: Point,
    excluded_rects: &[Rect],
) -> Option<SelectionVisualState> {
    let window_rect = source_window_screen_rect(source_window_handle)?;
    let search_rect = drag_visual_search_rect(window_rect, down_point, up_point)?;
    let buffer = PixelBuffer::capture(search_rect)?;

    let mid_point = Point {
        x: (down_point.x + up_point.x) / 2.0,
        y: (down_point.y + up_point.y) / 2.0,
    };
    let points = [down_point, up_point, mid_point];
    let color =
        sample_selection_color(&buffer, &points).filter(selection_color_looks_like_highlight)?;

    find_visual_selection_rect_in_buffer(
        &buffer,
        search_rect,
        color,
        Some(down_point.y.min(up_point.y)),
        Some(down_point.x.min(up_point.x)),
        excluded_rects,
    )
    .map(|rect| SelectionVisualState {
        source_window_handle,
        color,
        rect,
    })
}

/// 在指定搜索区域内重新定位已知颜色的选区高亮。
///
/// `excluded_rects` 用于排除助手自己的置顶窗口：屏幕 DC 抓到的像素里
/// 也包含迷你操作条本身，不排除的话它可能把自己的像素当成选区。
fn visual_selection_within(
    visual: SelectionVisualState,
    search_rect: Rect,
    excluded_rects: &[Rect],
) -> Option<SelectionVisualState> {
    let buffer = PixelBuffer::capture(search_rect)?;
    find_visual_selection_rect_in_buffer(
        &buffer,
        search_rect,
        visual.color,
        None,
        Some(visual.rect.x + visual.rect.width / 2.0),
        excluded_rects,
    )
    .map(|rect| SelectionVisualState { rect, ..visual })
}

/// 以预测位置为中心的窄搜索带。
///
/// 跟踪时已经知道选区大概会落在哪里，只需要覆盖预测误差，
/// 因此比拖拽首次定位用的搜索区小得多。
fn tracked_visual_search_rect(
    window_rect: Rect,
    predicted_rect: Rect,
    previous_rect: Rect,
) -> Option<Rect> {
    let top = predicted_rect.y.min(previous_rect.y) - SCROLL_TRACK_SEARCH_PADDING_Y;
    let bottom = (predicted_rect.y + predicted_rect.height)
        .max(previous_rect.y + previous_rect.height)
        + SCROLL_TRACK_SEARCH_PADDING_Y;
    let left = predicted_rect.x.min(previous_rect.x) - SCROLL_TRACK_SEARCH_PADDING_X;
    let right = (predicted_rect.x + predicted_rect.width)
        .max(previous_rect.x + previous_rect.width)
        + SCROLL_TRACK_SEARCH_PADDING_X;

    intersect_rects(
        Rect {
            x: left,
            y: top,
            width: (right - left).max(1.0),
            height: (bottom - top).max(1.0),
        },
        window_rect,
    )
}

/// 当前所有显示器的屏幕坐标范围；取不到时返回空表，调用方自行退化。
fn monitor_screen_rects(app: &tauri::AppHandle) -> Vec<Rect> {
    let Ok(monitors) = app.available_monitors() else {
        return Vec::new();
    };

    monitors
        .iter()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            Rect {
                x: position.x as f64,
                y: position.y as f64,
                width: size.width as f64,
                height: size.height as f64,
            }
        })
        .collect()
}

fn source_window_screen_rect(source_window_handle: isize) -> Option<Rect> {
    let hwnd = source_window_handle as *mut c_void;
    let mut rect = WinRect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
    if ok == 0 || rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }

    Some(Rect {
        x: rect.left as f64,
        y: rect.top as f64,
        width: (rect.right - rect.left) as f64,
        height: (rect.bottom - rect.top) as f64,
    })
}

/// 拖拽选字时的搜索区：纵向覆盖拖拽行上下，横向以拖拽范围为中心适度外扩。
///
/// 横向**不能**直接用整个窗口宽度：8K 屏或跨显示器的窗口宽度可能超过
/// `MAX_CAPTURE_WIDTH`，那样每次捕获都会被上限直接拒掉，视觉检测彻底失效——
/// 而自动划词路径恰恰在 UIA 拿不到选区时才依赖它，且不允许剪贴板兜底。
fn drag_visual_search_rect(window_rect: Rect, down_point: Point, up_point: Point) -> Option<Rect> {
    const DRAG_SEARCH_PADDING_X: f64 = 320.0;

    let top = down_point.y.min(up_point.y) - 140.0;
    let bottom = down_point.y.max(up_point.y) + 220.0;
    let left = down_point.x.min(up_point.x) - DRAG_SEARCH_PADDING_X;
    let right = down_point.x.max(up_point.x) + DRAG_SEARCH_PADDING_X;

    intersect_rects(
        Rect {
            x: left,
            y: top,
            width: (right - left).max(1.0),
            height: (bottom - top).max(1.0),
        },
        window_rect,
    )
}

fn intersect_rects(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > left && bottom > top).then_some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn sample_selection_color(buffer: &PixelBuffer, points: &[Point]) -> Option<(u8, u8, u8)> {
    let mut buckets: HashMap<(u8, u8, u8), ColorBucket> = HashMap::new();

    for point in points {
        for y_offset in (-12..=12).step_by(3) {
            for x_offset in (-12..=12).step_by(3) {
                let screen_x = point.x.round() as i32 + x_offset;
                let screen_y = point.y.round() as i32 + y_offset;
                let Some((red, green, blue)) = buffer.pixel(screen_x, screen_y) else {
                    continue;
                };
                let key = (red / 16, green / 16, blue / 16);
                let bucket = buckets.entry(key).or_default();
                bucket.count += 1;
                bucket.red_sum += red as u32;
                bucket.green_sum += green as u32;
                bucket.blue_sum += blue as u32;
            }
        }
    }

    let bucket = buckets
        .values()
        .filter(|bucket| bucket.count >= 6)
        .max_by_key(|bucket| bucket.count)?;
    Some((
        (bucket.red_sum / bucket.count) as u8,
        (bucket.green_sum / bucket.count) as u8,
        (bucket.blue_sum / bucket.count) as u8,
    ))
}

fn find_visual_selection_rect_in_buffer(
    buffer: &PixelBuffer,
    search_rect: Rect,
    color: (u8, u8, u8),
    preferred_y: Option<f64>,
    preferred_x: Option<f64>,
    excluded_rects: &[Rect],
) -> Option<Rect> {
    let left = search_rect.x.round() as i32;
    let right = (search_rect.x + search_rect.width).round() as i32;
    let top = search_rect.y.round() as i32;
    let bottom = (search_rect.y + search_rect.height).round() as i32;
    let mut row_matches = Vec::new();

    for y in top..bottom {
        let mut min_x: Option<i32> = None;
        let mut max_x: Option<i32> = None;
        let mut count = 0;
        let mut current_run = 0;
        let mut max_run = 0;

        for x in left..right {
            let matches = !point_in_any_rect(x, y, excluded_rects)
                && buffer
                    .pixel(x, y)
                    .map(|pixel| colors_are_close(pixel, color))
                    .unwrap_or(false);
            if matches {
                min_x = Some(min_x.map_or(x, |current| current.min(x)));
                max_x = Some(max_x.map_or(x, |current| current.max(x)));
                count += 1;
                current_run += 1;
                max_run = max_run.max(current_run);
            } else {
                current_run = 0;
            }
        }

        if count >= 18 && max_run >= 14 {
            row_matches.push(RowMatch {
                y,
                min_x: min_x.unwrap_or(left),
                max_x: max_x.unwrap_or(left),
                count,
            });
        }
    }

    let mut candidates = Vec::new();
    let mut index = 0;
    while index < row_matches.len() {
        let mut top_y = row_matches[index].y;
        let mut bottom_y = row_matches[index].y;
        let mut min_x = row_matches[index].min_x;
        let mut max_x = row_matches[index].max_x;
        let mut total_count = row_matches[index].count;
        index += 1;

        while index < row_matches.len() && row_matches[index].y - bottom_y <= 4 {
            bottom_y = row_matches[index].y;
            min_x = min_x.min(row_matches[index].min_x);
            max_x = max_x.max(row_matches[index].max_x);
            total_count += row_matches[index].count;
            index += 1;
        }

        let width = (max_x - min_x + 1) as f64;
        let height = (bottom_y - top_y + 1) as f64;
        if width >= 20.0 && height >= 8.0 && total_count >= 160 {
            candidates.push(Rect {
                x: min_x as f64,
                y: top_y as f64,
                width,
                height,
            });
        }
        top_y = bottom_y;
        let _ = top_y;
    }

    candidates.into_iter().min_by(|a, b| {
        let score_a = visual_rect_score(*a, preferred_y, preferred_x);
        let score_b = visual_rect_score(*b, preferred_y, preferred_x);
        score_a
            .partial_cmp(&score_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn visual_rect_score(rect: Rect, preferred_y: Option<f64>, preferred_x: Option<f64>) -> f64 {
    let center_x = rect.x + rect.width / 2.0;
    let center_y = rect.y + rect.height / 2.0;
    let y_score = preferred_y.map_or(rect.y * 0.02, |y| (center_y - y).abs());
    let x_score = preferred_x.map_or(0.0, |x| (center_x - x).abs() * 0.15);
    y_score + x_score - rect.width.min(900.0) * 0.01
}

fn point_in_rect(x: i32, y: i32, rect: Rect) -> bool {
    x as f64 >= rect.x
        && x as f64 <= rect.x + rect.width
        && y as f64 >= rect.y
        && y as f64 <= rect.y + rect.height
}

fn point_in_any_rect(x: i32, y: i32, rects: &[Rect]) -> bool {
    rects.iter().any(|rect| point_in_rect(x, y, *rect))
}

fn selection_color_looks_like_highlight(color: &(u8, u8, u8)) -> bool {
    let (red, green, blue) = *color;
    let brightness = red as u16 + green as u16 + blue as u16;
    let strongest_gap = red.max(green).max(blue) - red.min(green).min(blue);

    brightness > 90 && brightness < 700 && strongest_gap >= 24
}

fn colors_are_close(a: (u8, u8, u8), b: (u8, u8, u8)) -> bool {
    let red = a.0.abs_diff(b.0) as u16;
    let green = a.1.abs_diff(b.1) as u16;
    let blue = a.2.abs_diff(b.2) as u16;
    red <= 28 && green <= 28 && blue <= 28 && red + green + blue <= 72
}

fn start_low_level_mouse_hook(sender: mpsc::Sender<MouseButtonEvent>) {
    thread::spawn(move || {
        let sender_slot = MOUSE_EVENT_SENDER.get_or_init(|| Mutex::new(None));
        if let Ok(mut slot) = sender_slot.lock() {
            *slot = Some(sender);
        } else {
            return;
        }

        let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), null_mut(), 0) };
        if hook.is_null() {
            trace_selection_monitor(format_args!("failed to install low-level mouse hook"));
            if let Ok(mut slot) = sender_slot.lock() {
                *slot = None;
            }
            return;
        }
        trace_selection_monitor(format_args!("low-level mouse hook installed"));

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

unsafe extern "system" fn mouse_hook_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if code >= 0 {
        let hook_event = match w_param as u32 {
            WM_LBUTTONDOWN => Some(MouseButtonEvent::Down(mouse_hook_point(l_param))),
            WM_LBUTTONUP => Some(MouseButtonEvent::Up(mouse_hook_point(l_param))),
            WM_MOUSEMOVE => Some(MouseButtonEvent::Move(mouse_hook_point(l_param))),
            WM_MOUSEWHEEL => Some(MouseButtonEvent::Wheel {
                position: mouse_hook_point(l_param),
                delta: mouse_hook_wheel_delta(l_param),
            }),
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

unsafe fn mouse_hook_wheel_delta(l_param: LPARAM) -> f64 {
    let hook = &*(l_param as *const MSLLHOOKSTRUCT);
    ((hook.mouseData >> 16) as u16 as i16) as f64
}

fn capture_store_and_show_floating_button(
    app: &tauri::AppHandle,
    anchor: Point,
    selection_hint_rects: &[Rect],
    drag_points: Option<(Point, Point)>,
) -> Option<VisibleFloatingButton> {
    let config = match current_config(app) {
        Some(config) => config,
        None => {
            trace_selection_monitor(format_args!("capture failed: config unavailable"));
            return None;
        }
    };

    let options = drag_points
        .map(|(down_point, up_point)| {
            SelectionCaptureOptions::from_mouse_drag(down_point, up_point)
        })
        .unwrap_or_else(SelectionCaptureOptions::from_explicit_hotkey);
    let SelectionCaptureResult {
        context,
        source_window_handle,
        visual_selection,
    } = match read_current_selection_context(anchor, &config, options, &assistant_window_rects(app))
    {
        Some(result) => result,
        None => {
            trace_selection_monitor(format_args!("capture failed: no selection context"));
            return None;
        }
    };
    let uses_visual_selection = visual_selection.is_some();
    let uses_selection_hint =
        !uses_visual_selection && should_use_selection_hint_rects(selection_hint_rects);
    let toolbar_selection_rects = if let Some(visual) = visual_selection {
        trace_selection_monitor(format_args!(
            "floating button placement uses visual selection: rect={:?}, color={:?}, uia_rects={}",
            visual.rect,
            visual.color,
            context.selection.selection_rects.len()
        ));
        vec![visual.rect]
    } else if uses_selection_hint {
        trace_selection_monitor(format_args!(
            "floating button placement uses drag hint: uia_rects={}, hint_rects={}",
            context.selection.selection_rects.len(),
            selection_hint_rects.len()
        ));
        selection_hint_rects.to_vec()
    } else {
        context.selection.selection_rects.clone()
    };
    let toolbar_anchor = if uses_visual_selection || uses_selection_hint {
        toolbar_selection_rects
            .iter()
            .copied()
            .find(is_valid_rect)
            .map(|rect| Point {
                x: rect.x,
                y: rect.y,
            })
            .unwrap_or_else(|| context.selection.toolbar_anchor_point())
    } else {
        context.selection.toolbar_anchor_point()
    };
    let selection_rect = toolbar_selection_rects
        .iter()
        .copied()
        .find(is_valid_rect)
        .map(scroll_follow_placement_rect);
    let state = app.state::<AppState>();
    state.store_latest_selection(context.clone());
    state.store_latest_selection_window_handle(source_window_handle);
    if let Some(visual) = visual_selection {
        state.store_latest_selection_visual(visual);
    } else {
        state.clear_latest_selection_visual();
    }
    emit_context_if_panel_visible(app, &context);
    // 先自增 generation 再显示，顺序不能反。
    //
    // floating-button 是共享窗口。反过来的话，旧跟随会话可能在「校验 generation
    // 通过」之后、我们自增之前完成它的 show，把窗口挪回旧选区的位置；而新选区
    // 这边已经显示完毕，不会再摆一次，操作条就停在错的地方（issue #44 第 8 条）。
    // 先自增则旧会话的事务必然校验失败，根本不会执行 show。
    //
    // 显示失败时 generation 也已经自增：这没有副作用——选区确实换了，旧会话本来
    // 就该作废。
    next_scroll_follow_generation();
    match show_floating_button_for_selection(app.clone(), toolbar_anchor, &toolbar_selection_rects)
    {
        Ok(()) => {
            trace_selection_monitor(format_args!(
                "floating button shown: toolbar_anchor={toolbar_anchor:?}, text_len={}",
                context.selection.text.chars().count()
            ));
            let window_position = floating_button_window_position(app).unwrap_or(toolbar_anchor);
            let scroll_follow_enabled = should_follow_scroll_for_source(
                &context.selection.source_app,
                &context.selection.window_title,
            );
            state.store_latest_floating_button_window_position(window_position);
            Some(VisibleFloatingButton {
                window_position,
                selection_anchor: toolbar_anchor,
                selection_rect,
                scroll_follow_enabled,
            })
        }
        Err(error) => {
            trace_selection_monitor(format_args!(
                "capture failed: show_floating_button error: {error:?}"
            ));
            None
        }
    }
}

fn scroll_follow_placement_rect(rect: Rect) -> Rect {
    Rect {
        height: rect.height.min(SCROLL_FOLLOW_MAX_PLACEMENT_HEIGHT).max(1.0),
        ..rect
    }
}

fn floating_button_window_position(app: &tauri::AppHandle) -> Option<Point> {
    let window = app.get_webview_window("floating-button")?;
    let position = window.outer_position().ok()?;
    Some(Point {
        x: position.x as f64,
        y: position.y as f64,
    })
}

fn clear_selection_and_hide_button(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    // 让正在运行的跟随会话立刻作废：generation 现在只代表“选区身份”，
    // 不再每次滚轮都自增。
    next_scroll_follow_generation();
    state.clear_latest_selection();
    let _ = hide_floating_button(app.clone());
}

/// 一次滚动跟随会话的共享状态。
///
/// 旧实现为**每个滚轮事件**都 spawn 一个线程，并用 generation 让新事件取消
/// 上一个线程；连续滚动时修正永远来不及落地。现在整段滚动只有一个跟随线程，
/// 它按固定节奏测量选区真实位置，因此误差不会随滚动累积。
struct ScrollTracker {
    session_id: u64,
    running: bool,
    generation: u64,
    started_at: Instant,
    last_wheel_at: Instant,
    pace: ScrollPace,
    /// 自上次成功测量以来累计的滚轮 delta，用于反推真实滚动比例。
    pending_wheel_delta: f64,
    /// 当前认为选区所在的矩形（预测或测量得到）。
    tracked_rect: Rect,
    /// `tracked_rect` 是否来自真实测量。预测值不能当作下一轮的位移基线。
    tracked_rect_measured: bool,
    /// 滚轮事件序号，每来一个事件自增。
    ///
    /// 测量在锁外进行，期间可能又来了新的滚轮事件（甚至把节奏推成快速滚动
    /// 并隐藏了操作条）。此时这次测量算出来的位置已经过期，序号对不上就丢弃，
    /// 否则会把操作条重新显示在旧位置上，并把 `hidden` 复位成 false。
    wheel_seq: u64,
    /// 上一次真实测量得到的 y，用于判断动画是否已经停下来。
    last_measured_y: f64,
    /// 上一次「动画已停」时的 y，作为学习滚动比例的位移起点。
    ///
    /// 比例必须用整段位移去算：动画中途的采样只走完了一小部分，
    /// 拿它除以整段 wheel delta 会把比例算小一个数量级。
    ratio_anchor_y: f64,
    /// 这一段位移是否全程都在测量之下。
    ///
    /// 快速滚动期间不测量（操作条已隐藏），但滚轮 delta 仍在累加。此时
    /// 「整段位移 ÷ 整段 delta」是假的：分母涨了几十格，分子却只有恢复
    /// 测量后那一点点。实测这会把比例从 0.82 拖到 0.52，必须丢弃。
    ratio_burst_valid: bool,
    failures: u32,
    /// 静止之后是否已经测到一次「位置不再变化」的结果。
    ///
    /// 不能只看「静止后测过一次」：平滑滚动动画可能比静止阈值更长，
    /// 那一次测量拿到的是动画中途的位置。
    settled_measurement_stable: bool,
    /// 是否因为快速滚动而处于隐藏状态，用于避免重复调用 hide。
    hidden: bool,
    /// 本次会话是否以「放弃」结束（选区滚出视口，或连续测量失败）。
    ///
    /// 放弃之后不能再凭预测把操作条显示出来：选区已经确认找不到了，
    /// 那样只会在无关内容上凭空冒出一个幽灵操作条。必须等真实测量成功。
    abandoned: bool,
    process_name: String,
    source_window_handle: isize,
}

/// 一次采样从锁里拷出来的会话状态。
///
/// 不含 `generation`：会话是否仍代表当前选区，由事务在锁内直接比对
/// `tracker.generation` 与 `ScrollFollowState::generation` 判定，
/// 快照带一份出来只会诱使调用方在锁外做二次判断。
#[derive(Debug, Clone)]
struct ScrollTrackerSnapshot {
    started_at: Instant,
    last_wheel_at: Instant,
    pace: ScrollPace,
    pending_wheel_delta: f64,
    tracked_rect: Rect,
    last_measured_y: f64,
    ratio_anchor_y: f64,
    ratio_burst_valid: bool,
    wheel_seq: u64,
    tracked_rect_measured: bool,
    failures: u32,
    settled_measurement_stable: bool,
    hidden: bool,
    process_name: String,
    source_window_handle: isize,
}

/// 滚动跟随的全部共享状态。
///
/// `generation` 代表「当前是哪一个选区」，原先住在 `AppState` 的独立
/// Mutex 里。它和会话状态分属两把锁时，一次 tick 只能先读 generation、
/// 再去加 tracker 的锁提交——两者之间始终有窗口，新选区正好落在里面
/// 就会让旧会话把它的 `latest_selection_visual` 覆盖掉、甚至把共享操作条
/// 挪回旧位置。补校验点只能缩窄这个窗口，关不掉它（见 issue #44）。
///
/// 放进同一把锁之后，「校验 → 决策 → 改状态」可以在一个临界区内完成。
struct ScrollFollowState {
    generation: u64,
    tracker: Option<ScrollTracker>,
}

fn scroll_follow_state() -> &'static Mutex<ScrollFollowState> {
    static STATE: OnceLock<Mutex<ScrollFollowState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(ScrollFollowState {
            generation: 0,
            tracker: None,
        })
    })
}

/// 换了选区：作废正在跑的跟随会话。
fn next_scroll_follow_generation() {
    if let Ok(mut state) = scroll_follow_state().lock() {
        state.generation = state.generation.saturating_add(1);
    }
}

fn next_scroll_session_id() -> u64 {
    static NEXT_ID: OnceLock<Mutex<u64>> = OnceLock::new();
    let slot = NEXT_ID.get_or_init(|| Mutex::new(0));
    let mut id = slot.lock().expect("scroll session id mutex poisoned");
    *id = id.saturating_add(1);
    *id
}

/// 每个来源进程学习到的滚动比例。
fn scroll_ratio_cache() -> &'static Mutex<HashMap<String, ScrollRatioEstimate>> {
    static CACHE: OnceLock<Mutex<HashMap<String, ScrollRatioEstimate>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_scroll_ratio(process_name: &str) -> Option<ScrollRatioEstimate> {
    scroll_ratio_cache().lock().ok()?.get(process_name).copied()
}

fn store_scroll_ratio(process_name: &str, estimate: ScrollRatioEstimate) {
    if let Ok(mut cache) = scroll_ratio_cache().lock() {
        cache.insert(process_name.to_string(), estimate);
    }
}

fn scroll_tracker_snapshot(session_id: u64) -> Option<ScrollTrackerSnapshot> {
    let guard = scroll_follow_state().lock().ok()?;
    let tracker = guard.tracker.as_ref()?;
    if tracker.session_id != session_id || !tracker.running {
        return None;
    }
    // 会话开始后选区就换了：这一轮不必再测。
    if tracker.generation != guard.generation {
        return None;
    }

    Some(ScrollTrackerSnapshot {
        started_at: tracker.started_at,
        last_wheel_at: tracker.last_wheel_at,
        pace: tracker.pace,
        pending_wheel_delta: tracker.pending_wheel_delta,
        tracked_rect: tracker.tracked_rect,
        last_measured_y: tracker.last_measured_y,
        ratio_anchor_y: tracker.ratio_anchor_y,
        ratio_burst_valid: tracker.ratio_burst_valid,
        wheel_seq: tracker.wheel_seq,
        tracked_rect_measured: tracker.tracked_rect_measured,
        failures: tracker.failures,
        hidden: tracker.hidden,
        settled_measurement_stable: tracker.settled_measurement_stable,
        process_name: tracker.process_name.clone(),
        source_window_handle: tracker.source_window_handle,
    })
}

/// 系统把滚轮事件送给哪个窗口，取决于「滚动非活动窗口」这项设置。
///
/// 不能简单地「光标在来源窗口内 **或** 来源窗口是前台」就放行——两个方向
/// 都会误判：设置开启时来源窗口可能是前台、但光标在别的应用上（滚的是别人）；
/// 设置关闭时光标可能停在非活动的来源窗口上（滚的其实是前台窗口）。
/// 必须按当前路由方式判断真正的接收者。
fn wheel_routes_to_focused_window() -> bool {
    const SPI_GETMOUSEWHEELROUTING: u32 = 0x201C;
    /// 滚轮送给焦点窗口。
    const MOUSEWHEEL_ROUTING_FOCUS: u32 = 0;
    /// 混合模式：Store 应用送指针下的窗口，**桌面应用送焦点窗口**。
    /// 我们跟随的来源全部是桌面应用，所以这里等同于焦点路由。
    const MOUSEWHEEL_ROUTING_HYBRID: u32 = 1;
    /// 滚轮送给指针下的窗口。
    const MOUSEWHEEL_ROUTING_MOUSE_POS: u32 = 2;

    let mut routing: u32 = u32::MAX;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETMOUSEWHEELROUTING,
            0,
            &mut routing as *mut u32 as *mut c_void,
            0,
        )
    };
    if ok == 0 {
        // 读不到就按 Windows 10+ 的默认观感（送给指针下的窗口）处理。
        return false;
    }

    match routing {
        MOUSEWHEEL_ROUTING_FOCUS | MOUSEWHEEL_ROUTING_HYBRID => true,
        MOUSEWHEEL_ROUTING_MOUSE_POS => false,
        // 未知取值同样退回指针路由。
        _ => false,
    }
}

/// 本次滚轮是否带着「这不是纵向滚动」的修饰键。
///
/// Ctrl+滚轮几乎都是缩放，Shift+滚轮几乎都是横向滚动。
fn wheel_has_non_vertical_modifier() -> bool {
    key_down(VK_CONTROL as i32) || key_down(VK_SHIFT as i32)
}

/// 光标位置所属的顶层窗口。
fn root_window_at_point(point: Point) -> isize {
    let hwnd = unsafe {
        WindowFromPoint(POINT {
            x: point.x.round() as i32,
            y: point.y.round() as i32,
        })
    };
    if hwnd.is_null() {
        return 0;
    }
    let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
    if root.is_null() {
        hwnd as isize
    } else {
        root as isize
    }
}

/// 这次滚轮事件是不是真的作用在选区所在的窗口上。
fn wheel_targets_source_window(source_window_handle: isize, wheel_position: Point) -> bool {
    if wheel_routes_to_focused_window() {
        let foreground = unsafe { GetForegroundWindow() };
        return !foreground.is_null() && foreground as isize == source_window_handle;
    }

    root_window_at_point(wheel_position) == source_window_handle
}

fn follow_visible_floating_button_after_scroll(
    app: &tauri::AppHandle,
    visible_floating_button: &mut Option<VisibleFloatingButton>,
    scroll_burst: &mut ScrollBurst,
    wheel_position: Point,
    wheel_delta: f64,
    now_ms: u64,
) {
    let Some(visible) = visible_floating_button.as_mut() else {
        return;
    };
    if !visible.scroll_follow_enabled {
        trace_selection_monitor(format_args!(
            "floating button scroll follow skipped for fixed-source selection"
        ));
        return;
    }

    let state = app.state::<AppState>();
    let Some(source_window_handle) = state.latest_selection_window_handle() else {
        return;
    };
    // Ctrl+滚轮通常是缩放、Shift+滚轮通常是横向滚动，都不是纵向滚动。
    // 按纵向处理会预测出一个凭空的位移；缩放还会同时改变选区的宽高，
    // 而跟随只平移 y、沿用旧的 x/width，结果必然是错的。这类手势直接不跟。
    if wheel_has_non_vertical_modifier() {
        trace_selection_monitor(format_args!(
            "scroll ignored: modifier wheel (zoom / horizontal), not vertical scrolling"
        ));
        return;
    }

    // 低层鼠标钩子是全局的：在别的窗口上滚动同样会走到这里。不加判断的话，
    // 滚动任意一个无关窗口都会驱动跟随——操作条被预测位移带偏、快滚还会
    // 把它整个隐藏掉，而来源窗口其实一动没动。
    if !wheel_targets_source_window(source_window_handle, wheel_position) {
        trace_selection_monitor(format_args!(
            "scroll ignored: wheel at {:?} is not over the selection source window",
            wheel_position
        ));
        return;
    }

    let process_name = state
        .latest_selection()
        .map(|context| context.selection.source_app)
        .unwrap_or_default();

    // 起始矩形优先取跟随会话里最近一次的测量结果，其次才是事件循环里
    // 记录的预测值，避免上一轮滚动的预测误差被当成新的起点。
    //
    // 只有真正测量过的矩形才配当基线：上一轮若是因连续测量失败而放弃，
    // 留下的 tracked_rect 是没验证过的预测值，拿它当位移起点会把上一轮的
    // 误差算进这一轮的 wheel delta，污染整个进程的比例缓存。
    //
    // 但「未经测量」只在**复用已停止/已放弃的会话**时才是问题。会话还在跑时，
    // tracked_rect 是本轮突发滚动累积下来的预测值，必须沿用：否则每个事件都
    // 从上一次测量重新起算，预测只前进一格。视觉跟踪的搜索带只有 ±160px，
    // 连续多格快滚会直接把选区甩出搜索带，每次扫描都落空、最终误判为跟丢。
    // 注意：`abandoned` 必须**独立于**几何资格判断读取。
    // 六次测量全失败而放弃时，running 和 tracked_rect_measured 同时为 false，
    // 若把它塞进几何资格的 then_some 里，恰好就是这个标记要保护的场景被过滤掉，
    // 幽灵操作条照样会冒出来。
    let (reusable_tracked_rect, baseline_measured, recovering_from_abandon) = scroll_follow_state()
        .lock()
        .ok()
        .and_then(|guard| {
            let generation = guard.generation;
            guard.tracker.as_ref().map(|tracker| {
                if tracker.generation != generation {
                    return (None, false, false);
                }
                let abandoned = !tracker.running && tracker.abandoned;
                // 会话仍在运行 => 沿用累积预测；已停止 => 只认测量过的几何。
                let usable = tracker.running || tracker.tracked_rect_measured;
                (
                    usable.then_some(tracker.tracked_rect),
                    tracker.tracked_rect_measured,
                    abandoned,
                )
            })
        })
        .unwrap_or((None, false, false));
    let Some(current_rect) = reusable_tracked_rect
        .or_else(|| {
            state
                .latest_selection_visual()
                .map(|visual| scroll_follow_placement_rect(visual.rect))
        })
        .or(visible.selection_rect)
    else {
        return;
    };

    let pace = scroll_burst.register(wheel_delta, now_ms);
    let predicted_delta_y =
        predicted_scroll_offset(cached_scroll_ratio(&process_name), wheel_delta);
    let predicted_rect = Rect {
        y: current_rect.y + predicted_delta_y,
        ..current_rect
    };

    visible.selection_rect = Some(predicted_rect);
    visible.selection_anchor.y += predicted_delta_y;

    let session_id = next_scroll_session_id();
    let mut started_session = None;
    let mut hide_now = false;
    if let Ok(mut guard) = scroll_follow_state().lock() {
        let generation = guard.generation;
        let restart = guard
            .tracker
            .as_ref()
            .map(|tracker| !tracker.running || tracker.generation != generation)
            .unwrap_or(true);

        if restart {
            guard.tracker = Some(ScrollTracker {
                session_id,
                running: true,
                generation,
                started_at: Instant::now(),
                last_wheel_at: Instant::now(),
                pace,
                pending_wheel_delta: wheel_delta,
                tracked_rect: predicted_rect,
                tracked_rect_measured: false,
                last_measured_y: current_rect.y,
                ratio_anchor_y: current_rect.y,
                // 快速滚动一开始就不测量；基线若不是测量得来的，同样不能
                // 拿这段位移去学比例——等第一次稳定测量重新锚定后再说。
                ratio_burst_valid: pace != ScrollPace::Fast && baseline_measured,
                wheel_seq: 1,
                abandoned: false,
                failures: 0,
                settled_measurement_stable: false,
                // 上一轮是放弃收场的话，本轮同样先保持隐藏：等真实测量成功
                // 再显示，不能凭预测把操作条摆回一个已经找不到的选区上。
                hidden: pace == ScrollPace::Fast || recovering_from_abandon,
                process_name,
                source_window_handle,
            });
            started_session = Some(session_id);
            hide_now = pace == ScrollPace::Fast;
        } else if let Some(tracker) = guard.tracker.as_mut() {
            tracker.last_wheel_at = Instant::now();
            tracker.pace = pace;
            tracker.pending_wheel_delta += wheel_delta;
            tracker.tracked_rect = predicted_rect;
            tracker.tracked_rect_measured = false;
            tracker.wheel_seq = tracker.wheel_seq.saturating_add(1);
            tracker.settled_measurement_stable = false;
            // 只在进入快速滚动的那一刻隐藏一次，避免每个滚轮事件都调用 hide。
            hide_now = pace == ScrollPace::Fast && !tracker.hidden;
            if hide_now {
                tracker.hidden = true;
            }
        }
    }

    if pace == ScrollPace::Fast {
        // 快速滚动：不在没验证过的位置上渲染操作条，先隐藏，静止后再吸附。
        if hide_now {
            let _ = hide_floating_button(app.clone());
            trace_selection_monitor(format_args!(
                "floating button hidden during fast scroll: notches={:.2}",
                scroll_burst.accumulated_notches()
            ));
        }
    } else if recovering_from_abandon {
        // 上一轮已经确认选区找不到了，这一轮先不显示，等测量把它找回来。
        trace_selection_monitor(format_args!(
            "scroll follow restarted after abandon; waiting for a real measurement"
        ));
    } else if let Ok(position) = floating_button_position_for_selection(
        app,
        Point {
            x: predicted_rect.x,
            y: predicted_rect.y,
        },
        &[predicted_rect],
    ) {
        if show_floating_button_at_position(app.clone(), position).is_ok() {
            visible.window_position = position;
            state.store_latest_floating_button_window_position(position);
        }
    }

    if let Some(session_id) = started_session {
        spawn_scroll_tracker(app.clone(), session_id);
    }
}

fn spawn_scroll_tracker(app: tauri::AppHandle, session_id: u64) {
    thread::spawn(move || {
        let mut sample_index: u64 = 0;
        loop {
            thread::sleep(SCROLL_TRACK_SAMPLE_INTERVAL);
            sample_index = sample_index.saturating_add(1);

            // 会话被取代（换了选区、或换了跟随会话）时快照直接取不到，
            // generation 的校验已经在取快照的那把锁里做掉了。
            let Some(snapshot) = scroll_tracker_snapshot(session_id) else {
                return;
            };
            if snapshot.started_at.elapsed() > SCROLL_TRACK_MAX_SESSION {
                // 安全阀触发时并不代表已经吸附成功。持续快滚 20 秒的场景里
                // tracked_rect 只是预测值、测量还被刻意跳过，按成功收尾会把
                // 猜出来的几何平移进选区上下文并推给面板。只有确实测量过才
                // 允许提交，否则一律按放弃处理。
                let measured = snapshot.tracked_rect_measured;
                trace_selection_monitor(format_args!(
                    "scroll tracking hit the {}s safety timeout (measured={measured})",
                    SCROLL_TRACK_MAX_SESSION.as_secs()
                ));
                // 安全阀是硬停：不校验 wheel_seq，否则一直滚下去就永远停不了。
                finish_scroll_tracker(&app, session_id, None, !measured);
                return;
            }

            let idle_ms = snapshot.last_wheel_at.elapsed().as_millis() as u64;
            match scroll_tracker_action(
                snapshot.pace,
                idle_ms,
                snapshot.failures,
                snapshot.settled_measurement_stable,
            ) {
                ScrollTrackerAction::WaitHidden => {
                    // 这一帧没测量，本段位移不再完整，作废比例学习窗口。
                    invalidate_scroll_ratio_burst(session_id);
                    continue;
                }
                ScrollTrackerAction::Finish => {
                    finish_scroll_tracker(&app, session_id, Some(snapshot.wheel_seq), false);
                    return;
                }
                ScrollTrackerAction::Abandon => {
                    trace_selection_monitor(format_args!(
                        "floating button hidden after {} failed scroll measurements",
                        snapshot.failures
                    ));
                    finish_scroll_tracker(&app, session_id, Some(snapshot.wheel_seq), true);
                    return;
                }
                ScrollTrackerAction::Measure => {
                    let settled = idle_ms >= SCROLL_SETTLE_IDLE_MS;
                    // UIA 是跨进程 COM 调用，成本远高于像素测量，不能每帧都试。
                    let allow_uia = sample_index % UIA_MEASURE_SAMPLE_STRIDE == 0 || settled;
                    measure_and_follow_selection(&app, session_id, &snapshot, allow_uia, settled);
                }
            }
        }
    });
}

fn measure_and_follow_selection(
    app: &tauri::AppHandle,
    session_id: u64,
    snapshot: &ScrollTrackerSnapshot,
    allow_uia: bool,
    settled: bool,
) {
    let (measured_rect, measured_visual) =
        match measure_tracked_selection_rect(app, snapshot, allow_uia) {
            MeasureOutcome::Measured { rect, visual } => (rect, visual),
            MeasureOutcome::Failed => {
                record_scroll_measure_failure(session_id, snapshot.wheel_seq);
                return;
            }
            // 这一帧被节流跳过，什么都没发生，不能算跟丢。
            MeasureOutcome::Skipped => return,
        };

    // 只跟随纵向位移：横向沿用原选区的 x/width，避免检测抖动让操作条左右乱跳。
    let placement_rect = Rect {
        x: snapshot.tracked_rect.x,
        width: snapshot.tracked_rect.width,
        y: measured_rect.y,
        height: measured_rect.height,
    };

    let trackable =
        source_window_screen_rect(snapshot.source_window_handle).is_none_or(|window_rect| {
            selection_still_trackable_on_monitors(
                placement_rect,
                window_rect,
                &monitor_screen_rects(app),
            )
        });

    // 相邻两次测量几乎没有位移 => 应用的滚动动画已经播完。
    let step_delta_y = measured_rect.y - snapshot.last_measured_y;
    let measurement_is_stable = settled_measurement_is_stable(settled, step_delta_y);

    // 因快速滚动而隐藏的操作条，必须等动画真的停下来才能重新出现。空闲时间
    // 一到就把它显示在中途位置上，它会跟着剩余动画一路追，正是要消灭的观感。
    //
    // 这两项都只依赖快照，是纯函数，锁外算好再交给事务，事务里不再重算。
    let will_show = !snapshot.hidden || measurement_is_stable;

    // 位置同样在锁外算：它只依赖 placement_rect 与显示器几何，不碰会话状态。
    // 只在确实要显示时才算——枚举显示器有成本，不显示的帧没必要付。
    let position = if trackable && will_show {
        match floating_button_position_for_selection(
            app,
            Point {
                x: placement_rect.x,
                y: placement_rect.y,
            },
            &[placement_rect],
        ) {
            Ok(position) => Some(position),
            Err(_) => {
                record_scroll_measure_failure(session_id, snapshot.wheel_seq);
                return;
            }
        }
    } else {
        None
    };

    // 到这里为止全部在锁外。下面一次加锁，把校验、判定、状态变更和视觉状态
    // 写回一并做掉；锁外只按返回的结果执行副作用，不再重新读状态。
    let state = app.state::<AppState>();
    let commit = apply_scroll_measurement(
        &state,
        session_id,
        snapshot,
        measured_rect,
        placement_rect,
        measured_visual,
        trackable,
        measurement_is_stable,
        will_show,
    );

    let position = match commit {
        ScrollFollowCommit::Stale => return,
        ScrollFollowCommit::Abandon => {
            trace_selection_monitor(format_args!(
                "floating button hidden: selection scrolled out of source window"
            ));
            let _ = hide_floating_button(app.clone());
            return;
        }
        // 快滚尚未停稳：测量已记下，但不在未验证的位置上渲染。
        ScrollFollowCommit::MeasuredOnly => return,
        ScrollFollowCommit::Show => {
            let Some(position) = position else {
                return;
            };
            position
        }
    };

    if show_floating_button_at_position(app.clone(), position).is_err() {
        // 提交已经把 settled_measurement_stable 置真、hidden 清零。若就此返回，
        // 下一轮会直接 Finish 收尾，而操作条其实根本没显示出来——快滚之后
        // 一次偶发的 show 失败就会让它永久消失。回滚以便重试；hidden 只在
        // 本来就是隐藏状态时才恢复，否则会误判屏幕上仍在的旧操作条。
        rollback_failed_show(session_id, snapshot.hidden);
        record_scroll_measure_failure(session_id, snapshot.wheel_seq);
        return;
    }

    state.store_latest_floating_button_window_position(position);

    // show 期间若又来滚轮事件要求隐藏，把刚显示出来的收回去。
    //
    // 这里**不能**因为 generation 变了就隐藏：floating-button 是共享窗口。
    // 换选区的路径现在先自增 generation 再显示操作条，所以走到这一步、
    // generation 却已经变了，只可能是新主人已经把窗口摆到了它自己的位置上。
    // 隐藏等于把那个位置正确的操作条删掉，而监视循环仍记录它可见，不会自动
    // 再显示。旧会话只管自己那一份。
    let superseded_and_hidden = scroll_follow_state()
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .tracker
                .as_ref()
                .map(|tracker| tracker.session_id == session_id && tracker.hidden)
        })
        .unwrap_or(false);
    if superseded_and_hidden {
        let _ = hide_floating_button(app.clone());
        return;
    }

    // 只在动画停下来之后学习比例，且用「整段位移 ÷ 整段 wheel delta」。
    // 用动画中途的单帧位移去算会把比例算小一个数量级（实测记事本 0.09
    // 对真实 0.69），预测因此永远追不上真实滚动步长。
    if measurement_is_stable && snapshot.ratio_burst_valid {
        let burst_delta_y = measured_rect.y - snapshot.ratio_anchor_y;
        if let Some(estimate) = update_scroll_ratio(
            cached_scroll_ratio(&snapshot.process_name),
            snapshot.pending_wheel_delta,
            burst_delta_y,
        ) {
            store_scroll_ratio(&snapshot.process_name, estimate);
            trace_selection_monitor(format_args!(
                "scroll ratio for {}: {:.3} px/delta after {} samples (burst dy={:.1}, delta={:.1})",
                snapshot.process_name,
                estimate.pixels_per_delta,
                estimate.samples,
                burst_delta_y,
                snapshot.pending_wheel_delta
            ));
        }
    }
}

/// 一次测量在锁内做出的落地决定。锁外只按这个结果执行副作用。
enum ScrollFollowCommit {
    /// 快照已过期：会话被取代、期间来了新滚轮、或选区已经换了。什么都不做。
    Stale,
    /// 选区已不在可见区域内。状态已在锁内标记为放弃，调用方只需隐藏操作条。
    Abandon,
    /// 测量已记下，但这一帧不显示（快滚尚未停稳）。
    MeasuredOnly,
    /// 提交并显示。
    Show,
}

/// 把一次测量的「校验 → 判定 → 改状态」收敛进单个临界区。
///
/// 测量必须在锁外做（UIA 是跨进程 COM 调用，像素扫描要抓屏），结果天然可能
/// 过期。此前的做法是测完之后分多次加锁逐项校验：先 `scroll_wheel_seq_unchanged`
/// 再 `finish_scroll_tracker`、先读 generation 再 `commit_scroll_measurement`。
/// 每一层都只把竞态窗口缩窄到「两次相邻加锁之间」，关不掉它——issue #44 的
/// 第 1、7、8 条都是这么来的，而且后期几乎每条都是在补上一条修复留下的窗口。
///
/// 这里改成单一事务：三项校验（session_id / wheel_seq / generation）、放弃与
/// 提交的判定、状态变更、以及视觉状态写回，全部在同一次加锁内完成。调用方拿到
/// 结果后不再重新读状态做二次判断。
#[allow(clippy::too_many_arguments)]
fn apply_scroll_measurement(
    state: &AppState,
    session_id: u64,
    snapshot: &ScrollTrackerSnapshot,
    measured_rect: Rect,
    placement_rect: Rect,
    measured_visual: Option<SelectionVisualState>,
    trackable: bool,
    measurement_is_stable: bool,
    will_show: bool,
) -> ScrollFollowCommit {
    let Ok(mut guard) = scroll_follow_state().lock() else {
        return ScrollFollowCommit::Stale;
    };
    let generation = guard.generation;
    let Some(tracker) = guard.tracker.as_mut() else {
        return ScrollFollowCommit::Stale;
    };

    // 三项校验一次做完：
    // - session_id：这一轮跟随会话是否还是发起测量的那一个；
    // - wheel_seq：测量期间有没有来新的滚轮事件（结果是否已被更新的预测取代）；
    // - generation：选区有没有被换掉（旧会话不得再碰新选区的任何状态）。
    if tracker.session_id != session_id
        || tracker.wheel_seq != snapshot.wheel_seq
        || tracker.generation != generation
    {
        return ScrollFollowCommit::Stale;
    }

    if !trackable {
        // 放弃是**持久**动作：下一轮也要保持隐藏。它与上面的校验同处一个临界区，
        // 不会再出现「校验通过后、标记停止前来了反向滚轮，却仍把选区判死」。
        tracker.running = false;
        tracker.abandoned = true;
        return ScrollFollowCommit::Abandon;
    }

    tracker.tracked_rect = placement_rect;
    tracker.tracked_rect_measured = true;
    tracker.last_measured_y = measured_rect.y;
    if measurement_is_stable && will_show {
        // 这一段滚动结算完毕，重置位移起点。只扣掉本次快照已计入的 delta，
        // 测量期间新到的滚轮事件要保留。无论这段是否可信都要重置：
        // 不可信的那段更不能留着累加。
        tracker.pending_wheel_delta -= snapshot.pending_wheel_delta;
        tracker.ratio_anchor_y = measured_rect.y;
        tracker.ratio_burst_valid = true;
    }
    tracker.failures = 0;
    // 只有「已经静止 + 这一帧几乎没再动 + 确实显示了」才算吸附完成。
    tracker.settled_measurement_stable = measurement_is_stable && will_show;

    if !will_show {
        return ScrollFollowCommit::MeasuredOnly;
    }
    tracker.hidden = false;

    // 视觉状态的写回也放进这个临界区。
    //
    // 它原先在锁外做，于是「generation 校验通过」与「写回」之间存在窗口：
    // 替换选区若正好落在里面，旧会话就会用自己那份视觉状态盖掉新选区的
    // （issue #44 第 8 条）。这里 generation 已在上面校验过且锁未释放，
    // 写回不可能再落到新选区头上。
    //
    // 与原实现一致，只在确实显示这一帧时才写回：不显示的帧沿用上一份视觉
    // 状态作为下次扫描的搜索种子。这是行为保持，不是竞态相关的取舍。
    //
    // 锁序：follow-state → AppState 的单个字段锁。反向嵌套在代码里不存在，
    // 且这里不做任何系统调用，不会因窗口消息重入而自锁。
    if let Some(visual) = measured_visual {
        state.store_latest_selection_visual(visual);
    }

    ScrollFollowCommit::Show
}

/// 回滚一次提交：show 实际上失败了。
///
/// 不回滚 `settled_measurement_stable` 的话，下一轮会因为它为真而直接收尾，
/// 一次偶发的 show 失败就变成操作条永久消失。
///
/// `restore_hidden` 只在**本次调用之前操作条确实处于隐藏状态**时才置回 true。
/// 普通慢滚样本 show 失败时并没有执行过隐藏，屏幕上那个旧操作条还在——
/// 此时若把 hidden 置真，后续不稳定样本会一直不显示，而滚动一旦转为快速，
/// 滚轮处理里 `!tracker.hidden` 为假会跳过 hide，旧操作条就整段快滚都留在
/// 屏幕上。
fn rollback_failed_show(session_id: u64, restore_hidden: bool) {
    if let Ok(mut guard) = scroll_follow_state().lock() {
        if let Some(tracker) = guard.tracker.as_mut() {
            if tracker.session_id == session_id {
                tracker.settled_measurement_stable = false;
                if restore_hidden {
                    tracker.hidden = true;
                }
            }
        }
    }
}

/// 作废当前的比例学习窗口：这一段位移中间有没测到的空档。
fn invalidate_scroll_ratio_burst(session_id: u64) {
    if let Ok(mut guard) = scroll_follow_state().lock() {
        if let Some(tracker) = guard.tracker.as_mut() {
            if tracker.session_id == session_id {
                tracker.ratio_burst_valid = false;
            }
        }
    }
}

/// 记一次测量失败。
///
/// 同样要校验 `wheel_seq`：失败结果和成功结果一样可能是过期的。只比对
/// session_id 的话，连续滚动时几次被取代的 UIA 尝试就能把失败数累到上限，
/// 放弃一个在当前位置上从未失败过的选区。
fn record_scroll_measure_failure(session_id: u64, wheel_seq: u64) {
    if let Ok(mut guard) = scroll_follow_state().lock() {
        if let Some(tracker) = guard.tracker.as_mut() {
            if tracker.session_id == session_id && tracker.wheel_seq == wheel_seq {
                tracker.failures = tracker.failures.saturating_add(1);
            }
        }
    }
}

/// 一次测量的结果。
///
/// 必须区分「测了但没测到」和「这一帧根本没测」：UIA 有采样节流，
/// 被节流跳过的帧如果也计入失败次数，纯 UIA 选区（没有视觉高亮可用）
/// 会在几帧之内耗尽失败额度，操作条被误判为跟丢而隐藏。
enum MeasureOutcome {
    /// 测到了位置；`visual` 非空时表示这次是像素扫描的结果，
    /// 需要在确认快照仍然有效之后再写回全局状态。
    Measured {
        rect: Rect,
        visual: Option<SelectionVisualState>,
    },
    Failed,
    Skipped,
}

/// 测量选区当前真实位置：优先视觉高亮（对 UIA 不可用的应用也有效），
/// 失败时回退到 UIA。
fn measure_tracked_selection_rect(
    app: &tauri::AppHandle,
    snapshot: &ScrollTrackerSnapshot,
    allow_uia: bool,
) -> MeasureOutcome {
    let state = app.state::<AppState>();
    let excluded_rects = assistant_window_rects(app);
    let has_visual_state = state.latest_selection_visual().is_some();

    if let Some(visual) = state.latest_selection_visual() {
        if let Some(window_rect) = source_window_screen_rect(visual.source_window_handle) {
            if let Some(search_rect) =
                tracked_visual_search_rect(window_rect, snapshot.tracked_rect, visual.rect)
            {
                if let Some(found) = visual_selection_within(visual, search_rect, &excluded_rects) {
                    // 不在这里写回：扫描期间选区可能已经被替换，
                    // 立刻落盘会用旧选区的视觉状态覆盖掉新选区的。
                    return MeasureOutcome::Measured {
                        rect: scroll_follow_placement_rect(found.rect),
                        visual: Some(found),
                    };
                }
            }
        }
    }

    // 这一帧不允许走 UIA：属于「没测」，不是「测失败」。
    if !allow_uia {
        return MeasureOutcome::Skipped;
    }

    // 来源窗口不在前台时不查 UIA。
    //
    // read_current_uia_selection_from_hwnd 会先取 GetFocusedElement，并在选择
    // 结果时优先采用它。指针路由下用户可以滚动一个未获焦点的来源窗口，此时
    // 焦点应用若也有选中文本，拿回来的就是**别的应用**的选区：文本不同会连续
    // 累计失败、把一个仍然有效的操作条隐藏掉；文本恰好相同则会把操作条吸到
    // 毫不相干的几何上。
    let source_is_foreground = {
        let foreground = unsafe { GetForegroundWindow() };
        !foreground.is_null() && foreground as isize == snapshot.source_window_handle
    };
    if !source_is_foreground {
        // 有视觉状态可用时，这一帧退回「没测」，交给像素测量接手即可。
        // 但**没有**视觉状态时不能一直 Skipped：那样既不推进稳定性也不计失败，
        // 慢速路径会持续显示未经验证的预测，直到 20 秒安全阀才收场。
        // 此时唯一的测量手段已经不可用，按失败处理，让失败上限尽快接管。
        return if has_visual_state {
            MeasureOutcome::Skipped
        } else {
            MeasureOutcome::Failed
        };
    }

    let Some(context) = state.latest_selection() else {
        return MeasureOutcome::Failed;
    };
    let Some(uia_result) =
        read_current_uia_selection_from_hwnd(snapshot.source_window_handle as *mut c_void)
    else {
        return MeasureOutcome::Failed;
    };
    if let Some(text) = uia_result.text.as_ref() {
        if text.trim() != context.selection.text.trim() {
            return MeasureOutcome::Failed;
        }
    }
    uia_result
        .rects
        .iter()
        .copied()
        .find(is_valid_rect)
        .map(scroll_follow_placement_rect)
        .map_or(MeasureOutcome::Failed, |rect| MeasureOutcome::Measured {
            rect,
            visual: None,
        })
}

/// 结束跟随会话。`abandon` 表示无法确认选区位置，此时隐藏操作条而不是
/// 把它留在一个猜测出来的位置上。
///
/// `expected_wheel_seq` 是发起收尾的那次采样看到的滚轮序号，`None` 表示
/// 不校验（只有安全阀这种硬停才该这么做）。停止会话是**持久**动作：读出快照
/// 到这里之间若来了新滚轮，这次收尾结论就已经过期。以前只比对 `session_id`，
/// 于是稳定收尾那一格滚动只有预测没有测量（issue #44 第 1 条）；放弃路径更糟，
/// 反向滚动把选区带回视口时会被 latch 成 `abandoned`（第 7 条）。
///
/// generation 与会话同处一把锁，一并在这里校验：选区已经换掉的旧会话不得再
/// 把自己那份几何平移进新选区的上下文。
fn finish_scroll_tracker(
    app: &tauri::AppHandle,
    session_id: u64,
    expected_wheel_seq: Option<u64>,
    abandon: bool,
) {
    let tracked_rect;
    {
        let Ok(mut guard) = scroll_follow_state().lock() else {
            return;
        };
        let generation = guard.generation;
        let Some(tracker) = guard.tracker.as_mut() else {
            return;
        };
        if tracker.session_id != session_id || tracker.generation != generation {
            return;
        }
        if expected_wheel_seq.is_some_and(|expected| tracker.wheel_seq != expected) {
            return;
        }
        tracker.running = false;
        tracker.abandoned = abandon;
        tracked_rect = tracker.tracked_rect;
    }

    let state = app.state::<AppState>();

    if abandon {
        let _ = hide_floating_button(app.clone());
        return;
    }

    // 静止后把最终位置同步回选区上下文，供面板等后续逻辑使用。
    let Some(mut context) = state.latest_selection() else {
        return;
    };
    // 整体平移原有的全部选区矩形，而不是用这一个替换掉它们。
    //
    // tracked_rect 只是「首行的放置矩形」，高度还被 scroll_follow_placement_rect
    // 压到了 36px。多行选区如果被它替换，后续翻译结果窗口之类的定位就只能看到
    // 顶部一条带，侧边放不下时会盖住没被表示出来的下方行。
    let scrolled_delta_y = context
        .selection
        .selection_rects
        .iter()
        .copied()
        .find(is_valid_rect)
        .map(|first| tracked_rect.y - first.y);
    match scrolled_delta_y {
        Some(delta_y) => {
            for rect in context.selection.selection_rects.iter_mut() {
                rect.y += delta_y;
            }
        }
        // 原本就没有可用的几何信息，只能退回到单个矩形。
        None => context.selection.selection_rects = vec![tracked_rect],
    }
    context.selection.explicit_anchor = None;
    let anchor_point = context.selection.anchor_point();
    context.selection.explicit_anchor = Some(anchor_point);
    context.selection.fallback_point = anchor_point;
    state.store_latest_selection(context.clone());
    emit_context_if_panel_visible(app, &context);
}

/// 滚动结束后把新几何同步给面板。
///
/// **必须清掉 `auto_run`**：面板是从操作条打开的，上下文里 `auto_run` 为 true；
/// 前端见到 `autoRun === true` 就会再跑一次 AI 请求。仅仅滚动一下选区就重复
/// 发起一次计费请求显然不对——这里只是几何刷新，不是新的执行意图。
fn emit_context_if_panel_visible(
    app: &tauri::AppHandle,
    context: &crate::commands::selection::PanelContext,
) {
    let Some(window) = app.get_webview_window("ai-panel") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = emit_panel_context(app, &panel_context_for_visible_refresh(context));
    }
}

fn assistant_window_rects(app: &tauri::AppHandle) -> Vec<Rect> {
    [
        "floating-button",
        "replacement-preset",
        "ai-panel",
        "source-text",
        "translate-result",
        "screenshot-overlay",
    ]
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
    options: SelectionCaptureOptions,
    assistant_rects: &[Rect],
) -> Option<SelectionCaptureResult> {
    let (window, source_window_handle) = match foreground_window_info(config) {
        Some(result) => result,
        None => {
            trace_selection_monitor(format_args!(
                "selection read failed: foreground window unavailable"
            ));
            return None;
        }
    };
    trace_selection_monitor(format_args!(
        "foreground window: process={}, title={:?}, elevated={}, fallback_point={fallback_point:?}",
        window.process_name, window.window_title, window.elevated
    ));
    let visual_selection = options.drag_points.and_then(|(down_point, up_point)| {
        visual_selection_from_drag(source_window_handle, down_point, up_point, assistant_rects)
    });
    let uia_points = options
        .drag_points
        .map(uia_probe_points_for_drag)
        .unwrap_or_default();
    let uia_result = if uia_points.is_empty() {
        read_current_uia_selection_from_hwnd(source_window_handle as *mut c_void)
    } else {
        read_current_uia_selection_from_hwnd_with_points(
            source_window_handle as *mut c_void,
            &uia_points,
        )
    };
    if let Some(selection) = uia_result
        .clone()
        .filter(|result| result.is_usable())
        .and_then(|result| {
            SelectionCandidate::from_uia_result(
                result,
                window.process_name.clone(),
                window.window_title.clone(),
                fallback_point,
            )
        })
    {
        let has_visual_selection = visual_selection.is_some();
        let matches_drag = options
            .drag_points
            .map(|(down_point, up_point)| {
                selection_geometry_matches_drag_gesture(
                    &selection.selection_rects,
                    down_point,
                    up_point,
                    has_visual_selection,
                )
            })
            .unwrap_or(true);

        if !options.require_drag_rect_match || matches_drag {
            trace_selection_monitor(format_args!(
                "selection read succeeded via UIA: chars={}, rects={}, visual_selection={}",
                selection.text.chars().count(),
                selection.selection_rects.len(),
                has_visual_selection
            ));
            return match create_panel_context_for_selection(selection, false) {
                Ok(context) => Some(SelectionCaptureResult {
                    context,
                    source_window_handle,
                    visual_selection,
                }),
                Err(error) => {
                    trace_selection_monitor(format_args!(
                        "selection read failed: UIA context error: {error:?}"
                    ));
                    None
                }
            };
        }

        trace_selection_monitor(format_args!(
            "selection read skipped UIA: geometry does not match current drag gesture"
        ));
    }

    if should_block_clipboard_fallback_after_uia_result(uia_result.as_ref()) {
        trace_selection_monitor(format_args!(
            "selection read failed: UIA reported password control; clipboard fallback blocked"
        ));
        return None;
    }

    let fallback_context = ClipboardFallbackContext {
        clipboard_fallback_enabled: config.clipboard_fallback_enabled,
        process_name: window.process_name.clone(),
        disabled_apps: config.disabled_apps.clone(),
        is_password_control: false,
        is_elevated_window: window.elevated,
        disable_in_elevated_windows: config.disable_in_elevated_windows,
    };

    if !options.allow_clipboard_fallback {
        trace_selection_monitor(format_args!(
            "selection read skipped clipboard fallback for automatic mouse selection"
        ));
        return None;
    }

    if !should_use_clipboard_fallback(&fallback_context) {
        trace_selection_monitor(format_args!(
            "selection read failed: clipboard fallback disabled for process={} elevated={}",
            fallback_context.process_name, fallback_context.is_elevated_window
        ));
        return None;
    }

    let text = match copy_selection_with_clipboard_restore() {
        Some(text) => text,
        None => {
            trace_selection_monitor(format_args!(
                "selection read failed: clipboard copy returned no text"
            ));
            return None;
        }
    };
    trace_selection_monitor(format_args!(
        "selection read succeeded: chars={}",
        text.chars().count()
    ));
    let selection = SelectionCandidate::from_clipboard_text(
        text,
        window.process_name,
        window.window_title,
        fallback_point,
    );
    match create_panel_context_for_selection(selection, false) {
        Ok(context) => Some(SelectionCaptureResult {
            context,
            source_window_handle,
            visual_selection,
        }),
        Err(error) => {
            trace_selection_monitor(format_args!(
                "selection read failed: context error: {error:?}"
            ));
            None
        }
    }
}

pub fn capture_screen_region_png_data_url(rect: Rect) -> Result<String, crate::types::PublicError> {
    let width = rect.width.round().max(1.0) as i32;
    let height = rect.height.round().max(1.0) as i32;
    let x = rect.x.round() as i32;
    let y = rect.y.round() as i32;

    let screen_dc = unsafe { GetDC(null_mut()) };
    if screen_dc.is_null() {
        return Err(public_error(
            "screenshot_capture_failed",
            "无法读取屏幕 DC。",
        ));
    }

    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if memory_dc.is_null() {
        unsafe {
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err(public_error(
            "screenshot_capture_failed",
            "无法创建截图缓冲区。",
        ));
    }

    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err(public_error(
            "screenshot_capture_failed",
            "无法创建截图位图。",
        ));
    }

    let previous = unsafe { SelectObject(memory_dc, bitmap) };
    let copied = unsafe { BitBlt(memory_dc, 0, 0, width, height, screen_dc, x, y, SRCCOPY) };
    if copied == 0 {
        unsafe {
            SelectObject(memory_dc, previous);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err(public_error(
            "screenshot_capture_failed",
            "截图区域读取失败。",
        ));
    }

    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: (width * height * 4) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    };
    let mut bgra = vec![0_u8; (width * height * 4) as usize];
    let scanlines = unsafe {
        GetDIBits(
            memory_dc,
            bitmap,
            0,
            height as u32,
            bgra.as_mut_ptr() as *mut c_void,
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    };

    unsafe {
        SelectObject(memory_dc, previous);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(null_mut(), screen_dc);
    }

    if scanlines == 0 {
        return Err(public_error(
            "screenshot_capture_failed",
            "截图像素读取失败。",
        ));
    }

    let mut rgba = Vec::with_capacity(bgra.len());
    for pixel in bgra.chunks_exact(4) {
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
    }

    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|err| public_error("screenshot_encode_failed", err))?;
        writer
            .write_image_data(&rgba)
            .map_err(|err| public_error("screenshot_encode_failed", err))?;
    }

    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png_bytes)
    ))
}

fn public_error(code: &str, err: impl ToString) -> crate::types::PublicError {
    crate::types::PublicError {
        code: code.to_string(),
        message: err.to_string(),
    }
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

fn foreground_window_info(config: &AppConfig) -> Option<(AppWindowInfo, isize)> {
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

    Some((
        AppWindowInfo {
            process_name,
            window_title: window_title(hwnd).unwrap_or_else(|| "Unknown window".to_string()),
            elevated,
        },
        hwnd as isize,
    ))
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
    Some(String::from_utf16_lossy(&buffer[..len as usize])).filter(|title| !title.trim().is_empty())
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
    let restore_plan = clipboard_restore_plan();
    if restore_plan.is_none() {
        trace_selection_monitor(format_args!(
            "clipboard capture: original clipboard cannot be restored; selected text may remain on clipboard"
        ));
    }

    send_ctrl_c();
    thread::sleep(Duration::from_millis(120));

    let after_sequence = clipboard_sequence_number();
    let sequence_changed = after_sequence != before_sequence;
    let selected = if sequence_changed {
        let selected = read_clipboard_unicode().map(|text| text.trim().to_string());
        trace_selection_monitor(format_args!(
            "clipboard changed: before={before_sequence}, after={after_sequence}, chars={}",
            selected
                .as_deref()
                .map(str::chars)
                .map(Iterator::count)
                .unwrap_or(0)
        ));
        selected
    } else {
        trace_selection_monitor(format_args!(
            "clipboard did not change after Ctrl+C: sequence={before_sequence}"
        ));
        None
    };

    let restore_status = match restore_plan {
        Some(plan) => {
            let restored_clipboard = restore_clipboard_with_retry(plan);
            trace_selection_monitor(format_args!(
                "clipboard restore result: restored_original_clipboard={restored_clipboard}"
            ));
            if restored_clipboard {
                ClipboardRestoreStatus::RestoredOriginal
            } else {
                ClipboardRestoreStatus::RestoreFailed
            }
        }
        None => ClipboardRestoreStatus::OriginalUnavailable,
    };

    should_accept_selected_text_after_capture(selected.as_deref(), restore_status)
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

        if format_count == 0 {
            Some(ClipboardRestorePlan::Empty)
        } else {
            snapshot_clipboard_formats(format_count as u32).map(ClipboardRestorePlan::Formats)
        }
    })
    .flatten()
}

unsafe fn snapshot_clipboard_formats(format_count: u32) -> Option<Vec<ClipboardFormatSnapshot>> {
    let mut snapshots = Vec::with_capacity(format_count as usize);
    let mut format = 0u32;

    loop {
        format = EnumClipboardFormats(format);
        if format == 0 {
            break;
        }

        let handle = GetClipboardData(format);
        if handle.is_null() {
            return None;
        }

        let size = GlobalSize(handle);
        if size == 0 {
            return None;
        }

        let ptr = GlobalLock(handle) as *const u8;
        if ptr.is_null() {
            return None;
        }

        let data = std::slice::from_raw_parts(ptr, size).to_vec();
        GlobalUnlock(handle);
        snapshots.push(ClipboardFormatSnapshot { format, data });
    }

    if snapshots.len() == format_count as usize {
        Some(snapshots)
    } else {
        None
    }
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
        ClipboardRestorePlan::Formats(formats) => write_clipboard_formats(&formats),
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

fn write_clipboard_formats(formats: &[ClipboardFormatSnapshot]) -> bool {
    with_open_clipboard(|| unsafe {
        let _ = EmptyClipboard();

        for snapshot in formats {
            let handle = GlobalAlloc(GMEM_MOVEABLE, snapshot.data.len());
            if handle.is_null() {
                return false;
            }

            let ptr = GlobalLock(handle) as *mut u8;
            if ptr.is_null() {
                let _ = GlobalFree(handle);
                return false;
            }

            std::ptr::copy_nonoverlapping(snapshot.data.as_ptr(), ptr, snapshot.data.len());
            GlobalUnlock(handle);

            if SetClipboardData(snapshot.format, handle).is_null() {
                let _ = GlobalFree(handle);
                return false;
            }
        }

        true
    })
    .unwrap_or(false)
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
