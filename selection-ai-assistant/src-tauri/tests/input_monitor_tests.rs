use selection_ai_assistant_lib::config::AppConfig;
use selection_ai_assistant_lib::input_monitor::events::{
    apply_mouse_up_action_to_pending_selection, classify_mouse_up, consume_pending_selection,
    handle_hotkey_state, handle_mouse_button_event, hover_action_for_pending_selection,
    hover_action_for_pending_selection_when_idle, is_drag_distance_met, HotkeyAction,
    HotkeyKeyState, MouseButtonEvent, MouseUpAction, PendingHotkeyAction, PendingSelection,
    PendingSelectionHoverAction, SelectionMouseUpEffect,
};
use selection_ai_assistant_lib::types::{Point, Rect};

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

#[test]
fn mouse_up_after_drag_distance_arms_selection_at_drag_center_without_showing_button() {
    assert_eq!(
        classify_mouse_up(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            6.0,
            &[],
        ),
        MouseUpAction::ArmSelection {
            anchor: Point { x: 5.0, y: 0.0 },
        }
    );
}

#[test]
fn applying_drag_mouse_up_stores_pending_anchor_and_requests_old_button_hide() {
    let mut pending_selection = None;

    let effect = apply_mouse_up_action_to_pending_selection(
        &mut pending_selection,
        MouseUpAction::ArmSelection {
            anchor: Point { x: 5.0, y: 0.0 },
        },
    );

    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 5.0, y: 0.0 },
            hover_started_at_ms: None,
        })
    );
    assert_eq!(
        effect,
        SelectionMouseUpEffect::PendingAnchorArmedAndClearSelectionAndHide
    );
}

#[test]
fn applying_drag_mouse_up_replaces_old_pending_anchor_while_requesting_old_button_hide() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    let effect = apply_mouse_up_action_to_pending_selection(
        &mut pending_selection,
        MouseUpAction::ArmSelection {
            anchor: Point { x: 5.0, y: 0.0 },
        },
    );

    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 5.0, y: 0.0 },
            hover_started_at_ms: None,
        })
    );
    assert_eq!(
        effect,
        SelectionMouseUpEffect::PendingAnchorArmedAndClearSelectionAndHide
    );
}

#[test]
fn short_mouse_up_outside_assistant_ui_clears_stale_selection() {
    let assistant_windows = [Rect {
        x: 40.0,
        y: 40.0,
        width: 30.0,
        height: 30.0,
    }];

    assert_eq!(
        classify_mouse_up(
            Point { x: 0.0, y: 0.0 },
            Point { x: 3.0, y: 4.0 },
            6.0,
            &assistant_windows,
        ),
        MouseUpAction::ClearSelection
    );
}

#[test]
fn short_mouse_up_on_assistant_ui_preserves_current_selection() {
    let assistant_windows = [
        Rect {
            x: 40.0,
            y: 40.0,
            width: 30.0,
            height: 30.0,
        },
        Rect {
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 120.0,
        },
    ];

    assert_eq!(
        classify_mouse_up(
            Point { x: 110.0, y: 110.0 },
            Point { x: 112.0, y: 113.0 },
            6.0,
            &assistant_windows,
        ),
        MouseUpAction::PreserveSelection
    );
}

#[test]
fn mouse_button_events_report_short_outside_click_for_stale_selection_clear() {
    let assistant_windows = [Rect {
        x: 40.0,
        y: 40.0,
        width: 30.0,
        height: 30.0,
    }];
    let mut down = None;
    let mut pending_selection = None;

    assert_eq!(
        handle_mouse_button_event(
            &mut down,
            &mut pending_selection,
            MouseButtonEvent::Down(Point { x: 0.0, y: 0.0 }),
            6.0,
            &assistant_windows,
        ),
        None
    );
    assert_eq!(down, Some(Point { x: 0.0, y: 0.0 }));

    assert_eq!(
        handle_mouse_button_event(
            &mut down,
            &mut pending_selection,
            MouseButtonEvent::Up(Point { x: 1.0, y: 1.0 }),
            6.0,
            &assistant_windows,
        ),
        Some(MouseUpAction::ClearSelection)
    );
    assert_eq!(down, None);
}

#[test]
fn mouse_down_cancels_old_pending_selection_before_new_drag() {
    let mut down = None;
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    assert_eq!(
        handle_mouse_button_event(
            &mut down,
            &mut pending_selection,
            MouseButtonEvent::Down(Point { x: 20.0, y: 20.0 }),
            6.0,
            &[],
        ),
        None
    );

    assert_eq!(down, Some(Point { x: 20.0, y: 20.0 }));
    assert_eq!(pending_selection, None);
}

#[test]
fn mouse_move_during_active_drag_never_hover_triggers_pending_selection() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: Some(900),
    });
    let drag_start = Some(Point { x: 20.0, y: 20.0 });

    assert_eq!(
        hover_action_for_pending_selection_when_idle(
            &mut pending_selection,
            drag_start.as_ref(),
            Point { x: 105.0, y: 105.0 },
            90.0,
            2_000,
            1_000,
        ),
        PendingSelectionHoverAction::KeepPending
    );
    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: None,
        })
    );
}

