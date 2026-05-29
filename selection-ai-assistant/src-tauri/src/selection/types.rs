use serde::{Deserialize, Serialize};

use crate::types::{Point, Rect};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionReadMethod {
    UiAutomation,
    Clipboard,
    HotkeyClipboard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionCandidate {
    pub id: String,
    pub text: String,
    pub source_app: String,
    pub window_title: String,
    pub anchor_rect: Option<Rect>,
    pub fallback_point: Point,
    pub read_method: SelectionReadMethod,
}

impl SelectionCandidate {
    pub fn from_clipboard_text(
        text: String,
        source_app: String,
        window_title: String,
        fallback_point: Point,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            source_app,
            window_title,
            anchor_rect: None,
            fallback_point,
            read_method: SelectionReadMethod::Clipboard,
        }
    }

    pub fn anchor_point(&self) -> Point {
        self.anchor_rect
            .map(|rect| rect.center())
            .unwrap_or(self.fallback_point)
    }
}
