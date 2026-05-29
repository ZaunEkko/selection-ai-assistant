use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::commands::ai::build_prompt_messages;

#[test]
fn builds_code_explain_prompt() {
    let messages = build_prompt_messages(AiAction::CodeExplain, "fn main() {}");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "system");
    assert!(messages[1].content.contains("请解释以下代码"));
    assert!(messages[1].content.contains("fn main()"));
}

#[test]
fn builds_translate_explain_prompt() {
    let messages = build_prompt_messages(AiAction::TranslateExplain, "hello world");
    assert!(messages[1].content.contains("翻译成中文"));
    assert!(messages[1].content.contains("hello world"));
}
