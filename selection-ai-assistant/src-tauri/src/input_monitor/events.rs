use serde::{Deserialize, Serialize};

use crate::types::{Point, Rect};

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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseUpAction {
    ArmSelection { anchor: Point },
    ClearSelection,
    PreserveSelection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingSelection {
    pub anchor: Point,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PendingSelectionHoverAction {
    NoPendingSelection,
    KeepPending,
    CaptureAndShowButton { anchor: Point },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMouseUpEffect {
    PendingAnchorArmedAndClearSelectionAndHide,
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
        MouseButtonEvent::Move(_) => None,
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
        MouseUpAction::ArmSelection { anchor } => {
            *pending_selection = Some(PendingSelection { anchor });
            SelectionMouseUpEffect::PendingAnchorArmedAndClearSelectionAndHide
        }
        MouseUpAction::ClearSelection => {
            *pending_selection = None;
            SelectionMouseUpEffect::ClearSelectionAndHide
        }
        MouseUpAction::PreserveSelection => SelectionMouseUpEffect::PreserveSelection,
    }
}

pub fn hover_action_for_pending_selection_when_idle(
    pending: Option<&PendingSelection>,
    drag_start: Option<&Point>,
    position: Point,
    hover_radius: f64,
) -> PendingSelectionHoverAction {
    if drag_start.is_some() {
        PendingSelectionHoverAction::KeepPending
    } else {
        hover_action_for_pending_selection(pending, position, hover_radius)
    }
}

pub fn hover_action_for_pending_selection(
    pending: Option<&PendingSelection>,
    position: Point,
    hover_radius: f64,
) -> PendingSelectionHoverAction {
    let Some(pending) = pending else {
        return PendingSelectionHoverAction::NoPendingSelection;
    };

    if is_drag_distance_met(pending.anchor, position, hover_radius) {
        PendingSelectionHoverAction::KeepPending
    } else {
        PendingSelectionHoverAction::CaptureAndShowButton {
            anchor: pending.anchor,
        }
    }
}

fn drag_anchor_point(down: Point, up: Point) -> Point {
    Point {
        x: (down.x + up.x) / 2.0,
        y: (down.y + up.y) / 2.0,
    }
}

fn rect_contains(rect: Rect, point: Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}