#[test]
fn mouse_move_after_drag_release_first_entering_hover_radius_does_not_show_button() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    assert_eq!(
        hover_action_for_pending_selection_when_idle(
            &mut pending_selection,
            None,
            Point { x: 105.0, y: 105.0 },
            90.0,
            2_000,
            1_000,
        ),
        PendingSelectionHoverAction::KeepPending
    );
    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: Some(2_000),
        })
    );
}

#[test]
fn hotkey_pending_selection_can_be_consumed_before_opening_panel() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: None,
        })
    );
    consume_pending_selection(&mut pending_selection);
    assert_eq!(pending_selection, None);
}

#[test]
fn all_hotkey_keys_down_arms_pending_explicit_action_without_capture() {
    let mut pending_hotkey = PendingHotkeyAction::default();

    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: true,
                alt: true,
                a: true,
            },
        ),
        HotkeyAction::Armed
    );
    assert!(pending_hotkey.is_armed());
}

#[test]
fn releasing_a_while_ctrl_alt_remain_down_does_not_capture_or_disarm() {
    let mut pending_hotkey = PendingHotkeyAction::default();

    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: true,
                alt: true,
                a: true,
            },
        ),
        HotkeyAction::Armed
    );
    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: true,
                alt: true,
                a: false,
            },
        ),
        HotkeyAction::AlreadyArmed
    );
    assert!(pending_hotkey.is_armed());
}

#[test]
fn ctrl_alt_released_while_a_remains_down_keeps_hotkey_armed_until_full_release() {
    let mut pending_hotkey = PendingHotkeyAction::default();

    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: true,
                alt: true,
                a: true,
            },
        ),
        HotkeyAction::Armed
    );
    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: false,
                alt: false,
                a: true,
            },
        ),
        HotkeyAction::AlreadyArmed
    );
    assert!(pending_hotkey.is_armed());
    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: false,
                alt: false,
                a: false,
            },
        ),
        HotkeyAction::CaptureAndOpen
    );
    assert!(!pending_hotkey.is_armed());
}

#[test]
fn all_hotkey_keys_released_captures_once_after_pending_explicit_action() {
    let mut pending_hotkey = PendingHotkeyAction::default();

    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: true,
                alt: true,
                a: true,
            },
        ),
        HotkeyAction::Armed
    );
    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: false,
                alt: false,
                a: false,
            },
        ),
        HotkeyAction::CaptureAndOpen
    );
    assert!(!pending_hotkey.is_armed());
}

#[test]
fn repeated_hotkey_release_does_not_capture_twice() {
    let mut pending_hotkey = PendingHotkeyAction::default();

    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: true,
                alt: true,
                a: true,
            },
        ),
        HotkeyAction::Armed
    );
    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: false,
                alt: false,
                a: false,
            },
        ),
        HotkeyAction::CaptureAndOpen
    );
    assert_eq!(
        handle_hotkey_state(
            &mut pending_hotkey,
            HotkeyKeyState {
                ctrl: false,
                alt: false,
                a: false,
            },
        ),
        HotkeyAction::Idle
    );
}

#[test]
fn pending_selection_waits_until_explicit_mouse_move_near_anchor() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    assert_eq!(
        hover_action_for_pending_selection(
            &mut pending_selection,
            Point { x: 250.0, y: 100.0 },
            90.0,
            1_000,
            1_000,
        ),
        PendingSelectionHoverAction::KeepPending
    );
    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: None,
        })
    );
}

#[test]
fn pending_selection_inside_hover_radius_before_delay_keeps_pending() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: Some(1_000),
    });

    assert_eq!(
        hover_action_for_pending_selection(
            &mut pending_selection,
            Point { x: 130.0, y: 130.0 },
            90.0,
            1_999,
            1_000,
        ),
        PendingSelectionHoverAction::KeepPending
    );
}

#[test]
fn pending_selection_inside_hover_radius_after_delay_shows_button() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: Some(1_000),
    });

    assert_eq!(
        hover_action_for_pending_selection(
            &mut pending_selection,
            Point { x: 130.0, y: 130.0 },
            90.0,
            2_000,
            1_000,
        ),
        PendingSelectionHoverAction::CaptureAndShowButton {
            anchor: Point { x: 100.0, y: 100.0 },
        }
    );
}

#[test]
fn pending_selection_leaving_hover_radius_resets_dwell() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: Some(1_000),
    });

    assert_eq!(
        hover_action_for_pending_selection(
            &mut pending_selection,
            Point { x: 250.0, y: 100.0 },
            90.0,
            2_000,
            1_000,
        ),
        PendingSelectionHoverAction::KeepPending
    );
    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: None,
        })
    );

    assert_eq!(
        hover_action_for_pending_selection(
            &mut pending_selection,
            Point { x: 130.0, y: 130.0 },
            90.0,
            2_001,
            1_000,
        ),
        PendingSelectionHoverAction::KeepPending
    );
}

#[test]
fn no_pending_selection_means_mouse_move_does_not_show_button() {
    let mut pending_selection = None;

    assert_eq!(
        hover_action_for_pending_selection(
            &mut pending_selection,
            Point { x: 100.0, y: 100.0 },
            90.0,
            1_000,
            1_000,
        ),
        PendingSelectionHoverAction::NoPendingSelection
    );
}

#[test]
fn default_hover_delay_is_one_second() {
    assert_eq!(AppConfig::default().hover_delay_ms, 1_000);
}
