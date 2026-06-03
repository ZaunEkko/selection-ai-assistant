use serde::{Deserialize, Serialize};

use crate::{
    selection::types::{union_valid_rects, weighted_rect_center},
    types::{Point, Rect},
};

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

    pub fn selection_anchor_point(&self) -> Option<Point> {
        weighted_rect_center(&self.rects)
    }

    pub fn selection_bounds(&self) -> Option<Rect> {
        union_valid_rects(&self.rects)
    }

    pub fn prefer_focused_attempt(
        focused: Option<UiaSelectionResult>,
        window: Option<UiaSelectionResult>,
    ) -> Option<UiaSelectionResult> {
        match (focused, window) {
            (Some(focused), _window) if focused.is_password_control => Some(focused),
            (Some(focused), Some(window)) => {
                let focused_has_geometry = focused.selection_anchor_point().is_some();
                let window_has_geometry = window.selection_anchor_point().is_some();

                if !focused_has_geometry && window_has_geometry {
                    Some(window)
                } else {
                    Some(focused)
                }
            }
            (focused, window) => focused.or(window),
        }
    }
}

#[cfg(windows)]
pub fn read_current_uia_selection_from_hwnd(
    hwnd: *mut std::ffi::c_void,
) -> Option<UiaSelectionResult> {
    windows_uia::read_current_uia_selection_from_hwnd(hwnd)
}

#[cfg(windows)]
mod windows_uia {
    use super::{SelectionConfidence, UiaSelectionResult};
    use crate::types::Rect;
    use windows::Win32::{
        Foundation::HWND,
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            Ole::{
                SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound,
                SafeArrayUnaccessData,
            },
        },
        UI::Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextRange,
            UIA_TextPatternId,
        },
    };

    pub fn read_current_uia_selection_from_hwnd(
        hwnd: *mut std::ffi::c_void,
    ) -> Option<UiaSelectionResult> {
        if hwnd.is_null() {
            return None;
        }

        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let should_uninitialize = hr.is_ok();
            let result = read_selection_from_initialized_com(HWND(hwnd));
            if should_uninitialize {
                CoUninitialize();
            }
            result
        }
    }

    unsafe fn read_selection_from_initialized_com(hwnd: HWND) -> Option<UiaSelectionResult> {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let focused_result = automation
            .GetFocusedElement()
            .ok()
            .and_then(|element| read_selection_from_element(&element));
        if focused_result
            .as_ref()
            .map(|result| result.is_password_control)
            .unwrap_or(false)
        {
            return focused_result;
        }

        let window_result = automation
            .ElementFromHandle(hwnd)
            .ok()
            .and_then(|element| read_selection_from_element(&element));

        UiaSelectionResult::prefer_focused_attempt(focused_result, window_result)
    }

    unsafe fn read_selection_from_element(
        element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
    ) -> Option<UiaSelectionResult> {
        let is_password_control = element
            .CurrentIsPassword()
            .ok()
            .map(|value| value.as_bool())
            .unwrap_or(false);
        if is_password_control {
            return Some(UiaSelectionResult {
                text: None,
                rects: Vec::new(),
                is_password_control: true,
                confidence: SelectionConfidence::Low,
            });
        }

        let text_pattern: IUIAutomationTextPattern =
            element.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
        let ranges = text_pattern.GetSelection().ok()?;
        let length = ranges.Length().ok()?;
        if length <= 0 {
            return None;
        }

        let mut rects = Vec::new();
        let mut selected_text = String::new();
        for index in 0..length {
            let range = ranges.GetElement(index).ok()?;
            let text = range
                .GetText(-1)
                .ok()
                .map(|text| text.to_string())
                .unwrap_or_default();
            if !text.trim().is_empty() {
                if !selected_text.is_empty() {
                    selected_text.push('\n');
                }
                selected_text.push_str(text.trim());
            }
            rects.extend(rects_from_range(&range));
        }

        if selected_text.trim().is_empty() && rects.is_empty() {
            return None;
        }

        Some(UiaSelectionResult {
            text: if selected_text.trim().is_empty() {
                None
            } else {
                Some(selected_text)
            },
            rects,
            is_password_control: false,
            confidence: SelectionConfidence::High,
        })
    }

    unsafe fn rects_from_range(range: &IUIAutomationTextRange) -> Vec<Rect> {
        let safe_array = match range.GetBoundingRectangles() {
            Ok(array) if !array.is_null() => array,
            _ => return Vec::new(),
        };

        let rects = rects_from_safearray(safe_array);
        let _ = SafeArrayDestroy(safe_array);
        rects
    }

    unsafe fn rects_from_safearray(
        safe_array: *mut windows::Win32::System::Com::SAFEARRAY,
    ) -> Vec<Rect> {
        let lower = match SafeArrayGetLBound(safe_array, 1) {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };
        let upper = match SafeArrayGetUBound(safe_array, 1) {
            Ok(value) => value,
            Err(_) => return Vec::new(),
        };
        if upper < lower {
            return Vec::new();
        }

        let len = (upper - lower + 1) as usize;
        if len < 4 {
            return Vec::new();
        }

        let mut data = std::ptr::null_mut();
        if SafeArrayAccessData(safe_array, &mut data).is_err() || data.is_null() {
            return Vec::new();
        }

        let values = std::slice::from_raw_parts(data as *const f64, len);
        let rects = values
            .chunks_exact(4)
            .filter_map(|chunk| {
                let rect = Rect {
                    x: chunk[0],
                    y: chunk[1],
                    width: chunk[2],
                    height: chunk[3],
                };
                (rect.width > 0.0 && rect.height > 0.0).then_some(rect)
            })
            .collect();

        let _ = SafeArrayUnaccessData(safe_array);
        rects
    }
}
