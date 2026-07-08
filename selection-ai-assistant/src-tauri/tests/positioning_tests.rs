use selection_ai_assistant_lib::floating_window::positioning::{
    place_near_anchor, place_source_left_of_panel, place_toolbar_above_anchor,
    place_toolbar_near_selection, place_translate_result_near_anchor,
    place_translate_result_near_selection, ScreenBounds, WindowSize,
};
use selection_ai_assistant_lib::types::{Point, Rect};

#[test]
fn places_toolbar_above_anchor_aligned_to_selection_start_when_top_space_allows() {
    let position = place_toolbar_above_anchor(
        Point { x: 500.0, y: 300.0 },
        WindowSize {
            width: 300.0,
            height: 52.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 500.0, y: 240.0 });
}

#[test]
fn clamps_toolbar_to_screen_top_when_top_space_is_tight() {
    let position = place_toolbar_above_anchor(
        Point { x: 120.0, y: 58.0 },
        WindowSize {
            width: 300.0,
            height: 52.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 120.0, y: 0.0 });
}

#[test]
fn places_window_near_anchor_inside_screen() {
    let position = place_near_anchor(
        Point { x: 500.0, y: 400.0 },
        WindowSize {
            width: 320.0,
            height: 240.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert!(position.x >= 0.0);
    assert!(position.y >= 0.0);
    assert!(position.x + 320.0 <= 1920.0);
    assert!(position.y + 240.0 <= 1080.0);
}

#[test]
fn clamps_window_at_right_edge() {
    let position = place_near_anchor(
        Point {
            x: 1900.0,
            y: 900.0,
        },
        WindowSize {
            width: 320.0,
            height: 240.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position.x, 1600.0);
}

#[test]
fn flips_panel_above_anchor_when_bottom_space_is_insufficient() {
    let position = place_near_anchor(
        Point {
            x: 500.0,
            y: 1040.0,
        },
        WindowSize {
            width: 420.0,
            height: 360.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position.y, 668.0);
}

#[test]
fn places_translate_result_above_anchor_when_space_allows() {
    let position = place_translate_result_near_anchor(
        Point { x: 500.0, y: 500.0 },
        WindowSize {
            width: 360.0,
            height: 220.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 500.0, y: 268.0 });
}

#[test]
fn places_translate_result_to_right_when_top_space_is_tight() {
    let position = place_translate_result_near_anchor(
        Point { x: 500.0, y: 70.0 },
        WindowSize {
            width: 360.0,
            height: 220.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 512.0, y: 70.0 });
}

#[test]
fn places_translate_result_to_left_when_top_and_right_space_are_tight() {
    let position = place_translate_result_near_anchor(
        Point { x: 1810.0, y: 70.0 },
        WindowSize {
            width: 360.0,
            height: 220.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 1438.0, y: 70.0 });
}

#[test]
fn places_toolbar_above_first_selection_rect_even_when_side_space_allows() {
    let position = place_toolbar_near_selection(
        Point { x: 420.0, y: 320.0 },
        &[Rect {
            x: 420.0,
            y: 320.0,
            width: 180.0,
            height: 24.0,
        }],
        WindowSize {
            width: 244.0,
            height: 44.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 420.0, y: 264.0 });
}

#[test]
fn places_toolbar_below_selection_when_selection_is_near_screen_top() {
    let position = place_toolbar_near_selection(
        Point { x: 120.0, y: 8.0 },
        &[Rect {
            x: 120.0,
            y: 8.0,
            width: 180.0,
            height: 22.0,
        }],
        WindowSize {
            width: 244.0,
            height: 44.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 120.0, y: 42.0 });
}

#[test]
fn places_translate_result_to_right_of_selection_before_trying_above() {
    let position = place_translate_result_near_selection(
        Point { x: 420.0, y: 320.0 },
        &[Rect {
            x: 420.0,
            y: 320.0,
            width: 180.0,
            height: 24.0,
        }],
        WindowSize {
            width: 320.0,
            height: 180.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 620.0, y: 242.0 });
}

#[test]
fn places_translate_result_to_left_when_right_side_would_cover_screen_edge() {
    let position = place_translate_result_near_selection(
        Point { x: 1680.0, y: 90.0 },
        &[Rect {
            x: 1680.0,
            y: 90.0,
            width: 210.0,
            height: 24.0,
        }],
        WindowSize {
            width: 320.0,
            height: 180.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    assert_eq!(position, Point { x: 1340.0, y: 12.0 });
}

#[test]
fn shifts_panel_right_when_source_window_needs_left_side_space() {
    let layout = place_source_left_of_panel(
        Point { x: 20.0, y: 100.0 },
        WindowSize {
            width: 520.0,
            height: 620.0,
        },
        WindowSize {
            width: 360.0,
            height: 620.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
        12.0,
    );

    assert_eq!(layout.source.x, 0.0);
    assert_eq!(layout.panel.x, 372.0);
    assert_eq!(layout.source.x + 360.0 + 12.0, layout.panel.x);
}

#[test]
fn keeps_panel_position_when_left_side_has_room_for_source_window() {
    let layout = place_source_left_of_panel(
        Point { x: 600.0, y: 100.0 },
        WindowSize {
            width: 520.0,
            height: 620.0,
        },
        WindowSize {
            width: 360.0,
            height: 620.0,
        },
        ScreenBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
        12.0,
    );

    assert_eq!(layout.panel.x, 600.0);
    assert_eq!(layout.source.x, 228.0);
}

#[test]
fn clamps_window_at_left_edge_and_handles_oversized_window() {
    let position = place_near_anchor(
        Point { x: -50.0, y: -20.0 },
        WindowSize {
            width: 2200.0,
            height: 1200.0,
        },
        ScreenBounds {
            x: 100.0,
            y: 50.0,
            width: 800.0,
            height: 600.0,
        },
    );

    assert_eq!(position, Point { x: 100.0, y: 50.0 });
}
