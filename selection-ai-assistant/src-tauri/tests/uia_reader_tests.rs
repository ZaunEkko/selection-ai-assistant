use selection_ai_assistant_lib::selection::uia_reader::{SelectionConfidence, UiaSelectionResult};
use selection_ai_assistant_lib::types::Rect;

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
