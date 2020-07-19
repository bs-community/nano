#[tokio::test]
async fn top_level_string() {
    let text = nano::i18n::trans("./tests/i18n", "general.title", "en").await;
    assert_eq!(text, "Title".to_string());

    let text = nano::i18n::trans("./tests/i18n", "Namespace::general.title", "en").await;
    assert_eq!(text, "Title".to_string());
}

#[tokio::test]
async fn nested_string() {
    let text = nano::i18n::trans("./tests/i18n", "general.nested.key", "en").await;
    assert_eq!(text, "value".to_string());

    let text = nano::i18n::trans("./tests/i18n", "Namespace::general.nested.key", "en").await;
    assert_eq!(text, "value".to_string());
}

#[tokio::test]
async fn failures() {
    let result = nano::i18n::trans("./tests/i18n", "", "en").await;
    assert_eq!(result, "");

    let result = nano::i18n::trans("./tests/i18n", "general.yuuko", "en").await;
    assert_eq!(result, "general.yuuko");
}
