use serde::{Deserialize, Serialize};

use crate::selection::types::SelectionCandidate;
use crate::types::Point;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionStateKind {
    Idle,
    SelectionProbing,
    SelectionArmed,
    ButtonVisible,
    PanelOpen,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionEvent {
    StartProbing,
    ReadSuccess(SelectionCandidate),
    ReadFailed,
    MouseHoverSatisfied(Point),
    ClickButton,
    Timeout,
    EscapePressed,
    ForegroundWindowChanged,
    KeyboardInputStarted,
    ClosePanel,
}

pub struct SelectionController {
    state: SelectionStateKind,
    candidate: Option<SelectionCandidate>,
}

impl SelectionController {
    pub fn new() -> Self {
        Self {
            state: SelectionStateKind::Idle,
            candidate: None,
        }
    }

    pub fn state_kind(&self) -> SelectionStateKind {
        self.state
    }

    pub fn candidate(&self) -> Option<&SelectionCandidate> {
        self.candidate.as_ref()
    }

    pub fn handle(&mut self, event: SelectionEvent) {
        match event {
            SelectionEvent::StartProbing => {
                self.state = SelectionStateKind::SelectionProbing;
                self.candidate = None;
            }
            SelectionEvent::ReadSuccess(candidate) => {
                self.candidate = Some(candidate);
                self.state = SelectionStateKind::SelectionArmed;
            }
            SelectionEvent::MouseHoverSatisfied(_) if self.candidate.is_some() => {
                self.state = SelectionStateKind::ButtonVisible;
            }
            SelectionEvent::ClickButton if self.candidate.is_some() => {
                self.state = SelectionStateKind::PanelOpen;
            }
            SelectionEvent::ReadFailed
            | SelectionEvent::Timeout
            | SelectionEvent::EscapePressed
            | SelectionEvent::ForegroundWindowChanged
            | SelectionEvent::KeyboardInputStarted
            | SelectionEvent::ClosePanel => {
                self.state = SelectionStateKind::Idle;
                self.candidate = None;
            }
            SelectionEvent::MouseHoverSatisfied(_) | SelectionEvent::ClickButton => {}
        }
    }
}

impl Default for SelectionController {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_point_near_anchor(anchor: Point, point: Point, radius: f64) -> bool {
    let dx = anchor.x - point.x;
    let dy = anchor.y - point.y;
    ((dx * dx) + (dy * dy)).sqrt() <= radius
}
