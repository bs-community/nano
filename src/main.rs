use nano::i18n::I18nStore;
use std::env;
use serde::Serialize;
use tokio::fs;

#[macro_use] extern crate log;

#[derive(Serialize)]
struct UpdateInfo<'a> {
    name: &'a str,
    version: &'a str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    println!("Blessing Skin Plugins Marketplace Builder.");

    let path = env::var("PLUGINS_DIR").unwrap_or_else(|_| String::from("."));

    let (_, plugins) = nano::analyzer::analyze(&path)?;
    if plugins.is_empty() {
        return Ok(());
    }

    let i18n_store = I18nStore::create(&path, plugins.keys()).await;

    let updated = plugins
        .iter()
        .map(|(name, version)| UpdateInfo {
            name: i18n_store
                .get(&name)
                .map(|info| info.title.zh_cn.as_str())
                .unwrap_or_default(),
            version: version.as_str(),
        })
        .collect::<Vec<_>>();
    if let Ok(bytes) = serde_json::to_vec(&updated) {
        if let Err(_) = fs::write("updated.json", &bytes).await {
            warn!("Failed to save updated plugins list.");
        }
    }

    Ok(())
}
