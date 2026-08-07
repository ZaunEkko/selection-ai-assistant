use selection_ai_assistant_lib::config::AppConfig;
use selection_ai_assistant_lib::input_monitor::events::{
    apply_mouse_up_action_to_pending_selection, classify_mouse_up, consume_pending_selection,
    handle_hotkey_state, handle_mouse_button_event, hover_action_for_pending_selection,
    hover_action_for_pending_selection_when_idle, is_drag_distance_met, manual_hotkey_trigger_key,
    predicted_scroll_offset, scroll_tracker_action, selection_geometry_matches_drag_gesture,
    selection_rects_match_drag_gesture, selection_still_trackable,
    selection_still_trackable_on_monitors, settled_measurement_is_stable,
    should_follow_scroll_for_source, update_scroll_ratio, visible_floating_button_action_when_idle,
    HotkeyAction, HotkeyKeyState, MouseButtonEvent, MouseUpAction, PendingHotkeyAction,
    PendingSelection, PendingSelectionHoverAction, ScrollBurst, ScrollPace, ScrollTrackerAction,
    SelectionMouseUpEffect, VisibleFloatingButton, VisibleFloatingButtonAction,
    DEFAULT_PIXELS_PER_WHEEL_DELTA, MAX_SCROLL_MEASURE_FAILURES, SCROLL_SETTLE_IDLE_MS,
    SCROLL_SETTLE_STABLE_EPSILON_PX,
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

// --- 滚动跟随：比例学习 ---

#[test]
fn scroll_ratio_learns_real_step_from_first_measurement() {
    // 一个滚轮刻度 = 120 delta。某编辑器一格滚 3 行 x 19px = 57px。
    let estimate = update_scroll_ratio(None, -120.0, -57.0).expect("应当学到比例");
    assert!((estimate.pixels_per_delta - 0.475).abs() < 1e-6);
    assert_eq!(estimate.samples, 1);
}

#[test]
fn scroll_ratio_blends_later_measurements() {
    let first = update_scroll_ratio(None, -120.0, -57.0).expect("首次测量");
    let second = update_scroll_ratio(Some(first), -120.0, -72.0).expect("二次测量");
    // EMA: 0.475 * 0.6 + 0.6 * 0.4
    assert!((second.pixels_per_delta - 0.525).abs() < 1e-6);
    assert_eq!(second.samples, 2);
}

#[test]
fn scroll_ratio_rejects_measurement_against_wheel_direction() {
    let current = update_scroll_ratio(None, -120.0, -57.0);
    // 向下滚但测到选区往下移：多半是误检到别的高亮块，不能写进估计值。
    let unchanged = update_scroll_ratio(current, -120.0, 57.0);
    assert_eq!(unchanged, current);
}

#[test]
fn scroll_ratio_ignores_noise_sized_movement() {
    let current = update_scroll_ratio(None, -120.0, -57.0);
    let unchanged = update_scroll_ratio(current, -120.0, -1.0);
    assert_eq!(unchanged, current);
}

#[test]
fn scroll_ratio_stays_within_sane_bounds() {
    let absurd = update_scroll_ratio(None, 120.0, 100_000.0).expect("仍应有值");
    assert!(absurd.pixels_per_delta <= 2.2);

    let tiny = update_scroll_ratio(None, 120.0, 3.0).expect("仍应有值");
    assert!(tiny.pixels_per_delta >= 0.08);
}

#[test]
fn predicted_offset_falls_back_to_default_ratio() {
    let offset = predicted_scroll_offset(None, -120.0);
    assert!((offset - (-120.0 * DEFAULT_PIXELS_PER_WHEEL_DELTA)).abs() < 1e-6);
}

#[test]
fn predicted_offset_uses_learned_ratio_once_measured() {
    let estimate = update_scroll_ratio(None, -120.0, -57.0);
    assert!((predicted_scroll_offset(estimate, -120.0) - (-57.0)).abs() < 1e-6);
}

// --- 滚动跟随：快慢节奏 ---

#[test]
fn single_notch_scrolling_counts_as_slow() {
    let mut burst = ScrollBurst::default();
    assert_eq!(burst.register(-120.0, 0), ScrollPace::Slow);
    assert_eq!(burst.register(-120.0, 50), ScrollPace::Slow);
}

#[test]
fn rapid_multi_notch_burst_counts_as_fast() {
    let mut burst = ScrollBurst::default();
    burst.register(-120.0, 0);
    burst.register(-120.0, 50);
    assert_eq!(burst.register(-120.0, 100), ScrollPace::Fast);
}

#[test]
fn scroll_burst_window_expires_and_restarts_slow() {
    let mut burst = ScrollBurst::default();
    burst.register(-120.0, 0);
    burst.register(-120.0, 50);
    assert_eq!(burst.register(-120.0, 100), ScrollPace::Fast);
    // 停止滚动后节奏衰减到零，下一次滚动重新从慢速开始。
    assert_eq!(burst.register(-120.0, 400), ScrollPace::Slow);
    assert!((burst.accumulated_notches() - 1.0).abs() < 1e-6);
}

#[test]
fn sustained_fast_scrolling_does_not_flicker_back_to_slow() {
    // 回归测试：早期的固定翻滚窗口在窗口到期时会把计数清零，持续快滚
    // 会周期性掉回 Slow，操作条随之隐藏/显示反复闪烁。
    let mut burst = ScrollBurst::default();
    let mut paces = Vec::new();
    for tick in 0..15 {
        paces.push(burst.register(-120.0, tick * 40));
    }

    let transitions = paces.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert_eq!(transitions, 1, "持续快滚只应该有一次 Slow -> Fast 切换");
    assert_eq!(paces.last(), Some(&ScrollPace::Fast));
}

#[test]
fn borderline_scroll_speed_does_not_oscillate() {
    // 速度停在阈值附近时，迟滞应当避免在隐藏/显示之间来回跳。
    let mut burst = ScrollBurst::default();
    let mut paces = Vec::new();
    for tick in 0..12 {
        paces.push(burst.register(-120.0, tick * 120));
    }

    let transitions = paces.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert!(
        transitions <= 1,
        "边界速度不应反复切换，实际切换 {transitions} 次"
    );
}

#[test]
fn reading_speed_scrolling_keeps_following() {
    let mut burst = ScrollBurst::default();
    for tick in 0..8 {
        assert_eq!(burst.register(-120.0, tick * 250), ScrollPace::Slow);
    }
}

// --- 滚动跟随：会话状态机 ---

#[test]
fn slow_scroll_keeps_measuring() {
    assert_eq!(
        scroll_tracker_action(ScrollPace::Slow, 0, 0, false),
        ScrollTrackerAction::Measure
    );
}

#[test]
fn fast_scroll_stays_hidden_until_it_settles() {
    assert_eq!(
        scroll_tracker_action(ScrollPace::Fast, 0, 0, false),
        ScrollTrackerAction::WaitHidden
    );
    // 静止后立刻量一次，用于吸附回真实选区。
    assert_eq!(
        scroll_tracker_action(ScrollPace::Fast, SCROLL_SETTLE_IDLE_MS, 0, false),
        ScrollTrackerAction::Measure
    );
}

#[test]
fn session_finishes_after_settled_measurement() {
    assert_eq!(
        scroll_tracker_action(ScrollPace::Fast, SCROLL_SETTLE_IDLE_MS, 0, true),
        ScrollTrackerAction::Finish
    );
    assert_eq!(
        scroll_tracker_action(ScrollPace::Slow, SCROLL_SETTLE_IDLE_MS, 0, true),
        ScrollTrackerAction::Finish
    );
}

/// 回归：空闲时间到了不等于画面停了，此时绝不能判定吸附完成。
///
/// 实测 Windows 11 记事本滚一格，内容移动约 83px，而平滑滚动动画比 150ms
/// 的静止阈值更长。旧实现只要求「静止后测到过一次」，于是在动画播到约一半
/// 时就收尾，把中途位置定死：操作条最终偏离选区约 46px，静置 3 秒也不修正。
/// 同一原因让学到的滚动比例偏小 8 倍（0.080 px/delta，真实约 0.69）。
#[test]
fn settle_requires_motion_to_stop_not_just_idle_time() {
    // 动画中途：空闲时间已到，但相邻两次测量仍有明显位移 => 不算吸附完成。
    // 37px 取自实测：最后一次静止前采样时，内容还差约一半没走完。
    assert!(!settled_measurement_is_stable(true, 37.0, 0.0));
    assert!(!settled_measurement_is_stable(true, -37.0, 0.0));
    // 刚好超过容差也不算。
    assert!(!settled_measurement_is_stable(
        true,
        SCROLL_SETTLE_STABLE_EPSILON_PX,
        0.0
    ));

    // 还没静止时，即使这一帧没动也不能收尾——可能只是动画的匀速段之间。
    assert!(!settled_measurement_is_stable(false, 0.0, 0.0));

    // 静止 + 位移停止 => 才是真正的最终位置。
    assert!(settled_measurement_is_stable(true, 0.0, 0.0));
    assert!(settled_measurement_is_stable(true, 1.0, 0.0));
}

/// 多行选区从视口**顶部**滚出时，UIA 返回的是裁剪后的矩形：首行被上边缘
/// 切掉一截，`y` 被钉在视口顶边不再变化，只有高度还在缩小。只比 y 会看到
/// 连续两帧「没动」而提前判稳，把操作条定死在一个正在滚走的选区上。
#[test]
fn settle_requires_clipping_to_stop_not_just_vertical_motion() {
    // y 不动（被顶边钉住），但高度还在被裁掉 => 内容其实还在滚。
    assert!(!settled_measurement_is_stable(true, 0.0, -18.0));
    // 反向：从顶部滚回来时高度在恢复，同样不算稳。
    assert!(!settled_measurement_is_stable(true, 0.0, 18.0));
    // 刚好达到容差也不算。
    assert!(!settled_measurement_is_stable(
        true,
        0.0,
        SCROLL_SETTLE_STABLE_EPSILON_PX
    ));

    // 高度抖动在容差内（像素扫描的边缘噪声）仍算稳，否则永远收不了尾。
    assert!(settled_measurement_is_stable(true, 0.0, 1.0));
    assert!(settled_measurement_is_stable(true, 0.0, -1.0));

    // 两个维度是与的关系：任一还在变就不算稳。
    assert!(!settled_measurement_is_stable(true, 37.0, -18.0));
}

#[test]
fn repeated_measure_failures_abandon_instead_of_guessing() {
    assert_eq!(
        scroll_tracker_action(ScrollPace::Slow, 0, MAX_SCROLL_MEASURE_FAILURES, false),
        ScrollTrackerAction::Abandon
    );
    // 失败判定优先于其它状态。
    assert_eq!(
        scroll_tracker_action(ScrollPace::Fast, 5_000, MAX_SCROLL_MEASURE_FAILURES, true),
        ScrollTrackerAction::Abandon
    );
}

// --- 滚动跟随：选区是否还看得见 ---

#[test]
fn selection_inside_viewport_is_trackable() {
    let viewport = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    assert!(selection_still_trackable(
        Rect {
            x: 100.0,
            y: 500.0,
            width: 400.0,
            height: 20.0,
        },
        viewport
    ));
}

#[test]
fn selection_scrolled_out_of_viewport_is_not_trackable() {
    let viewport = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    // 滚出顶部
    assert!(!selection_still_trackable(
        Rect {
            x: 100.0,
            y: -100.0,
            width: 400.0,
            height: 20.0,
        },
        viewport
    ));
    // 只剩下几个像素露在底部
    assert!(!selection_still_trackable(
        Rect {
            x: 100.0,
            y: 1076.0,
            width: 400.0,
            height: 20.0,
        },
        viewport
    ));
}

// --- 滚动跟随：窗口伸出桌面之外的部分不算可见 ---

const PRIMARY_MONITOR: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 1920.0,
    height: 1080.0,
};

