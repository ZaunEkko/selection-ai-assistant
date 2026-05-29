use serde::{Deserialize, Serialize};

use crate::types::Point;

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
