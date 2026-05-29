use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiAction {
    TranslateExplain,
    Explain,
    Summarize,
    CodeExplain,
    ErrorExplain,
    MenuFallback,
}

pub fn classify_action(text: &str) -> AiAction {
    let trimmed = text.trim();

    if trimmed.chars().count() < 2 {
        return AiAction::MenuFallback;
    }

    if looks_like_error(trimmed) {
        return AiAction::ErrorExplain;
    }

    if looks_like_code(trimmed) {
        return AiAction::CodeExplain;
    }

    if trimmed.chars().count() > 800 {
        return AiAction::Summarize;
    }

    if english_ratio(trimmed) > 0.65 {
        return AiAction::TranslateExplain;
    }

    if contains_cjk(trimmed) {
        return AiAction::Explain;
    }

    AiAction::MenuFallback
}

fn looks_like_error(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "error",
        "exception",
        "traceback",
        "panic",
        "failed",
        "stack trace",
        "cannot read",
        "undefined",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn looks_like_code(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let code_markers = [
        "function ",
        "class ",
        "import ",
        "export ",
        "select ",
        " from ",
        "fn ",
        "def ",
        "=>",
        "::",
        "{",
        "}",
        "</",
    ];

    code_markers.iter().any(|needle| lower.contains(needle))
}

fn english_ratio(text: &str) -> f64 {
    let mut ascii_alpha = 0usize;
    let mut counted = 0usize;

    for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
        counted += 1;
        if ch.is_ascii_alphabetic() {
            ascii_alpha += 1;
        }
    }

    if counted == 0 {
        0.0
    } else {
        ascii_alpha as f64 / counted as f64
    }
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
}
