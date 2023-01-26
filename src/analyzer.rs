use crate::types::PackageJson;
use git2::{DiffDelta, DiffLine, DiffOptions, Repository};
use regex::Regex;
use std::{collections::HashMap, path::Path};
use tokio::fs;

pub fn analyze(
    repo_path: impl AsRef<Path>,
) -> Result<(String, HashMap<String, String>), git2::Error> {
    info!(
        "Reading repository at '{}'...",
        repo_path.as_ref().display()
    );
    let repo = Repository::open(repo_path)?;

    let head = repo.head()?.peel_to_commit()?;
    let commit_msg = head.message().unwrap_or("(No commit message.)").trim();
    info!("Head commit is {}: {commit_msg}", head.id());

    let mut opts = DiffOptions::new();
    opts.include_untracked(true).ignore_filemode(true);

    let diff = repo.diff_tree_to_tree(
        Some(head.parent(0)?.tree()?).as_ref(),
        Some(head.tree()?).as_ref(),
        Some(&mut opts),
    )?;

    let mut map = HashMap::new();

    let re_plugin_name = Regex::new(r"plugins/([\w-]+)/package\.json").unwrap();
    let re_version = Regex::new(r#""version": "([\w\.]+)""#).unwrap();

    info!("Analyzing diff of latest commit...");
    diff.foreach(
        &mut |_, _| true,
        None,
        None,
        Some(&mut |delta: DiffDelta, _, line: DiffLine| {
            let file = delta.new_file();
            let path = file
                .path_bytes()
                .map(String::from_utf8_lossy) // we assumed path doesn't contain special chars
                .or_else(|| {
                    error!("Cannot get the path of object {}.", file.id());
                    None
                })
                .unwrap();
            if path.ends_with("package.json") && line.origin() == '+' {
                let content = String::from_utf8_lossy(line.content());
                if let Some(caps) = re_version.captures(&content) {
                    let plugin_name = re_plugin_name
                        .captures(&path)
                        .or_else(|| {
                            error!("Cannot extract plugin name from path: {path}");
                            None
                        })
                        .unwrap()
                        .get(1)
                        .or_else(|| {
                            error!("Cannot extract plugin name from path: {path}");
                            None
                        })
                        .unwrap()
                        .as_str()
                        .to_owned();
                    let version = caps
                        .get(1)
                        .or_else(|| {
                            error!("Cannot extract version of plugin \"{plugin_name}\"");
                            None
                        })
                        .unwrap()
                        .as_str()
                        .to_owned();

                    info!("Version changed: {plugin_name} -> {version}");

                    map.insert(plugin_name, version);
                }
            }
            true
        }),
    )?;

    Ok((commit_msg.to_owned(), map))
}

pub async fn analyze_commit_message(
    message: &str,
    root: impl AsRef<Path>,
    plugins: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    let re_force_update = Regex::new(r"force update: ([\w-]+)").unwrap();
    let plugin_name = re_force_update.captures(message).and_then(|s| s.get(1));
    if let Some(name) = plugin_name {
        let package_json = fs::read_to_string(format!(
            "{}/plugins/{}/package.json",
            root.as_ref().display(),
            name.as_str()
        ))
        .await?;
        let info = serde_json::from_str::<PackageJson>(&package_json)?;
        plugins.insert(name.as_str().to_owned(), info.version);
    }

    Ok(())
}
