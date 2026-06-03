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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePanelLayout {
    pub panel: Point,
    pub source: Point,
}

pub fn place_source_left_of_panel(
    panel_position: Point,
    panel_size: WindowSize,
    source_size: WindowSize,
    screen: ScreenBounds,
    gap: f64,
) -> SourcePanelLayout {
    let min_x = screen.x;
    let min_y = screen.y;
    let max_panel_x = (screen.x + screen.width - panel_size.width).max(min_x);
    let max_panel_y = (screen.y + screen.height - panel_size.height).max(min_y);
    let max_source_y = (screen.y + screen.height - source_size.height).max(min_y);
    let required_panel_x_for_left_source = screen.x + source_size.width + gap;

    let panel_x = if panel_position.x - source_size.width - gap < screen.x {
        required_panel_x_for_left_source.min(max_panel_x)
    } else {
        panel_position.x.clamp(min_x, max_panel_x)
    };
    let panel_y = panel_position.y.clamp(min_y, max_panel_y);
    let source_x = (panel_x - source_size.width - gap).max(screen.x);
    let source_y = panel_y.clamp(min_y, max_source_y);

    SourcePanelLayout {
        panel: Point {
            x: panel_x,
            y: panel_y,
        },
        source: Point {
            x: source_x,
            y: source_y,
        },
    }
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
