use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::commands::ai::{
    build_follow_up_prompt_messages, build_prompt_messages, build_prompt_messages_with_target,
};
use selection_ai_assistant_lib::commands::screenshot::build_screenshot_translate_prompt;

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
fn builds_translate_only_prompt_with_explicit_target_language() {
    let messages =
        build_prompt_messages_with_target(AiAction::TranslateOnly, "你好世界", Some("韩文"));

    assert!(messages[1].content.contains("翻译成韩文"));
    assert!(messages[1].content.contains("严格使用目标语言：韩文"));
    assert!(messages[1].content.contains("不要根据原文语言自动切换"));
    assert!(messages[1].content.contains("你好世界"));
}

#[test]
fn builds_morse_code_conversion_prompt_for_output_target() {
    let messages =
        build_prompt_messages_with_target(AiAction::TranslateOnly, "SOS 123", Some("摩斯密码"));

    assert!(messages[1].content.contains("转换成摩斯密码"));
    assert!(messages[1].content.contains("标准摩斯密码"));
    assert!(messages[1].content.contains("/ 分隔"));
    assert!(messages[1].content.contains("SOS 123"));
}

#[test]
fn builds_ancient_or_pictograph_conversion_prompt_for_output_target() {
    let messages = build_prompt_messages_with_target(
        AiAction::TranslateOnly,
        "山川日月",
        Some("甲骨文风格近似转写"),
    );

    assert!(messages[1].content.contains("转换成甲骨文风格近似转写"));
    assert!(messages[1].content.contains("近似转写"));
    assert!(messages[1].content.contains("不要求真实考古字形一一对应"));
    assert!(messages[1].content.contains("山川日月"));
}

#[test]
fn builds_style_rewrite_prompt_for_output_target() {
    let messages =
        build_prompt_messages_with_target(AiAction::TranslateOnly, "今天很好", Some("文言文"));

    assert!(messages[1].content.contains("改写成文言文"));
    assert!(messages[1]
        .content
        .contains("严格遵循目标风格或语体：文言文"));
    assert!(messages[1].content.contains("保留原文含义"));
    assert!(messages[1].content.contains("今天很好"));
}

#[test]
fn builds_screenshot_translate_prompt_with_target_language() {
    let prompt = build_screenshot_translate_prompt(Some("英文"));

    assert!(prompt.contains("识别截图中的可见文字"));
    assert!(prompt.contains("翻译成英文"));
    assert!(prompt.contains("严格使用目标语言：英文"));
    assert!(prompt.contains("不要根据截图文字语言自动切换"));
}

#[test]
fn builds_screenshot_morse_conversion_prompt() {
    let prompt = build_screenshot_translate_prompt(Some("摩斯密码"));

    assert!(prompt.contains("转换成摩斯密码"));
    assert!(prompt.contains("标准摩斯密码"));
    assert!(prompt.contains("/ 分隔"));
    assert!(prompt.contains("不要描述截图画面"));
}

#[test]
fn builds_screenshot_style_rewrite_prompt() {
    let prompt = build_screenshot_translate_prompt(Some("文言文"));

    assert!(prompt.contains("改写成文言文"));
    assert!(prompt.contains("严格遵循目标风格或语体：文言文"));
    assert!(prompt.contains("保留识别文字的原意"));
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
