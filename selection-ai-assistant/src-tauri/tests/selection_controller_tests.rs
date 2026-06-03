use selection_ai_assistant_lib::selection::controller::{
    is_point_near_anchor, SelectionController, SelectionEvent, SelectionStateKind,
};
use selection_ai_assistant_lib::selection::types::{
    SelectionAnchorSource, SelectionCandidate, SelectionReadMethod,
};
use selection_ai_assistant_lib::types::{Point, Rect};

fn candidate() -> SelectionCandidate {
    SelectionCandidate {
        id: "candidate-1".to_string(),
        text: "hello world".to_string(),
        source_app: "chrome.exe".to_string(),
        window_title: "Example".to_string(),
        anchor_rect: Some(Rect {
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 40.0,
        }),
        fallback_point: Point { x: 120.0, y: 120.0 },
        read_method: SelectionReadMethod::UiAutomation,
        selection_rects: Vec::new(),
        explicit_anchor: None,
        anchor_source: None,
    }
}

#[test]
fn arms_candidate_after_read_success() {
    let mut controller = SelectionController::new();
    controller.handle(SelectionEvent::ReadSuccess(candidate()));
    assert_eq!(controller.state_kind(), SelectionStateKind::SelectionArmed);
}

#[test]
fn shows_button_when_mouse_is_near_anchor() {
    let mut controller = SelectionController::new();
    controller.handle(SelectionEvent::ReadSuccess(candidate()));
    controller.handle(SelectionEvent::MouseHoverSatisfied(Point {
        x: 200.0,
        y: 120.0,
    }));
    assert_eq!(controller.state_kind(), SelectionStateKind::ButtonVisible);
}

#[test]
fn escape_returns_to_idle() {
    let mut controller = SelectionController::new();
    controller.handle(SelectionEvent::ReadSuccess(candidate()));
    controller.handle(SelectionEvent::EscapePressed);
    assert_eq!(controller.state_kind(), SelectionStateKind::Idle);
}

#[test]
fn detects_point_near_anchor() {
    assert!(is_point_near_anchor(
        Point { x: 100.0, y: 100.0 },
        Point { x: 140.0, y: 130.0 },
        90.0,
    ));
    assert!(!is_point_near_anchor(
        Point { x: 100.0, y: 100.0 },
        Point { x: 260.0, y: 260.0 },
        90.0,
    ));
}

#[test]
fn creates_clipboard_candidate_with_generated_id() {
    let candidate = SelectionCandidate::from_clipboard_text(
        "selected text".to_string(),
        "code.exe".to_string(),
        "Editor".to_string(),
        Point { x: 10.0, y: 20.0 },
    );

    assert!(!candidate.id.is_empty());
    assert_eq!(candidate.text, "selected text");
    assert_eq!(candidate.read_method, SelectionReadMethod::Clipboard);
    assert_eq!(candidate.anchor_rect, None);
}

#[test]
fn candidate_anchor_point_prefers_selection_rects_over_fallback_point() {
    let candidate = SelectionCandidate {
        id: "sel-real-anchor".to_string(),
        text: "selected".to_string(),
        source_app: "app".to_string(),
        window_title: "window".to_string(),
        read_method: SelectionReadMethod::UiAutomation,
        anchor_rect: None,
        selection_rects: vec![Rect {
            x: 100.0,
            y: 200.0,
            width: 80.0,
            height: 20.0,
        }],
        explicit_anchor: None,
        anchor_source: Some(SelectionAnchorSource::UiAutomationRects),
        fallback_point: Point { x: 10.0, y: 10.0 },
    };

    assert_eq!(candidate.anchor_point(), Point { x: 140.0, y: 210.0 });
}
