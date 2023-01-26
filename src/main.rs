use futures::try_join;
use nano::{
    analyzer, build::build, composer::install_php_dependencies, i18n::I18nStore, registry,
    zip::create_zip,
};
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

    let (message, mut plugins) = analyzer::analyze(&path)?;
    analyzer::analyze_commit_message(&message, &path, &mut plugins).await?;
    if plugins.is_empty() {
        return Ok(());
    }

    let i18n_store = I18nStore::create(&path, plugins.keys()).await;

    try_join!(
        build(&path, plugins.iter()),
        install_php_dependencies(&path, plugins.iter()),
    )?;

    for (name, version) in &plugins {
        create_zip(
            format!("{path}/plugins/{name}"),
            format!(".dist/{name}_{version}.zip"),
        )?;
    }

    registry::operate_registry(".dist", &path, &plugins, &i18n_store).await?;

    save_updated(
        plugins.iter().map(|(k, v)| (k.as_str(), v.as_str())),
        &i18n_store,
    )
    .await;

    Ok(())
}

async fn save_updated(plugins: impl Iterator<Item = (&str, &str)>, i18n_store: &I18nStore) {
    let updated = plugins
        .map(|(name, version)| UpdateInfo {
            name: i18n_store
                .get(name.as_ref())
                .map(|info| info.title.zh_cn.as_str())
                .unwrap_or_default(),
            version,
        })
        .collect::<Vec<_>>();
    if let Ok(bytes) = serde_json::to_vec(&updated) {
        if fs::write("updated.json", &bytes).await.is_err() {
            warn!("Failed to save updated plugins list.");
        }
    }
}
