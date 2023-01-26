use crate::types;
use std::{collections::HashMap, path::Path};
use tokio::fs;
use yaml_rust::{ScanError, YamlLoader};

pub struct I18nStore {
    store: HashMap<String, PluginInfo>,
}

#[derive(Clone)]
pub struct PluginInfo {
    pub title: Text,
    pub description: Text,
}

#[derive(Clone)]
pub struct Text {
    pub en: String,
    pub zh_cn: String,
}

impl I18nStore {
    pub async fn create<S: AsRef<str>>(
        root: impl AsRef<Path>,
        plugins: impl Iterator<Item = S>,
    ) -> I18nStore {
        let root = root.as_ref().display();

        let mut store = HashMap::new();

        for plugin in plugins {
            let plugin = plugin.as_ref();
            let path = format!("{root}/plugins/{plugin}");
            let package_json = match fs::read(format!("{path}/package.json")).await {
                Ok(bytes) => match serde_json::from_slice::<types::PackageJson>(&bytes) {
                    Ok(manifest) => manifest,
                    Err(_) => {
                        warn!("Failed to parse 'package.json' of '{}'.", plugin);
                        continue;
                    }
                },
                Err(_) => {
                    warn!("Failed to open file 'package.json' of '{}'.", plugin);
                    continue;
                }
            };

            let title = Text {
                en: trans(&path, &package_json.title, "en").await,
                zh_cn: trans(&path, &package_json.title, "zh_CN").await,
            };
            let description = Text {
                en: trans(&path, &package_json.description, "en").await,
                zh_cn: trans(&path, &package_json.description, "zh_CN").await,
            };
            let plugin_info = PluginInfo { title, description };
            store.insert(plugin.to_string(), plugin_info);
        }

        I18nStore { store }
    }

    pub fn get(&self, plugin: &str) -> Option<&PluginInfo> {
        self.store.get(plugin)
    }
}

pub async fn trans(path: impl AsRef<Path>, key: &str, lang: &'static str) -> String {
    let key = match key.split("::").last() {
        Some(key) => key,
        None => {
            warn!("I18n key '{key}' is incorrect. Translation will fail.");
            return key.to_owned();
        }
    };
    let mut components = key.split('.');
    let path = format!(
        "{}/lang/{lang}/{}.yml",
        path.as_ref().display(),
        components.next().unwrap_or_default()
    );

    let content = match fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(_) => return key.to_owned(),
    };

    extract(&content, components)
        .unwrap_or_else(|_| {
            warn!("Failed to parse YAML file: {path}");
            Some(key.to_owned())
        })
        .unwrap_or_else(|| {
            warn!("Cannot find translation of key '{key}'.");
            key.to_owned()
        })
}

fn extract<'a>(
    content: &str,
    components: impl Iterator<Item = &'a str>,
) -> Result<Option<String>, ScanError> {
    let result = components.fold(
        YamlLoader::load_from_str(content)?[0].clone(),
        |yaml, current| yaml[current].clone(),
    );

    Ok(result.into_string())
}
