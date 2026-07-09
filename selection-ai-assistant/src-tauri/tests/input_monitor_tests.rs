use selection_ai_assistant_lib::config::AppConfig;
use selection_ai_assistant_lib::input_monitor::events::{
    apply_mouse_up_action_to_pending_selection, classify_mouse_up, consume_pending_selection,
    handle_hotkey_state, handle_mouse_button_event, hover_action_for_pending_selection,
    hover_action_for_pending_selection_when_idle, is_drag_distance_met, manual_hotkey_trigger_key,
    selection_geometry_matches_drag_gesture, selection_rects_match_drag_gesture,
    should_follow_scroll_for_source, visible_floating_button_action_when_idle, HotkeyAction,
    HotkeyKeyState, MouseButtonEvent, MouseUpAction, PendingHotkeyAction, PendingSelection,
    PendingSelectionHoverAction, SelectionMouseUpEffect, VisibleFloatingButton,
    VisibleFloatingButtonAction,
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
fn mouse_up_after_drag_distance_arms_selection_with_hover_center_and_toolbar_start() {
    assert_eq!(
        classify_mouse_up(
            Point { x: 40.0, y: 80.0 },
            Point { x: 400.0, y: 120.0 },
            6.0,
            &[],
        ),
        MouseUpAction::ArmSelection {
            anchor: Point { x: 220.0, y: 100.0 },
            toolbar_anchor: Point { x: 40.0, y: 68.0 },
        }
    );
}

#[test]
fn mouse_up_after_drag_distance_arms_selection_at_drag_start_for_immediate_show() {
    assert_eq!(
        classify_mouse_up(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            6.0,
            &[],
        ),
        MouseUpAction::ArmSelection {
            anchor: Point { x: 5.0, y: 0.0 },
            toolbar_anchor: Point { x: 0.0, y: 0.0 },
        }
    );
}

#[test]
fn applying_drag_mouse_up_clears_pending_anchor_and_requests_immediate_show() {
    let mut pending_selection = None;

    let effect = apply_mouse_up_action_to_pending_selection(
        &mut pending_selection,
        MouseUpAction::ArmSelection {
            anchor: Point { x: 5.0, y: 0.0 },
            toolbar_anchor: Point { x: 5.0, y: 0.0 },
        },
    );

    assert_eq!(pending_selection, None);
    assert_eq!(effect, SelectionMouseUpEffect::ShowButtonAndClearPending);
}

#[test]
fn applying_drag_mouse_up_clears_old_pending_anchor_for_immediate_show() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    let effect = apply_mouse_up_action_to_pending_selection(
        &mut pending_selection,
        MouseUpAction::ArmSelection {
            anchor: Point { x: 5.0, y: 0.0 },
            toolbar_anchor: Point { x: 5.0, y: 0.0 },
        },
    );

    assert_eq!(pending_selection, None);
    assert_eq!(effect, SelectionMouseUpEffect::ShowButtonAndClearPending);
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
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
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
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
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
            toolbar_anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: None,
        })
    );
}

#[test]
fn mouse_move_after_drag_release_first_entering_hover_radius_shows_button() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
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
        PendingSelectionHoverAction::CaptureAndShowButton {
            anchor: Point { x: 100.0, y: 100.0 },
        }
    );
}

#[test]
fn hotkey_pending_selection_can_be_consumed_before_opening_panel() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    assert_eq!(
        pending_selection,
        Some(PendingSelection {
            anchor: Point { x: 100.0, y: 100.0 },
            toolbar_anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: None,
        })
    );
    consume_pending_selection(&mut pending_selection);
    assert_eq!(pending_selection, None);
}

#[test]
fn parses_manual_ctrl_alt_letter_hotkey_from_config() {
    assert_eq!(manual_hotkey_trigger_key("Ctrl+Alt+T"), Some('T'));
    assert_eq!(manual_hotkey_trigger_key("ctrl + alt + k"), Some('K'));
    assert_eq!(manual_hotkey_trigger_key("Ctrl+Shift+T"), None);
    assert_eq!(manual_hotkey_trigger_key("Ctrl+Alt+Enter"), None);
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
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
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
            toolbar_anchor: Point { x: 100.0, y: 100.0 },
            hover_started_at_ms: None,
        })
    );
}

#[test]
fn pending_selection_inside_hover_radius_shows_button_on_first_move() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    assert_eq!(
        hover_action_for_pending_selection(
            &mut pending_selection,
            Point { x: 130.0, y: 130.0 },
            90.0,
            1_000,
            1_000,
        ),
        PendingSelectionHoverAction::CaptureAndShowButton {
            anchor: Point { x: 100.0, y: 100.0 },
        }
    );
}

