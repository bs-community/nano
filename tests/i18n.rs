#[cfg(test)]
mod i18n {
    #[tokio::test]
    async fn top_level_string() {
        let text = nano::i18n::trans("./tests/i18n", "general.title", "en")
            .await
            .unwrap();
        assert_eq!(text, "Title".to_string());

        let text = nano::i18n::trans("./tests/i18n", "Namespace::general.title", "en")
            .await
            .unwrap();
        assert_eq!(text, "Title".to_string());
    }

    #[tokio::test]
    async fn nested_string() {
        let text = nano::i18n::trans("./tests/i18n", "general.nested.key", "en")
            .await
            .unwrap();
        assert_eq!(text, "value".to_string());

        let text = nano::i18n::trans("./tests/i18n", "Namespace::general.nested.key", "en")
            .await
            .unwrap();
        assert_eq!(text, "value".to_string());
    }

    #[tokio::test]
    async fn failures() {
        let result = nano::i18n::trans("./tests/i18n", "", "en").await;
        assert!(result.is_err());

        let result = nano::i18n::trans("./tests/i18n", "general.yuuko", "en")
            .await
            .unwrap();
        assert_eq!(result, "general.yuuko");
    }
}
