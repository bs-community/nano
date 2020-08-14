use crate::{i18n::I18nStore, types::PackageJson};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    process::Output,
};
use tokio::{fs, io::Result, process::Command};

#[derive(Serialize, Deserialize)]
pub struct Registry {
    version: u8,
    packages: Vec<Package>,
}

#[derive(Serialize, Deserialize)]
pub struct Package {
    name: String,
    version: String,
    title: String,
    description: String,
    author: String,
    require: HashMap<String, String>,
    dist: Dist,
}

#[derive(Serialize, Deserialize)]
struct Dist {
    r#type: String,
    url: String,
    shasum: String,
}

pub async fn clone() -> Result<Output> {
    info!("Cloning registry repository into '.dist' directory...");
    Command::new("git")
        .arg("clone")
        .arg("https://github.com/bs-community/plugins-dist.git")
        .arg(".dist")
        .arg("--depth=1")
        .output()
        .await
}

#[allow(dead_code)]
fn to_map(list: Vec<Package>) -> BTreeMap<String, Package> {
    list.into_iter()
        .map(|package| (package.name.clone(), package))
        .collect()
}

#[allow(dead_code)]
fn to_list(map: BTreeMap<String, Package>) -> Vec<Package> {
    map.into_iter().map(|(_, package)| package).collect()
}

async fn read_registry(path: impl AsRef<Path>) -> Result<BTreeMap<String, Package>> {
    let json = fs::read_to_string(path).await?;
    let registry = serde_json::from_str::<Registry>(&json)
        .or_else(|e| {
            error!("Failed to parse previous registry data.");
            Err(e)
        })
        .expect("Failed to parse previous registry data.");

    Ok(to_map(registry.packages))
}

async fn update_registry<'a, S1, S2>(
    packages: &'a mut BTreeMap<String, Package>,
    plugins_dir: S1,
    updated: impl Iterator<Item = (S2, S2)>,
    lang: &'static str,
    i18n_store: &'a I18nStore,
) -> Result<()>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    for (name, version) in updated {
        let name = name.as_ref();
        let version = version.as_ref();
        let json = fs::read_to_string(format!(
            "{}/plugins/{}/package.json",
            plugins_dir.as_ref(),
            name
        ))
        .await?;
        let package_json = serde_json::from_str::<PackageJson>(&json)
            .or_else(|e| {
                error!("Failed to parse 'package.json` of plugin '{}'.", name);
                Err(e)
            })
            .expect("Failed to parse 'package.json' file.");

        let i18n = i18n_store
            .get(&name)
            .or_else(|| {
                error!("Cannot retrieve i18n texts of plugin {}.", name);
                None
            })
            .expect("Cannot retrieve i18n texts.");

        packages.insert(
            name.to_owned(),
            Package {
                name: package_json.name,
                version: package_json.version,
                title: match lang {
                    "en" => i18n.title.en.clone(),
                    "zh_CN" => i18n.title.zh_cn.clone(),
                    _ => package_json.title,
                },
                description: match lang {
                    "en" => i18n.description.en.clone(),
                    "zh_CN" => i18n.description.zh_cn.clone(),
                    _ => package_json.description,
                },
                author: package_json.author,
                require: package_json.require,
                dist: Dist {
                    r#type: String::from("zip"),
                    url: format!(
                        "https://cdn.jsdelivr.net/gh/bs-community/plugins-dist/{}_{}.zip",
                        name, version
                    ),
                    shasum: String::from("<hash>"),
                },
            },
        );
    }

    Ok(())
}

async fn write_registry(path: impl AsRef<Path>, packages: BTreeMap<String, Package>) -> Result<()> {
    let registry = Registry {
        version: 1,
        packages: to_list(packages),
    };
    let json = serde_json::to_vec_pretty(&registry).expect("Failed to serialize registry to JSON.");

    fs::write(path, &json).await
}

pub async fn operate_registry<S: AsRef<str>>(
    path: S,
    plugins_dir: S,
    updated: &HashMap<String, String>,
    i18n_store: &I18nStore,
) -> Result<()> {
    for lang in &["en", "zh_CN"] {
        let path = path.as_ref().replace("{lang}", lang);
        let mut packages = read_registry(&path).await?;
        update_registry(
            &mut packages,
            &plugins_dir,
            updated.iter(),
            lang,
            &i18n_store,
        )
        .await?;
        write_registry(&path, packages).await?;
    }

    Ok(())
}