#[test]
fn pending_selection_inside_hover_radius_still_shows_button_after_delay() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
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
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
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
            toolbar_anchor: Point { x: 100.0, y: 100.0 },
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
        PendingSelectionHoverAction::CaptureAndShowButton {
            anchor: Point { x: 100.0, y: 100.0 },
        }
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
fn visible_floating_button_stays_visible_after_mouse_leaves_hover_radius() {
    let mut visible_button = Some(VisibleFloatingButton {
        window_position: Point { x: 100.0, y: 100.0 },
        selection_anchor: Point { x: 100.0, y: 100.0 },
        selection_rect: None,
        scroll_follow_enabled: true,
    });

    assert_eq!(
        visible_floating_button_action_when_idle(
            &mut visible_button,
            None,
            Point { x: 250.0, y: 100.0 },
            90.0,
            &[],
        ),
        VisibleFloatingButtonAction::KeepVisible
    );
    assert_eq!(
        visible_button,
        Some(VisibleFloatingButton {
            window_position: Point { x: 100.0, y: 100.0 },
            selection_anchor: Point { x: 100.0, y: 100.0 },
            selection_rect: None,
            scroll_follow_enabled: true,
        })
    );
}

#[test]
fn hidden_floating_button_can_show_again_after_mouse_returns_to_hover_radius() {
    let mut pending_selection = Some(PendingSelection {
        anchor: Point { x: 100.0, y: 100.0 },
        toolbar_anchor: Point { x: 100.0, y: 100.0 },
        hover_started_at_ms: None,
    });

    assert_eq!(
        hover_action_for_pending_selection_when_idle(
            &mut pending_selection,
            None,
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
fn visible_floating_button_stays_visible_when_mouse_is_on_assistant_ui() {
    let mut visible_button = Some(VisibleFloatingButton {
        window_position: Point { x: 100.0, y: 100.0 },
        selection_anchor: Point { x: 100.0, y: 100.0 },
        selection_rect: None,
        scroll_follow_enabled: true,
    });
    let assistant_windows = [Rect {
        x: 112.0,
        y: 112.0,
        width: 40.0,
        height: 40.0,
    }];

    assert_eq!(
        visible_floating_button_action_when_idle(
            &mut visible_button,
            None,
            Point { x: 130.0, y: 130.0 },
            20.0,
            &assistant_windows,
        ),
        VisibleFloatingButtonAction::KeepVisible
    );
    assert_eq!(
        visible_button,
        Some(VisibleFloatingButton {
            window_position: Point { x: 100.0, y: 100.0 },
            selection_anchor: Point { x: 100.0, y: 100.0 },
            selection_rect: None,
            scroll_follow_enabled: true,
        })
    );
}

#[test]
fn selection_rects_match_current_drag_gesture() {
    let rects = [Rect {
        x: 90.0,
        y: 88.0,
        width: 180.0,
        height: 24.0,
    }];

    assert!(selection_rects_match_drag_gesture(
        &rects,
        Point { x: 100.0, y: 100.0 },
        Point { x: 260.0, y: 103.0 },
    ));
}

#[test]
fn selection_rects_reject_stale_selection_far_from_current_drag() {
    let stale_rects = [Rect {
        x: 520.0,
        y: 420.0,
        width: 160.0,
        height: 24.0,
    }];

    assert!(!selection_rects_match_drag_gesture(
        &stale_rects,
        Point { x: 100.0, y: 100.0 },
        Point { x: 170.0, y: 104.0 },
    ));
}

#[test]
fn selection_rects_reject_large_control_bounds_for_click_drag() {
    let control_bounds = [Rect {
        x: 40.0,
        y: 40.0,
        width: 760.0,
        height: 420.0,
    }];

    assert!(!selection_rects_match_drag_gesture(
        &control_bounds,
        Point { x: 100.0, y: 100.0 },
        Point { x: 150.0, y: 105.0 },
    ));
}

#[test]
fn selection_geometry_accepts_visual_selection_when_browser_uia_has_no_rects() {
    assert!(selection_geometry_matches_drag_gesture(
        &[],
        Point { x: 100.0, y: 100.0 },
        Point { x: 260.0, y: 103.0 },
        true,
    ));
}

#[test]
fn selection_geometry_rejects_empty_uia_rects_without_visual_selection() {
    assert!(!selection_geometry_matches_drag_gesture(
        &[],
        Point { x: 100.0, y: 100.0 },
        Point { x: 260.0, y: 103.0 },
        false,
    ));
}

#[test]
fn selection_geometry_rejects_stale_uia_rects_without_visual_selection() {
    let stale_rects = [Rect {
        x: 520.0,
        y: 420.0,
        width: 160.0,
        height: 24.0,
    }];

    assert!(!selection_geometry_matches_drag_gesture(
        &stale_rects,
        Point { x: 100.0, y: 100.0 },
        Point { x: 170.0, y: 104.0 },
        false,
    ));
}

#[test]
fn browser_source_disables_scroll_follow_for_fixed_desktop_compromise() {
    assert!(!should_follow_scroll_for_source(
        "chrome.exe",
        "Docs - Google Chrome"
    ));
    assert!(!should_follow_scroll_for_source(
        "msedge.exe",
        "Microsoft Edge"
    ));
    assert!(!should_follow_scroll_for_source(
        "firefox.exe",
        "Mozilla Firefox"
    ));
}

#[test]
fn non_browser_source_keeps_scroll_follow_enabled() {
    assert!(should_follow_scroll_for_source(
        "notepad.exe",
        "note.txt - Notepad"
    ));
    assert!(should_follow_scroll_for_source(
        "WINWORD.EXE",
        "Document1 - Word"
    ));
}

#[test]
fn default_hover_delay_is_one_second() {
    assert_eq!(AppConfig::default().hover_delay_ms, 1_000);
}
