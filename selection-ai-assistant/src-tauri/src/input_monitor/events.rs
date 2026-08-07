use serde::{Deserialize, Serialize};

use crate::types::{Point, Rect};

const ESTIMATED_TEXT_TOP_OFFSET: f64 = 12.0;
const DRAG_SELECTION_MATCH_PADDING_X: f64 = 96.0;
const DRAG_SELECTION_MATCH_PADDING_Y: f64 = 72.0;
const CONTROL_RECT_MIN_WIDTH: f64 = 240.0;
const CONTROL_RECT_MIN_HEIGHT: f64 = 48.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputEvent {
    DragEnded { down: Point, up: Point },
    MouseMoved { position: Point },
    HotkeyPressed,
    EscapePressed,
    ForegroundWindowChanged,
}

pub fn is_drag_distance_met(down: Point, up: Point, min_drag_distance: f64) -> bool {
    let dx = up.x - down.x;
    let dy = up.y - down.y;
    ((dx * dx) + (dy * dy)).sqrt() >= min_drag_distance
}

pub fn selection_geometry_matches_drag_gesture(
    rects: &[Rect],
    down: Point,
    up: Point,
    has_visual_selection: bool,
) -> bool {
    has_visual_selection || selection_rects_match_drag_gesture(rects, down, up)
}

pub fn selection_rects_match_drag_gesture(rects: &[Rect], down: Point, up: Point) -> bool {
    let drag_bounds = drag_match_bounds(down, up);

    rects
        .iter()
        .copied()
        .filter(is_valid_selection_rect)
        .any(|rect| {
            !looks_like_control_bounds_for_drag(rect, down, up)
                && rects_intersect(rect, drag_bounds)
        })
}

fn drag_match_bounds(down: Point, up: Point) -> Rect {
    let left = down.x.min(up.x) - DRAG_SELECTION_MATCH_PADDING_X;
    let top = down.y.min(up.y) - DRAG_SELECTION_MATCH_PADDING_Y;
    let right = down.x.max(up.x) + DRAG_SELECTION_MATCH_PADDING_X;
    let bottom = down.y.max(up.y) + DRAG_SELECTION_MATCH_PADDING_Y;

    Rect {
        x: left,
        y: top,
        width: (right - left).max(1.0),
        height: (bottom - top).max(1.0),
    }
}

