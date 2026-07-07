use serde::{Deserialize, Serialize};

use crate::{
    selection::uia_reader::UiaSelectionResult,
    types::{Point, Rect},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionReadMethod {
    UiAutomation,
    Clipboard,
    HotkeyClipboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionAnchorSource {
    UiAutomationRects,
    ClipboardNoGeometryDragFallback,
    HotkeyCursorFallback,
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
    #[serde(default)]
    pub selection_rects: Vec<Rect>,
    #[serde(default)]
    pub explicit_anchor: Option<Point>,
    #[serde(default)]
    pub anchor_source: Option<SelectionAnchorSource>,
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
            selection_rects: Vec::new(),
            explicit_anchor: None,
            anchor_source: Some(SelectionAnchorSource::ClipboardNoGeometryDragFallback),
        }
    }

    pub fn from_uia_result(
        result: UiaSelectionResult,
        source_app: String,
        window_title: String,
        fallback_point: Point,
    ) -> Option<Self> {
        if result.is_password_control {
            return None;
        }

        let explicit_anchor = result.selection_anchor_point();
        let selection_rects = result.rects;
        let text = result.text?.trim().to_string();
        if text.is_empty() {
            return None;
        }

        Some(Self {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            source_app,
            window_title,
            anchor_rect: None,
            fallback_point,
            read_method: SelectionReadMethod::UiAutomation,
            selection_rects,
            explicit_anchor,
            anchor_source: Some(SelectionAnchorSource::UiAutomationRects),
        })
    }

    pub fn anchor_point(&self) -> Point {
        self.explicit_anchor
            .or_else(|| weighted_rect_center(&self.selection_rects))
            .or_else(|| self.anchor_rect.map(|rect| rect.center()))
            .unwrap_or(self.fallback_point)
    }

    pub fn toolbar_anchor_point(&self) -> Point {
        toolbar_anchor_from_rects(&self.selection_rects, self.fallback_point)
            .or(self.explicit_anchor)
            .or_else(|| {
                toolbar_anchor_from_rects(
                    &self.anchor_rect.into_iter().collect::<Vec<_>>(),
                    self.fallback_point,
                )
            })
            .unwrap_or_else(|| clipboard_toolbar_anchor_from_fallback(self.fallback_point))
    }
}

fn clipboard_toolbar_anchor_from_fallback(fallback_point: Point) -> Point {
    const ESTIMATED_TEXT_TOP_OFFSET: f64 = 18.0;

    Point {
        x: fallback_point.x,
        y: (fallback_point.y - ESTIMATED_TEXT_TOP_OFFSET).max(0.0),
    }
}

fn toolbar_anchor_from_rects(rects: &[Rect], fallback_point: Point) -> Option<Point> {
    const TEXT_SPACE_MIN_HEIGHT: f64 = 48.0;
    const ESTIMATED_TEXT_HALF_HEIGHT: f64 = 18.0;

    if let Some(rect) = rects.iter().copied().filter(is_valid_rect).find(|rect| {
        rect_contains_point(rect, fallback_point) && rect.height >= TEXT_SPACE_MIN_HEIGHT
    }) {
        return Some(Point {
            x: fallback_point.x.clamp(rect.x, rect.x + rect.width),
            y: (fallback_point.y - ESTIMATED_TEXT_HALF_HEIGHT).clamp(rect.y, rect.y + rect.height),
        });
    }

    rects
        .iter()
        .copied()
        .filter(is_valid_rect)
        .next()
        .map(|rect| Point {
            x: rect.x,
            y: rect.y,
        })
}

fn rect_contains_point(rect: &Rect, point: Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

pub fn weighted_rect_center(rects: &[Rect]) -> Option<Point> {
    let mut weighted_x = 0.0;
    let mut weighted_y = 0.0;
    let mut total_area = 0.0;

    for rect in rects.iter().copied().filter(is_valid_rect) {
        let area = rect.width * rect.height;
        let center = rect.center();
        weighted_x += center.x * area;
        weighted_y += center.y * area;
        total_area += area;
    }

    if total_area > 0.0 {
        let y = union_valid_rects(rects)
            .map(|bounds| bounds.center().y)
            .unwrap_or(weighted_y / total_area);
        Some(Point {
            x: weighted_x / total_area,
            y,
        })
    } else {
        None
    }
}

pub fn union_valid_rects(rects: &[Rect]) -> Option<Rect> {
    let mut valid = rects.iter().copied().filter(is_valid_rect);
    let first = valid.next()?;
    let (mut left, mut top, mut right, mut bottom) = (
        first.x,
        first.y,
        first.x + first.width,
        first.y + first.height,
    );

    for rect in valid {
        left = left.min(rect.x);
        top = top.min(rect.y);
        right = right.max(rect.x + rect.width);
        bottom = bottom.max(rect.y + rect.height);
    }

    Some(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn is_valid_rect(rect: &Rect) -> bool {
    rect.width > 0.0 && rect.height > 0.0
}
