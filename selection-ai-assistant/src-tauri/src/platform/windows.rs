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
        GetDIBits, GetPixel, GetWindowDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, HDC, RGBQUAD, SRCCOPY,
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
            KEYEVENTF_KEYUP, VK_CONTROL, VK_MENU,
        },
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetCursorPos, GetForegroundWindow, GetMessageW,
            GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, SetWindowsHookExW,
            TranslateMessage, UnhookWindowsHookEx, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
            WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
        },
    },
};

use crate::{
    app_state::{AppState, SelectionVisualState},
    commands::{
        panel::{
            floating_button_position_for_selection, hide_floating_button,
            show_floating_button_at_position, show_floating_button_for_selection,
        },
        screenshot::show_screenshot_overlay_for_point,
        selection::{create_panel_context_for_selection, emit_panel_context},
    },
    config::AppConfig,
    input_monitor::events::{
        consume_pending_selection, handle_hotkey_state,
        hover_action_for_pending_selection_when_idle, manual_hotkey_trigger_key,
        selection_geometry_matches_drag_gesture, should_follow_scroll_for_source,
        visible_floating_button_action_when_idle, HotkeyAction, HotkeyKeyState, MouseButtonEvent,
        PendingHotkeyAction, PendingSelection, PendingSelectionHoverAction, VisibleFloatingButton,
        VisibleFloatingButtonAction,
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
const CLIPBOARD_RESTORE_RETRY_COUNT: usize = 2;
const CLIPBOARD_RESTORE_RETRY_DELAY: Duration = Duration::from_millis(30);
const SCROLL_FOLLOW_RETRY_COUNT: usize = 2;
const SCROLL_FOLLOW_RETRY_DELAY: Duration = Duration::from_millis(50);
const SCROLL_FOLLOW_DEBOUNCE: Duration = Duration::from_millis(120);
const SCROLL_FOLLOW_MAX_PLACEMENT_HEIGHT: f64 = 36.0;
const SCROLL_PREDICT_PIXELS_PER_DELTA: f64 = 0.85;

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
        let mut pending_hotkey = PendingHotkeyAction::default();
        let monitor_started_at = Instant::now();

        loop {
            while let Ok(event) = mouse_rx.try_recv() {
                handle_mouse_event(
                    &app,
                    &mut drag_start,
                    &mut pending_selection,
                    &mut visible_floating_button,
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

fn handle_mouse_event(
    app: &tauri::AppHandle,
    drag_start: &mut Option<Point>,
    pending_selection: &mut Option<PendingSelection>,
    visible_floating_button: &mut Option<VisibleFloatingButton>,
    event: MouseButtonEvent,
    now_ms: u64,
) {
    trace_selection_monitor(format_args!("mouse event: {event:?}"));
    if let MouseButtonEvent::Wheel { position, delta } = event {
        let in_assistant_window = assistant_window_rects(app)
            .iter()
            .any(|window| crate::input_monitor::events::rect_contains(*window, position));
        if !in_assistant_window {
            follow_visible_floating_button_after_scroll(app, visible_floating_button, delta);
        }
        return;
    }

    if let MouseButtonEvent::Move(position) = event {
        emit_floating_button_pointer_position(app, position);
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
                let in_assistant_window = assistant_window_rects(app)
                    .iter()
                    .any(|window| crate::input_monitor::events::rect_contains(*window, up_point));
                trace_selection_monitor(format_args!(
                    "mouse up: down={down_point:?}, up={up_point:?}, min_drag_distance={min_drag_distance}, is_drag_met={is_drag_met}, in_assistant_window={in_assistant_window}"
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

fn visual_selection_from_drag(
    source_window_handle: isize,
    down_point: Point,
    up_point: Point,
) -> Option<SelectionVisualState> {
    let window_rect = source_window_screen_rect(source_window_handle)?;
    let hwnd = source_window_handle as *mut c_void;
    let hdc = unsafe { GetWindowDC(hwnd) };
    if hdc.is_null() {
        return None;
    }

    let mid_point = Point {
        x: (down_point.x + up_point.x) / 2.0,
        y: (down_point.y + up_point.y) / 2.0,
    };
    let points = [down_point, up_point, mid_point];
    let color = sample_selection_color(hdc, window_rect, &points)
        .filter(selection_color_looks_like_highlight);
    let rect = color.and_then(|color| {
        let search_rect = drag_visual_search_rect(window_rect, down_point, up_point)?;
        find_visual_selection_rect_in_dc(
            hdc,
            window_rect,
            search_rect,
            color,
            Some(down_point.y.min(up_point.y)),
            Some(down_point.x.min(up_point.x)),
        )
        .map(|rect| SelectionVisualState {
            source_window_handle,
            color,
            rect,
        })
    });

    unsafe {
        ReleaseDC(hwnd, hdc);
    }
    rect
}

fn visual_selection_from_stored(visual: SelectionVisualState) -> Option<SelectionVisualState> {
    let window_rect = source_window_screen_rect(visual.source_window_handle)?;
    let search_rect = stored_visual_search_rect(window_rect, visual.rect)?;
    let hwnd = visual.source_window_handle as *mut c_void;
    let hdc = unsafe { GetWindowDC(hwnd) };
    if hdc.is_null() {
        return None;
    }

    let rect = find_visual_selection_rect_in_dc(
        hdc,
        window_rect,
        search_rect,
        visual.color,
        None,
        Some(visual.rect.x + visual.rect.width / 2.0),
    )
    .map(|rect| SelectionVisualState { rect, ..visual });

    unsafe {
        ReleaseDC(hwnd, hdc);
    }
    rect
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

fn drag_visual_search_rect(window_rect: Rect, down_point: Point, up_point: Point) -> Option<Rect> {
    let top = down_point.y.min(up_point.y) - 140.0;
    let bottom = down_point.y.max(up_point.y) + 220.0;
    intersect_rects(
        Rect {
            x: window_rect.x,
            y: top,
            width: window_rect.width,
            height: (bottom - top).max(1.0),
        },
        window_rect,
    )
}

fn stored_visual_search_rect(window_rect: Rect, previous_rect: Rect) -> Option<Rect> {
    let left = previous_rect.x - 220.0;
    let right = previous_rect.x + previous_rect.width + 220.0;
    let top = previous_rect.y - 320.0;
    let bottom = previous_rect.y + previous_rect.height + 320.0;
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

fn sample_selection_color(hdc: HDC, window_rect: Rect, points: &[Point]) -> Option<(u8, u8, u8)> {
    let mut buckets: HashMap<(u8, u8, u8), ColorBucket> = HashMap::new();

    for point in points {
        for y_offset in (-12..=12).step_by(3) {
            for x_offset in (-12..=12).step_by(3) {
                let screen_x = point.x.round() as i32 + x_offset;
                let screen_y = point.y.round() as i32 + y_offset;
                if !point_in_rect(screen_x, screen_y, window_rect) {
                    continue;
                }
                let Some((red, green, blue)) = pixel_rgb(hdc, window_rect, screen_x, screen_y)
                else {
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

fn find_visual_selection_rect_in_dc(
    hdc: HDC,
    window_rect: Rect,
    search_rect: Rect,
    color: (u8, u8, u8),
    preferred_y: Option<f64>,
    preferred_x: Option<f64>,
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
            let matches = pixel_rgb(hdc, window_rect, x, y)
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

fn pixel_rgb(hdc: HDC, window_rect: Rect, screen_x: i32, screen_y: i32) -> Option<(u8, u8, u8)> {
    let local_x = screen_x - window_rect.x.round() as i32;
    let local_y = screen_y - window_rect.y.round() as i32;
    if local_x < 0 || local_y < 0 {
        return None;
    }

    let color = unsafe { GetPixel(hdc, local_x, local_y) };
    if color == u32::MAX {
        return None;
    }

    Some((
        (color & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        ((color >> 16) & 0xff) as u8,
    ))
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
    } = match read_current_selection_context(anchor, &config, options) {
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
            state.next_scroll_follow_generation();
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
    app.state::<AppState>().clear_latest_selection();
    let _ = hide_floating_button(app.clone());
}

fn follow_visible_floating_button_after_scroll(
    app: &tauri::AppHandle,
    visible_floating_button: &mut Option<VisibleFloatingButton>,
    wheel_delta: f64,
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
    let generation = state.next_scroll_follow_generation();
    let predicted_delta_y = wheel_delta * SCROLL_PREDICT_PIXELS_PER_DELTA;
    let base_window_position = state
        .latest_floating_button_window_position()
        .or_else(|| floating_button_window_position(app))
        .unwrap_or(visible.window_position);
    let fallback_window_position = Point {
        x: base_window_position.x,
        y: base_window_position.y + predicted_delta_y,
    };
    visible.selection_anchor.y += predicted_delta_y;
    visible.selection_rect = visible.selection_rect.map(|rect| Rect {
        y: rect.y + predicted_delta_y,
        ..rect
    });

    let predicted_visual_rect = visible.selection_rect.or_else(|| {
        state.latest_selection_visual().map(|visual| {
            scroll_follow_placement_rect(Rect {
                y: visual.rect.y + predicted_delta_y,
                ..visual.rect
            })
        })
    });
    visible.window_position = predicted_visual_rect
        .and_then(|rect| {
            floating_button_position_for_selection(app, visible.selection_anchor, &[rect]).ok()
        })
        .unwrap_or(fallback_window_position);
    state.store_latest_floating_button_window_position(visible.window_position);

    if show_floating_button_at_position(app.clone(), visible.window_position).is_ok() {
        trace_selection_monitor(format_args!(
            "floating button predicted scroll follow: generation={generation}, delta_y={predicted_delta_y}, window_position={:?}",
            visible.window_position
        ));
    }

    let app = app.clone();
    let predicted_window_position = visible.window_position;
    thread::spawn(move || {
        thread::sleep(SCROLL_FOLLOW_DEBOUNCE);
        if app.state::<AppState>().scroll_follow_generation() != generation {
            return;
        }
        for attempt in 0..SCROLL_FOLLOW_RETRY_COUNT {
            if let Some(window_position) = refresh_visible_floating_button_from_visual(
                &app,
                predicted_visual_rect,
                predicted_window_position,
            ) {
                trace_selection_monitor(format_args!(
                    "floating button corrected scroll via visual selection: attempt={}, generation={}, window_position={window_position:?}",
                    attempt + 1,
                    generation
                ));
                return;
            }
            if let Some(window_position) = refresh_visible_floating_button_from_uia(
                &app,
                predicted_visual_rect,
                predicted_window_position,
            ) {
                trace_selection_monitor(format_args!(
                    "floating button corrected scroll via UIA: attempt={}, generation={}, window_position={window_position:?}",
                    attempt + 1,
                    generation
                ));
                return;
            }
            thread::sleep(SCROLL_FOLLOW_RETRY_DELAY);
        }

        trace_selection_monitor(format_args!(
            "floating button kept predicted position after scroll: selection rect could not be refreshed"
        ));
    });
}

fn refresh_visible_floating_button_from_visual(
    app: &tauri::AppHandle,
    predicted_rect: Option<Rect>,
    _predicted_window_position: Point,
) -> Option<Point> {
    let state = app.state::<AppState>();
    let visual = state.latest_selection_visual()?;
    let refreshed = predicted_rect
        .and_then(|rect| visual_selection_from_stored(SelectionVisualState { rect, ..visual }))
        .or_else(|| visual_selection_from_stored(visual))?;
    let placement_rect = predicted_rect.map_or(refreshed.rect, |rect| Rect {
        x: rect.x,
        width: rect.width,
        y: refreshed.rect.y,
        height: refreshed.rect.height,
    });
    let toolbar_anchor = Point {
        x: placement_rect.x,
        y: placement_rect.y,
    };
    let positioned =
        floating_button_position_for_selection(app, toolbar_anchor, &[placement_rect]).ok()?;
    let window_position = positioned;
    show_floating_button_at_position(app.clone(), window_position).ok()?;
    state.store_latest_floating_button_window_position(window_position);
    state.store_latest_selection_visual(SelectionVisualState {
        rect: placement_rect,
        ..refreshed
    });
    Some(window_position)
}

fn refresh_visible_floating_button_from_uia(
    app: &tauri::AppHandle,
    predicted_rect: Option<Rect>,
    _predicted_window_position: Point,
) -> Option<Point> {
    let state = app.state::<AppState>();
    let mut context = state.latest_selection()?;
    let source_window_handle = state.latest_selection_window_handle()?;
    let uia_result = read_current_uia_selection_from_hwnd(source_window_handle as *mut c_void)?;
    if uia_result.rects.is_empty() {
        return None;
    }
    if let Some(text) = uia_result.text.as_ref() {
        if text.trim() != context.selection.text.trim() {
            return None;
        }
    }

    context.selection.selection_rects = if let Some(predicted_rect) = predicted_rect {
        let refreshed_rect = uia_result.rects.iter().copied().find(is_valid_rect)?;
        vec![Rect {
            x: predicted_rect.x,
            width: predicted_rect.width,
            y: refreshed_rect.y,
            height: refreshed_rect.height,
        }]
    } else {
        uia_result.rects
    };
    context.selection.explicit_anchor = None;
    let anchor_point = context.selection.anchor_point();
    context.selection.explicit_anchor = Some(anchor_point);
    context.selection.fallback_point = anchor_point;
    let toolbar_anchor = context.selection.toolbar_anchor_point();
    state.store_latest_selection(context.clone());
    state.clear_latest_selection_visual();
    emit_context_if_panel_visible(app, &context);
    let positioned = floating_button_position_for_selection(
        app,
        toolbar_anchor,
        &context.selection.selection_rects,
    )
    .ok()?;
    let window_position = positioned;
    show_floating_button_at_position(app.clone(), window_position).ok()?;
    state.store_latest_floating_button_window_position(window_position);
    Some(window_position)
}

fn emit_context_if_panel_visible(
    app: &tauri::AppHandle,
    context: &crate::commands::selection::PanelContext,
) {
    let Some(window) = app.get_webview_window("ai-panel") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = emit_panel_context(app, context);
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
        visual_selection_from_drag(source_window_handle, down_point, up_point)
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