fn looks_like_control_bounds_for_drag(rect: Rect, down: Point, up: Point) -> bool {
    rect.width >= CONTROL_RECT_MIN_WIDTH
        && rect.height >= CONTROL_RECT_MIN_HEIGHT
        && rect_contains(rect, down)
        && rect_contains(rect, up)
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

/// 两个矩形的重叠部分；不重叠时返回 `None`。
fn rect_intersection(a: Rect, b: Rect) -> Option<Rect> {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    if right <= left || bottom <= top {
        return None;
    }

    Some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn is_valid_selection_rect(rect: &Rect) -> bool {
    rect.width > 0.0 && rect.height > 0.0
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HotkeyKeyState {
    pub ctrl: bool,
    pub alt: bool,
    pub a: bool,
}

impl HotkeyKeyState {
    fn is_chord_down(&self) -> bool {
        self.ctrl && self.alt && self.a
    }

    fn is_release_ready(&self) -> bool {
        !self.ctrl && !self.alt && !self.a
    }
}

pub fn manual_hotkey_trigger_key(hotkey: &str) -> Option<char> {
    let mut has_ctrl = false;
    let mut has_alt = false;
    let mut trigger = None;

    for part in hotkey.split('+').map(|part| part.trim()) {
        if part.eq_ignore_ascii_case("ctrl") || part.eq_ignore_ascii_case("control") {
            has_ctrl = true;
        } else if part.eq_ignore_ascii_case("alt") {
            has_alt = true;
        } else if part.len() == 1 {
            let key = part.chars().next()?.to_ascii_uppercase();
            if key.is_ascii_alphabetic() {
                trigger = Some(key);
            }
        }
    }

    if has_ctrl && has_alt {
        trigger
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PendingHotkeyAction {
    armed: bool,
    keys: HotkeyKeyState,
}

impl PendingHotkeyAction {
    pub fn is_armed(&self) -> bool {
        self.armed
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HotkeyAction {
    Idle,
    Armed,
    AlreadyArmed,
    CaptureAndOpen,
}

pub fn handle_hotkey_state(
    pending_hotkey: &mut PendingHotkeyAction,
    keys: HotkeyKeyState,
) -> HotkeyAction {
    pending_hotkey.keys = keys;

    if keys.is_chord_down() {
        if pending_hotkey.armed {
            HotkeyAction::AlreadyArmed
        } else {
            pending_hotkey.armed = true;
            HotkeyAction::Armed
        }
    } else if pending_hotkey.armed {
        if keys.is_release_ready() {
            pending_hotkey.armed = false;
            HotkeyAction::CaptureAndOpen
        } else {
            HotkeyAction::AlreadyArmed
        }
    } else {
        HotkeyAction::Idle
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseButtonEvent {
    Down(Point),
    Up(Point),
    Move(Point),
    Wheel { position: Point, delta: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseUpAction {
    ArmSelection {
        anchor: Point,
        toolbar_anchor: Point,
    },
    ClearSelection,
    PreserveSelection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingSelection {
    pub anchor: Point,
    pub toolbar_anchor: Point,
    pub hover_started_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibleFloatingButton {
    pub window_position: Point,
    pub selection_anchor: Point,
    pub selection_rect: Option<Rect>,
    pub scroll_follow_enabled: bool,
}

pub fn should_follow_scroll_for_source(source_app: &str, _window_title: &str) -> bool {
    !is_browser_process(source_app)
}

/// 一个标准滚轮刻度的 delta 值。
pub const WHEEL_DELTA_UNIT: f64 = 120.0;
/// 尚未测量出真实滚动比例时的保守初值。
pub const DEFAULT_PIXELS_PER_WHEEL_DELTA: f64 = 0.85;
const MIN_PIXELS_PER_WHEEL_DELTA: f64 = 0.08;
const MAX_PIXELS_PER_WHEEL_DELTA: f64 = 2.2;
const SCROLL_RATIO_SMOOTHING: f64 = 0.4;
const MIN_MEASURABLE_SCROLL_PX: f64 = 2.0;

/// 滚轮节奏的衰减地平线：超过这段时间的历史事件不再计入当前节奏。
pub const FAST_SCROLL_DECAY_WINDOW_MS: f64 = 300.0;
/// 累计刻度数达到该值即进入快速滚动。
pub const FAST_SCROLL_NOTCH_THRESHOLD: f64 = 2.5;
/// 迟滞下沿：进入快速滚动后要降到该值以下才恢复逐帧跟随。
///
/// 没有迟滞的话，速度停在阈值附近时会在隐藏/显示之间反复抖动。
pub const SLOW_RESUME_NOTCH_THRESHOLD: f64 = 1.5;
/// 停止滚动后判定为静止所需的空闲时间。
pub const SCROLL_SETTLE_IDLE_MS: u64 = 150;
/// 连续测量失败多少次后放弃跟随并隐藏操作条。
pub const MAX_SCROLL_MEASURE_FAILURES: u32 = 6;
/// 相邻两次测量的纵向位移小于该值，就认为应用的滚动动画已经停下来了。
///
/// 空闲时间到了不等于画面已经停：Windows 11 记事本这类应用的平滑滚动动画
/// 比 `SCROLL_SETTLE_IDLE_MS` 还长，静止判定触发时内容仍在移动。必须等
/// 位置本身不再变化才能收尾，否则会把动画中途的位置当成最终位置。
pub const SCROLL_SETTLE_STABLE_EPSILON_PX: f64 = 2.0;

/// 每个来源进程的滚动比例估计：一个 wheel delta 单位对应多少屏幕像素。
///
/// 不同应用的滚轮步长差异很大（行高、缩放、平滑滚动设置都会影响），
/// 固定常数必然在大多数应用上偏移，因此改为从真实测量中学习。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollRatioEstimate {
    pub pixels_per_delta: f64,
    pub samples: u32,
}

impl Default for ScrollRatioEstimate {
    fn default() -> Self {
        Self {
            pixels_per_delta: DEFAULT_PIXELS_PER_WHEEL_DELTA,
            samples: 0,
        }
    }
}

impl ScrollRatioEstimate {
    pub fn predicted_offset(&self, wheel_delta: f64) -> f64 {
        wheel_delta * self.pixels_per_delta
    }

    pub fn is_measured(&self) -> bool {
        self.samples > 0
    }
}

pub fn predicted_scroll_offset(estimate: Option<ScrollRatioEstimate>, wheel_delta: f64) -> f64 {
    estimate.unwrap_or_default().predicted_offset(wheel_delta)
}

/// 用一次真实测量修正滚动比例。
///
/// 只有当测量位移足够大、且方向与滚轮方向一致时才采信，避免把
/// 误检到的其它高亮块或抖动写进估计值。
pub fn update_scroll_ratio(
    current: Option<ScrollRatioEstimate>,
    wheel_delta: f64,
    measured_delta_y: f64,
) -> Option<ScrollRatioEstimate> {
    if wheel_delta.abs() < f64::EPSILON {
        return current;
    }
    if measured_delta_y.abs() < MIN_MEASURABLE_SCROLL_PX {
        return current;
    }
    if measured_delta_y.signum() != wheel_delta.signum() {
        return current;
    }

    let observed = (measured_delta_y / wheel_delta)
        .clamp(MIN_PIXELS_PER_WHEEL_DELTA, MAX_PIXELS_PER_WHEEL_DELTA);
    let blended = match current {
        Some(estimate) if estimate.is_measured() => {
            estimate.pixels_per_delta * (1.0 - SCROLL_RATIO_SMOOTHING)
                + observed * SCROLL_RATIO_SMOOTHING
        }
        _ => observed,
    };

    Some(ScrollRatioEstimate {
        pixels_per_delta: blended.clamp(MIN_PIXELS_PER_WHEEL_DELTA, MAX_PIXELS_PER_WHEEL_DELTA),
        samples: current.map_or(1, |estimate| estimate.samples.saturating_add(1)),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollPace {
    Slow,
    Fast,
}

/// 滚轮节奏统计。慢速滚动逐帧跟随；快速滚动改为先隐藏、静止后再吸附，
/// 避免把操作条渲染在一个还没验证过的位置上。
///
/// 用**衰减滑动窗口**而不是固定翻滚窗口：翻滚窗口到期会把计数清零，
/// 持续快速滚动时节奏会周期性掉回 Slow，操作条随之反复隐藏/显示而闪烁。
/// 衰减窗口下，稳态累计值约等于 `FAST_SCROLL_DECAY_WINDOW_MS / 事件间隔`，
/// 即“一个地平线内滚了几格”，语义不变但不会因为窗口边界产生跳变。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollBurst {
    last_event_at_ms: Option<u64>,
    accumulated_notches: f64,
    is_fast: bool,
}

impl ScrollBurst {
    pub fn register(&mut self, wheel_delta: f64, now_ms: u64) -> ScrollPace {
        let decay = match self.last_event_at_ms {
            Some(last) => {
                let elapsed = now_ms.saturating_sub(last) as f64;
                (1.0 - elapsed / FAST_SCROLL_DECAY_WINDOW_MS).max(0.0)
            }
            None => 0.0,
        };
        self.last_event_at_ms = Some(now_ms);
        self.accumulated_notches =
            self.accumulated_notches * decay + (wheel_delta / WHEEL_DELTA_UNIT).abs();

        // 迟滞：进入和退出用不同阈值，速度停在边界时不会来回抖。
        if self.is_fast {
            if self.accumulated_notches < SLOW_RESUME_NOTCH_THRESHOLD {
                self.is_fast = false;
            }
        } else if self.accumulated_notches >= FAST_SCROLL_NOTCH_THRESHOLD {
            self.is_fast = true;
        }

        if self.is_fast {
            ScrollPace::Fast
        } else {
            ScrollPace::Slow
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn accumulated_notches(&self) -> f64 {
        self.accumulated_notches
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollTrackerAction {
    /// 快速滚动进行中：保持隐藏，不做测量。
    WaitHidden,
    /// 采样一次真实选区位置并跟随。
    Measure,
    /// 已经在静止后完成吸附，结束本次跟随。
    Finish,
    /// 连续测量失败：隐藏操作条而不是停在猜测位置。
    Abandon,
}

/// 这次测量之后，是否可以认为「已经吸附到最终位置」。
///
/// 必须同时满足三个条件，缺一不可：
/// - `settled`：距最后一个滚轮事件已超过 [`SCROLL_SETTLE_IDLE_MS`]；
/// - 位移停止：本次测得的 y 相对上一次几乎没变；
/// - 裁剪停止：本次测得的高度相对上一次几乎没变。
///
/// 只看 `settled` 是不够的——平滑滚动动画可能比静止阈值更长，此时内容仍在
/// 移动，用那一帧的位置收尾会把操作条永久留在动画中途的位置上。
///
/// 只看 y 也不够。多行选区从视口**顶部**滚出时，UIA 的
/// `GetBoundingRectangles` 返回的是裁剪后的矩形：首行被上边缘切掉一截，
/// `y` 就被钉在视口顶边不再变化，而高度还在持续缩小。此时只比 y 会看到
/// 连续两帧「没动」而提前判稳，把操作条定死在一个正在滚走的选区上。
/// 高度仍在变 == 裁剪仍在推进 == 内容其实还在动。
///
/// 两个 delta 都必须来自**未经压缩**的测量几何。`scroll_follow_placement_rect`
/// 会把高度钳到 36px，压缩之后的高度在裁剪过程中大部分时间是常数 36，
/// 这个信号就没了。
pub fn settled_measurement_is_stable(
    settled: bool,
    measured_delta_y: f64,
    measured_delta_height: f64,
) -> bool {
    settled
        && measured_delta_y.abs() < SCROLL_SETTLE_STABLE_EPSILON_PX
        && measured_delta_height.abs() < SCROLL_SETTLE_STABLE_EPSILON_PX
}

/// 新的跟随会话是否应当以**隐藏**状态启动。
///
/// 三种情况都不能在启动时就把操作条摆出来，共同的原则是「不在未经验证的
/// 位置上渲染」：
///
/// - `pace == Fast`：快滚期间刻意不测量，摆出去的只能是预测值；
/// - `recovering_from_abandon`：上一轮已经确认选区找不到了，凭预测摆回去
///   就是在无关内容上凭空生出一个幽灵操作条；
/// - `!has_seed_geometry`：剪贴板兜底选区没有 `selection_rects`、也没有视觉
///   状态，连预测所需的起始 y 都不存在。此时会话仍然启动（这样才有机会由
///   tracker 线程从 UIA 取到第一帧真实几何），但必须先隐藏着等。
pub fn scroll_follow_starts_hidden(
    pace: ScrollPace,
    recovering_from_abandon: bool,
    has_seed_geometry: bool,
) -> bool {
    pace == ScrollPace::Fast || recovering_from_abandon || !has_seed_geometry
}

/// 决定跟随线程这一帧该做什么。
///
/// `settled_measurement_stable` 表示「已经在静止之后测到过一次、且那次测量
/// 相对上一次几乎没有位移」。只有空闲时间够是不够的——平滑滚动动画可能比
/// 静止阈值更长，此时位置还在变，收尾会把中途位置定死。
pub fn scroll_tracker_action(
    pace: ScrollPace,
    idle_ms: u64,
    consecutive_failures: u32,
    settled_measurement_stable: bool,
) -> ScrollTrackerAction {
    if consecutive_failures >= MAX_SCROLL_MEASURE_FAILURES {
        return ScrollTrackerAction::Abandon;
    }

    let settled = idle_ms >= SCROLL_SETTLE_IDLE_MS;
    if settled && settled_measurement_stable {
        return ScrollTrackerAction::Finish;
    }
    if pace == ScrollPace::Fast && !settled {
        return ScrollTrackerAction::WaitHidden;
    }

    ScrollTrackerAction::Measure
}

/// 选区滚出可视区域时应当隐藏，而不是把操作条留在无关内容上。
pub fn selection_still_trackable(selection_rect: Rect, viewport: Rect) -> bool {
    let visible_top = selection_rect.y.max(viewport.y);
    let visible_bottom =
        (selection_rect.y + selection_rect.height).min(viewport.y + viewport.height);
    let visible_height = visible_bottom - visible_top;
    let visible_left = selection_rect.x.max(viewport.x);
    let visible_right = (selection_rect.x + selection_rect.width).min(viewport.x + viewport.width);

    visible_height >= (selection_rect.height * 0.5).min(6.0) && visible_right - visible_left >= 8.0
}

/// 同上，但把来源窗口先裁到真正可见的桌面区域再判断。
///
/// `GetWindowRect` 给的是完整窗口边界，窗口被拖出显示器边缘时，边界会延伸
/// 到桌面之外。只用窗口边界判断的话，一个已经落在桌面外的选区照样算「可
/// 跟踪」，而放置逻辑随后又会把操作条钳回显示器可见边缘——结果是选区看不
/// 见、操作条却停在屏幕边上，稳定后还会提交并收尾，留下一个幽灵操作条。
///
/// 逐块显示器求交而不是用虚拟屏幕的外接矩形：后者会把多屏之间的空隙也算
/// 成可见区域。窗口跨屏时任一块上的可见部分够用即可。
pub fn selection_still_trackable_on_monitors(
    selection_rect: Rect,
    window_rect: Rect,
    monitors: &[Rect],
) -> bool {
    // 拿不到显示器信息时退回只看窗口边界：宁可多跟一帧，也好过误隐藏。
    if monitors.is_empty() {
        return selection_still_trackable(selection_rect, window_rect);
    }

    monitors.iter().any(|monitor| {
        rect_intersection(window_rect, *monitor)
            .is_some_and(|viewport| selection_still_trackable(selection_rect, viewport))
    })
}

fn is_browser_process(source_app: &str) -> bool {
    let process_name = source_app
        .rsplit(|ch| ch == '\\' || ch == '/')
        .next()
        .unwrap_or(source_app)
        .trim()
        .to_ascii_lowercase();

    matches!(
        process_name.as_str(),
        "chrome.exe"
            | "chromium.exe"
            | "msedge.exe"
            | "firefox.exe"
            | "brave.exe"
            | "vivaldi.exe"
            | "opera.exe"
            | "opera_gx.exe"
            | "iexplore.exe"
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisibleFloatingButtonAction {
    NoVisibleButton,
    KeepVisible,
    HideAndRearmSelection { anchor: Point },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PendingSelectionHoverAction {
    NoPendingSelection,
    KeepPending,
    CaptureAndShowButton { anchor: Point },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMouseUpEffect {
    ShowButtonAndClearPending,
    ClearSelectionAndHide,
    PreserveSelection,
}

pub fn classify_mouse_up(
    down: Point,
    up: Point,
    min_drag_distance: f64,
    assistant_windows: &[Rect],
) -> MouseUpAction {
    if is_drag_distance_met(down, up, min_drag_distance) {
        MouseUpAction::ArmSelection {
            anchor: drag_anchor_point(down, up),
            toolbar_anchor: drag_toolbar_anchor_point(down, up),
        }
    } else if assistant_windows
        .iter()
        .any(|window| rect_contains(*window, up))
    {
        MouseUpAction::PreserveSelection
    } else {
        MouseUpAction::ClearSelection
    }
}

pub fn handle_mouse_button_event(
    drag_start: &mut Option<Point>,
    pending_selection: &mut Option<PendingSelection>,
    event: MouseButtonEvent,
    min_drag_distance: f64,
    assistant_windows: &[Rect],
) -> Option<MouseUpAction> {
    match event {
        MouseButtonEvent::Down(point) => {
            *drag_start = Some(point);
            consume_pending_selection(pending_selection);
            None
        }
        MouseButtonEvent::Up(point) => drag_start
            .take()
            .map(|down| classify_mouse_up(down, point, min_drag_distance, assistant_windows)),
        MouseButtonEvent::Move(_) | MouseButtonEvent::Wheel { .. } => None,
    }
}

pub fn consume_pending_selection(pending_selection: &mut Option<PendingSelection>) {
    *pending_selection = None;
}

pub fn apply_mouse_up_action_to_pending_selection(
    pending_selection: &mut Option<PendingSelection>,
    action: MouseUpAction,
) -> SelectionMouseUpEffect {
    match action {
        MouseUpAction::ArmSelection { .. } => {
            *pending_selection = None;
            SelectionMouseUpEffect::ShowButtonAndClearPending
        }
        MouseUpAction::ClearSelection => {
            *pending_selection = None;
            SelectionMouseUpEffect::ClearSelectionAndHide
        }
        MouseUpAction::PreserveSelection => SelectionMouseUpEffect::PreserveSelection,
    }
}

pub fn hover_action_for_pending_selection_when_idle(
    pending_selection: &mut Option<PendingSelection>,
    drag_start: Option<&Point>,
    position: Point,
    hover_radius: f64,
    now_ms: u64,
    hover_delay_ms: u64,
) -> PendingSelectionHoverAction {
    if drag_start.is_some() {
        reset_hover_dwell(pending_selection);
        PendingSelectionHoverAction::KeepPending
    } else {
        hover_action_for_pending_selection(
            pending_selection,
            position,
            hover_radius,
            now_ms,
            hover_delay_ms,
        )
    }
}

pub fn hover_action_for_pending_selection(
    pending_selection: &mut Option<PendingSelection>,
    position: Point,
    hover_radius: f64,
    _now_ms: u64,
    _hover_delay_ms: u64,
) -> PendingSelectionHoverAction {
    let Some(pending) = pending_selection.as_mut() else {
        return PendingSelectionHoverAction::NoPendingSelection;
    };

    if is_drag_distance_met(pending.anchor, position, hover_radius) {
        pending.hover_started_at_ms = None;
        return PendingSelectionHoverAction::KeepPending;
    }

    PendingSelectionHoverAction::CaptureAndShowButton {
        anchor: pending.toolbar_anchor,
    }
}

pub fn visible_floating_button_action_when_idle(
    visible_button: &mut Option<VisibleFloatingButton>,
    drag_start: Option<&Point>,
    position: Point,
    _hover_radius: f64,
    assistant_windows: &[Rect],
) -> VisibleFloatingButtonAction {
    if visible_button.is_none() {
        return VisibleFloatingButtonAction::NoVisibleButton;
    }

    if drag_start.is_some()
        || assistant_windows
            .iter()
            .any(|window| rect_contains(*window, position))
    {
        return VisibleFloatingButtonAction::KeepVisible;
    }

    VisibleFloatingButtonAction::KeepVisible
}

fn reset_hover_dwell(pending_selection: &mut Option<PendingSelection>) {
    if let Some(pending) = pending_selection.as_mut() {
        pending.hover_started_at_ms = None;
    }
}

fn drag_anchor_point(down: Point, up: Point) -> Point {
    Point {
        x: (down.x + up.x) / 2.0,
        y: (down.y + up.y) / 2.0,
    }
}

fn drag_toolbar_anchor_point(down: Point, up: Point) -> Point {
    Point {
        x: down.x.min(up.x),
        y: (down.y.min(up.y) - ESTIMATED_TEXT_TOP_OFFSET).max(0.0),
    }
}

pub fn rect_contains(rect: Rect, point: Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}
