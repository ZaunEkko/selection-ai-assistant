use crate::types::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn place_near_anchor(anchor: Point, size: WindowSize, screen: ScreenBounds) -> Point {
    let margin = 12.0;
    let preferred_x = anchor.x + margin;
    let below_y = anchor.y + margin;
    let above_y = anchor.y - size.height - margin;

    let min_x = screen.x;
    let min_y = screen.y;
    let max_x = (screen.x + screen.width - size.width).max(min_x);
    let max_y = (screen.y + screen.height - size.height).max(min_y);

    let screen_bottom = screen.y + screen.height;
    let has_room_below = below_y + size.height <= screen_bottom;
    let has_room_above = above_y >= screen.y;
    let preferred_y = if !has_room_below && has_room_above {
        above_y
    } else {
        below_y
    };

    Point {
        x: preferred_x.clamp(min_x, max_x),
        y: preferred_y.clamp(min_y, max_y),
    }
}
