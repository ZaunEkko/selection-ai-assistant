use serde::{Deserialize, Serialize};

use crate::types::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiaSelectionResult {
    pub text: Option<String>,
    pub rects: Vec<Rect>,
    pub is_password_control: bool,
    pub confidence: SelectionConfidence,
}

impl UiaSelectionResult {
    pub fn is_usable(&self) -> bool {
        if self.is_password_control {
            return false;
        }

        self.text
            .as_ref()
            .map(|text| !text.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn primary_rect(&self) -> Option<Rect> {
        self.rects.first().copied()
    }
}
