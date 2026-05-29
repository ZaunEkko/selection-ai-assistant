use selection_ai_assistant_lib::input_monitor::events::is_drag_distance_met;
use selection_ai_assistant_lib::types::Point;

#[test]
fn detects_drag_distance() {
    assert!(is_drag_distance_met(
        Point { x: 0.0, y: 0.0 },
        Point { x: 10.0, y: 0.0 },
        6.0,
    ));
    assert!(!is_drag_distance_met(
        Point { x: 0.0, y: 0.0 },
        Point { x: 3.0, y: 4.0 },
        6.0,
    ));
}
