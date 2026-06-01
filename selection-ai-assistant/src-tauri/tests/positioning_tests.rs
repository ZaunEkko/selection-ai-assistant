use selection_ai_assistant_lib::floating_window::positioning::{
    place_near_anchor, ScreenBounds, WindowSize,
};
use selection_ai_assistant_lib::types::Point;

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