#[test]
fn selection_outside_the_desktop_is_not_trackable() {
    // 窗口被拖出右边缘，一半在桌面外。
    let window = Rect {
        x: 1600.0,
        y: 100.0,
        width: 800.0,
        height: 600.0,
    };
    let selection_off_desktop = Rect {
        x: 2000.0,
        y: 300.0,
        width: 300.0,
        height: 20.0,
    };

    // 只看窗口边界会误判成可跟踪：选区确实落在窗口矩形之内。
    assert!(selection_still_trackable(selection_off_desktop, window));
    assert!(!selection_still_trackable_on_monitors(
        selection_off_desktop,
        window,
        &[PRIMARY_MONITOR]
    ));
}

#[test]
fn selection_on_the_visible_part_of_the_window_stays_trackable() {
    let window = Rect {
        x: 1600.0,
        y: 100.0,
        width: 800.0,
        height: 600.0,
    };
    assert!(selection_still_trackable_on_monitors(
        Rect {
            x: 1650.0,
            y: 300.0,
            width: 200.0,
            height: 20.0,
        },
        window,
        &[PRIMARY_MONITOR]
    ));
}

#[test]
fn selection_on_a_secondary_monitor_stays_trackable() {
    let secondary = Rect {
        x: 1920.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    // 跨双屏的窗口：选区在副屏上，与主屏没有交集。
    let window = Rect {
        x: 1600.0,
        y: 100.0,
        width: 1200.0,
        height: 600.0,
    };
    assert!(selection_still_trackable_on_monitors(
        Rect {
            x: 2000.0,
            y: 300.0,
            width: 300.0,
            height: 20.0,
        },
        window,
        &[PRIMARY_MONITOR, secondary]
    ));
}

#[test]
fn selection_in_the_gap_between_monitors_is_not_trackable() {
    // 两块屏纵向错开，中间留出一段没有任何显示器覆盖的区域。
    let secondary = Rect {
        x: 1920.0,
        y: 600.0,
        width: 1920.0,
        height: 1080.0,
    };
    let window = Rect {
        x: 1600.0,
        y: 0.0,
        width: 1200.0,
        height: 600.0,
    };
    // y=300 这一行在主屏右侧之外、又在副屏上边缘之上，属于虚拟屏幕外接矩形
    // 里的空隙——用外接矩形判断会漏掉这种情况。
    assert!(!selection_still_trackable_on_monitors(
        Rect {
            x: 2000.0,
            y: 300.0,
            width: 300.0,
            height: 20.0,
        },
        window,
        &[PRIMARY_MONITOR, secondary]
    ));
}

#[test]
fn missing_monitor_information_falls_back_to_the_window_bounds() {
    let window = Rect {
        x: 1600.0,
        y: 100.0,
        width: 800.0,
        height: 600.0,
    };
    let selection = Rect {
        x: 2000.0,
        y: 300.0,
        width: 300.0,
        height: 20.0,
    };
    assert!(selection_still_trackable_on_monitors(
        selection,
        window,
        &[]
    ));
}
