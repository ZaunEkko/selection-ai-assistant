use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::commands::ai::{
    build_follow_up_prompt_messages, build_prompt_messages,
};

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

#[test]
fn builds_translate_only_prompt_without_explanations() {
    let messages = build_prompt_messages(AiAction::TranslateOnly, "你好世界");

    assert!(messages[1].content.contains("只输出译文"));
    assert!(messages[1].content.contains("不要解释"));
    assert!(messages[1].content.contains("你好世界"));
}

#[test]
fn builds_expand_prompt_prompt() {
    let messages = build_prompt_messages(AiAction::ExpandPrompt, "帮我写一个产品发布的提示词");

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "system");
    assert!(messages[1].content.contains("扩写"));
    assert!(messages[1].content.contains("优化后提示词"));
    assert!(messages[1].content.contains("主要改进点"));
    assert!(messages[1].content.contains("帮我写一个产品发布的提示词"));
}

#[test]
fn builds_follow_up_prompt_with_original_answer_and_question() {
    let messages = build_follow_up_prompt_messages("hello world", "初始解释", "继续解释它的语气");

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, "system");
    assert!(messages[1].content.contains("用户追问"));
    assert!(messages[1].content.contains("原始选中文本"));
    assert!(messages[1].content.contains("上一轮回答"));
    assert!(messages[1].content.contains("hello world"));
    assert!(messages[1].content.contains("初始解释"));
    assert!(messages[1].content.contains("继续解释它的语气"));
}
