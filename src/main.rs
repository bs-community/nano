use futures::future::join3;
use nano::{build::build, composer::install_php_dependencies, i18n::I18nStore};
use serde::Serialize;
use std::env;
use tokio::fs;

#[macro_use]
extern crate log;

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

    let (build, composer, _) = join3(
        build(&path, plugins.iter()),
        install_php_dependencies(&path, plugins.iter()),
        save_updated(
            plugins.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            &i18n_store,
        ),
    )
    .await;

    build?;
    composer?;

    Ok(())
}

async fn save_updated(plugins: impl Iterator<Item = (&str, &str)>, i18n_store: &I18nStore) {
    let updated = plugins
        .map(|(name, version)| UpdateInfo {
            name: i18n_store
                .get(name.as_ref())
                .map(|info| info.title.zh_cn.as_str())
                .unwrap_or_default(),
            version: version.as_ref(),
        })
        .collect::<Vec<_>>();
    if let Ok(bytes) = serde_json::to_vec(&updated) {
        if let Err(_) = fs::write("updated.json", &bytes).await {
            warn!("Failed to save updated plugins list.");
        }
    }
}
