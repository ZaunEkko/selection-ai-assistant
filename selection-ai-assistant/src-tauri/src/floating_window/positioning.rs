use crate::types::{Point, Rect};

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

pub fn place_translate_result_near_anchor(
    anchor: Point,
    size: WindowSize,
    screen: ScreenBounds,
) -> Point {
    let gap = 12.0;
    let min_x = screen.x;
    let min_y = screen.y;
    let max_x = (screen.x + screen.width - size.width).max(min_x);
    let max_y = (screen.y + screen.height - size.height).max(min_y);

    let above = Point {
        x: anchor.x,
        y: anchor.y - size.height - gap,
    };
    if above.y >= min_y {
        return Point {
            x: above.x.clamp(min_x, max_x),
            y: above.y.clamp(min_y, max_y),
        };
    }

    let right = Point {
        x: anchor.x + gap,
        y: anchor.y,
    };
    if right.x + size.width <= screen.x + screen.width {
        return Point {
            x: right.x.clamp(min_x, max_x),
            y: right.y.clamp(min_y, max_y),
        };
    }

    Point {
        x: (anchor.x - size.width - gap).clamp(min_x, max_x),
        y: anchor.y.clamp(min_y, max_y),
    }
}

pub fn place_toolbar_near_selection(
    anchor: Point,
    selection_rects: &[Rect],
    size: WindowSize,
    screen: ScreenBounds,
) -> Point {
    let gap = 12.0;
    let selection = first_selection_rect(anchor, selection_rects);
    let candidates = [
        Point {
            x: selection.x,
            y: selection.y - size.height - gap,
        },
        Point {
            x: selection.x,
            y: selection.y + selection.height + gap,
        },
        Point {
            x: selection.x + selection.width + gap,
            y: selection.y - 4.0,
        },
        Point {
            x: selection.x - size.width - gap,
            y: selection.y - 4.0,
        },
    ];

    first_non_overlapping_position(&candidates, selection, size, screen)
        .unwrap_or_else(|| clamp_to_screen(candidates[0], size, screen))
}

pub fn place_translate_result_near_selection(
    anchor: Point,
    selection_rects: &[Rect],
    size: WindowSize,
    screen: ScreenBounds,
) -> Point {
    let gap = 20.0;
    let selection = placement_selection_rect(anchor, selection_rects);
    let side_y = selection.y + selection.height / 2.0 - size.height / 2.0;
    let candidates = [
        Point {
            x: selection.x + selection.width + gap,
            y: side_y,
        },
        Point {
            x: selection.x - size.width - gap,
            y: side_y,
        },
        Point {
            x: selection.x,
            y: selection.y - size.height - gap,
        },
        Point {
            x: selection.x,
            y: selection.y + selection.height + gap,
        },
    ];

    first_non_overlapping_position(&candidates, selection, size, screen)
        .unwrap_or_else(|| place_translate_result_near_anchor(anchor, size, screen))
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

pub fn place_toolbar_above_anchor(anchor: Point, size: WindowSize, screen: ScreenBounds) -> Point {
    let gap = 8.0;
    let min_x = screen.x;
    let min_y = screen.y;
    let max_x = (screen.x + screen.width - size.width).max(min_x);
    let max_y = (screen.y + screen.height - size.height).max(min_y);

    let preferred_x = anchor.x;
    let above_y = anchor.y - size.height - gap;

    Point {
        x: preferred_x.clamp(min_x, max_x),
        y: above_y.clamp(min_y, max_y),
    }
}

fn first_non_overlapping_position(
    candidates: &[Point],
    selection: Rect,
    size: WindowSize,
    screen: ScreenBounds,
) -> Option<Point> {
    let avoid = expand_rect(selection, 3.0);

    candidates.iter().copied().find_map(|candidate| {
        let position = clamp_to_screen(candidate, size, screen);
        let window = window_rect(position, size);
        (!rects_intersect(window, avoid)).then_some(position)
    })
}

fn first_selection_rect(anchor: Point, selection_rects: &[Rect]) -> Rect {
    let Some(first) = selection_rects.iter().copied().find(is_valid_rect) else {
        return Rect {
            x: anchor.x,
            y: anchor.y,
            width: 1.0,
            height: 1.0,
        };
    };

    if looks_like_text_space(first, anchor) {
        Rect {
            x: anchor.x,
            y: anchor.y,
            width: 1.0,
            height: 1.0,
        }
    } else {
        first
    }
}

fn placement_selection_rect(anchor: Point, selection_rects: &[Rect]) -> Rect {
    let valid_rects = selection_rects
        .iter()
        .copied()
        .filter(is_valid_rect)
        .collect::<Vec<_>>();

    if valid_rects.is_empty() {
        return Rect {
            x: anchor.x,
            y: anchor.y,
            width: 1.0,
            height: 1.0,
        };
    }

    if valid_rects.len() == 1 && looks_like_text_space(valid_rects[0], anchor) {
        return estimated_line_rect(anchor, valid_rects[0]);
    }

    union_rects(&valid_rects).unwrap_or(Rect {
        x: anchor.x,
        y: anchor.y,
        width: 1.0,
        height: 1.0,
    })
}

fn looks_like_text_space(rect: Rect, anchor: Point) -> bool {
    rect.height >= 48.0 && rect.width >= 240.0 && rect_contains_point(rect, anchor)
}

fn estimated_line_rect(anchor: Point, text_space: Rect) -> Rect {
    let line_height = 24.0_f64.min(text_space.height.max(1.0));
    let max_y = (text_space.y + text_space.height - line_height).max(text_space.y);
    let max_x = (text_space.x + text_space.width - 1.0).max(text_space.x);
    let x = anchor.x.clamp(text_space.x, max_x);
    let y = (anchor.y - line_height / 2.0).clamp(text_space.y, max_y);
    let available_width = (text_space.x + text_space.width - x).max(1.0);

    Rect {
        x,
        y,
        width: available_width.min(320.0).max(1.0),
        height: line_height,
    }
}

fn union_rects(rects: &[Rect]) -> Option<Rect> {
    let mut valid = rects.iter().copied().filter(is_valid_rect);
    let first = valid.next()?;
    let mut left = first.x;
    let mut top = first.y;
    let mut right = first.x + first.width;
    let mut bottom = first.y + first.height;

    for rect in valid {
        left = left.min(rect.x);
        top = top.min(rect.y);
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    }

    Some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn clamp_to_screen(position: Point, size: WindowSize, screen: ScreenBounds) -> Point {
    let min_x = screen.x;
    let min_y = screen.y;
    let max_x = (screen.x + screen.width - size.width).max(min_x);
    let max_y = (screen.y + screen.height - size.height).max(min_y);

    Point {
        x: position.x.clamp(min_x, max_x),
        y: position.y.clamp(min_y, max_y),
    }
}

fn window_rect(position: Point, size: WindowSize) -> Rect {
    Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
}

fn expand_rect(rect: Rect, amount: f64) -> Rect {
    Rect {
        x: rect.x - amount,
        y: rect.y - amount,
        width: rect.width + amount * 2.0,
        height: rect.height + amount * 2.0,
    }
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

fn rect_contains_point(rect: Rect, point: Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

fn is_valid_rect(rect: &Rect) -> bool {
    rect.width > 0.0 && rect.height > 0.0
}
