use selection_ai_assistant_lib::ai::action_classifier::{classify_action, AiAction};

#[test]
fn classifies_error_before_code() {
    let text = "TypeError: Cannot read properties of undefined\n    at main.ts:42";
    assert_eq!(classify_action(text), AiAction::ErrorExplain);
}

#[test]
fn classifies_code_snippet() {
    let text = "import React from 'react';\nexport function App() { return <div />; }";
    assert_eq!(classify_action(text), AiAction::CodeExplain);
}

#[test]
fn classifies_long_text_as_summary() {
    let text = "这是一段很长的中文内容。".repeat(80);
    assert_eq!(classify_action(&text), AiAction::Summarize);
}

#[test]
fn classifies_english_as_translate_explain() {
    let text = "The quick brown fox jumps over the lazy dog.";
    assert_eq!(classify_action(text), AiAction::TranslateExplain);
}

#[test]
fn classifies_chinese_as_explain() {
    let text = "向量数据库的召回率是什么意思";
    assert_eq!(classify_action(text), AiAction::Explain);
}

#[test]
fn falls_back_for_tiny_or_empty_text() {
    assert_eq!(classify_action(""), AiAction::MenuFallback);
    assert_eq!(classify_action("a"), AiAction::MenuFallback);
}

#[test]
fn serializes_expand_prompt_action_as_camel_case() {
    assert_eq!(
        serde_json::from_str::<AiAction>("\"expandPrompt\"").unwrap(),
        AiAction::ExpandPrompt,
    );
    assert_eq!(
        serde_json::to_string(&AiAction::ExpandPrompt).unwrap(),
        "\"expandPrompt\"",
    );
}

#[test]
fn serializes_translate_only_action_as_camel_case() {
    assert_eq!(
        serde_json::from_str::<AiAction>("\"translateOnly\"").unwrap(),
        AiAction::TranslateOnly,
    );
    assert_eq!(
        serde_json::to_string(&AiAction::TranslateOnly).unwrap(),
        "\"translateOnly\"",
    );
}
