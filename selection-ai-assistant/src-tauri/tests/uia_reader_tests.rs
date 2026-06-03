use selection_ai_assistant_lib::selection::uia_reader::{SelectionConfidence, UiaSelectionResult};
use selection_ai_assistant_lib::types::Rect;

fn result(text: &str, x: f64) -> UiaSelectionResult {
    UiaSelectionResult {
        text: Some(text.to_string()),
        rects: vec![Rect {
            x,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }],
        is_password_control: false,
        confidence: SelectionConfidence::High,
    }
}

#[test]
fn focused_element_selection_is_preferred_over_window_selection() {
    let focused = result("focused child", 10.0);
    let window = result("top level", 100.0);

    let selected =
        UiaSelectionResult::prefer_focused_attempt(Some(focused), Some(window)).expect("selection");

    assert_eq!(selected.text.as_deref(), Some("focused child"));
    assert_eq!(selected.rects[0].x, 10.0);
}

#[test]
fn window_geometry_is_used_when_focused_selection_has_no_valid_rects() {
    let focused = UiaSelectionResult {
        text: Some("focused child".to_string()),
        rects: Vec::new(),
        is_password_control: false,
        confidence: SelectionConfidence::High,
    };
    let window = result("top level", 100.0);

    let selected =
        UiaSelectionResult::prefer_focused_attempt(Some(focused), Some(window)).expect("selection");

    assert_eq!(selected.text.as_deref(), Some("top level"));
    assert_eq!(selected.rects[0].x, 100.0);
}

#[test]
fn focused_password_control_blocks_window_geometry_and_drag_fallback() {
    let focused_password = UiaSelectionResult {
        text: None,
        rects: Vec::new(),
        is_password_control: true,
        confidence: SelectionConfidence::Low,
    };
    let window = result("top level", 100.0);

    let selected = UiaSelectionResult::prefer_focused_attempt(Some(focused_password), Some(window))
        .expect("password result");

    assert!(selected.is_password_control);
    assert_eq!(selected.selection_anchor_point(), None);
}

#[test]
fn window_selection_is_used_when_focused_element_has_no_selection() {
    let window = result("top level", 100.0);

    let selected =
        UiaSelectionResult::prefer_focused_attempt(None, Some(window)).expect("selection");

    assert_eq!(selected.text.as_deref(), Some("top level"));
    assert_eq!(selected.rects[0].x, 100.0);
}

#[test]
fn focused_password_control_blocks_window_fallback() {
    let focused_password = UiaSelectionResult {
        text: None,
        rects: Vec::new(),
        is_password_control: true,
        confidence: SelectionConfidence::Low,
    };
    let window = result("top level", 100.0);

    let selected = UiaSelectionResult::prefer_focused_attempt(Some(focused_password), Some(window))
        .expect("password result");

    assert!(selected.is_password_control);
    assert_eq!(selected.text, None);
    assert!(selected.rects.is_empty());
}

#[test]
fn password_control_result_is_not_usable() {
    let result = UiaSelectionResult {
        text: Some("secret".to_string()),
        rects: vec![Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        }],
        is_password_control: true,
        confidence: SelectionConfidence::High,
    };

    assert!(!result.is_usable());
}

#[test]
fn empty_text_result_is_not_usable() {
    let result = UiaSelectionResult {
        text: Some("  ".to_string()),
        rects: Vec::new(),
        is_password_control: false,
        confidence: SelectionConfidence::High,
    };

    assert!(!result.is_usable());
}

#[test]
fn non_password_text_result_is_usable() {
    let result = UiaSelectionResult {
        text: Some("hello".to_string()),
        rects: Vec::new(),
        is_password_control: false,
        confidence: SelectionConfidence::Medium,
    };

    assert!(result.is_usable());
}

#[test]
fn primary_rect_returns_first_rect() {
    let rect = Rect {
        x: 1.0,
        y: 2.0,
        width: 3.0,
        height: 4.0,
    };
    let result = UiaSelectionResult {
        text: Some("hello".to_string()),
        rects: vec![rect],
        is_password_control: false,
        confidence: SelectionConfidence::Medium,
    };

    assert_eq!(result.primary_rect(), Some(rect));
}

#[test]
fn multi_line_selection_anchor_uses_weighted_rect_center() {
    let result = UiaSelectionResult {
        text: Some("第一行\n第二行".to_string()),
        rects: vec![
            Rect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 20.0,
            },
            Rect {
                x: 10.0,
                y: 40.0,
                width: 200.0,
                height: 20.0,
            },
        ],
        is_password_control: false,
        confidence: SelectionConfidence::High,
    };

    let anchor = result.selection_anchor_point().expect("anchor");

    assert!((anchor.x - 93.333).abs() < 0.01);
    assert!((anchor.y - 35.0).abs() < 0.01);
}

#[test]
fn empty_or_zero_sized_rects_return_none_anchor() {
    let result = UiaSelectionResult {
        text: Some("selected".to_string()),
        rects: vec![
            Rect {
                x: 10.0,
                y: 10.0,
                width: 0.0,
                height: 20.0,
            },
            Rect {
                x: 10.0,
                y: 40.0,
                width: 100.0,
                height: 0.0,
            },
        ],
        is_password_control: false,
        confidence: SelectionConfidence::Low,
    };

    assert_eq!(result.selection_anchor_point(), None);
}
