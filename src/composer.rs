use futures::future::{try_join3, try_join_all};
use serde::Deserialize;
use serde_json::from_slice;
use std::{
    collections::HashSet,
    io::{Error, ErrorKind},
    path::{self, Path},
};
use tokio::{fs, io::Result, process::Command, stream::StreamExt};

#[derive(Deserialize)]
struct ComposerLock {
    packages: Vec<ComposerPackage>,
}

pub type ComposerPackages = HashSet<String>;

#[derive(Deserialize)]
pub struct ComposerPackage {
    name: String,
}

pub async fn parse_lock(path: impl AsRef<Path>) -> Result<ComposerPackages> {
    let path = format!("{}/composer.lock", path.as_ref().display());

    info!("Parsing lock file at '{}'", path);

    let json = fs::read(&path).await?;
    let lock = from_slice::<ComposerLock>(&json)
        .or_else(|e| {
            error!("Failed to parse composer.lock ({}).", path);
            Err(e)
        })
        .unwrap();

    let packages = lock
        .packages
        .into_iter()
        .map(|package| package.name)
        .collect();
    Ok(packages)
}

pub async fn run_composer(path: impl AsRef<Path>) -> Result<()> {
    info!("Running Composer at '{}'...", path.as_ref().display());
    let output = Command::new("composer")
        .arg("install")
        .arg("--no-dev")
        .current_dir(&path)
        .output()
        .await?;

    let status = output.status;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        error!(
            "Failed to run Composer at '{}'. Exit code is {}. Detail: {}",
            path.as_ref().display(),
            code,
            String::from_utf8_lossy(&output.stdout)
        );
        return Err(Error::new(ErrorKind::Other, format!("exit code: {}", code)));
    }

    Ok(())
}

async fn install_and_clean(lock: &ComposerPackages, path: impl AsRef<Path>) -> Result<()> {
    let path_display = path.as_ref().display();
    let composer_json = format!("{}/composer.json", path_display);
    if let Err(e) = fs::File::open(&composer_json).await {
        if e.kind() == ErrorKind::NotFound {
            // No composer.json, so there's no need to run Composer.
            return Ok(());
        }
    }

    run_composer(&path).await?;

    dedupe(&lock, &path, &path_display, &composer_json).await
}

pub async fn dedupe<'a>(
    lock: &ComposerPackages,
    path: impl AsRef<Path>,
    display: &path::Display<'a>,
    manifest_path: &str,
) -> Result<()> {
    let local_lock = parse_lock(&path).await?;
    let vendor_path = format!("{}/vendor", display);

    let deletion = async {
        let deletions = lock
            .intersection(&local_lock)
            .map(|name| vendor_path.clone() + "/" + name)
            .map(fs::remove_dir_all);
        try_join_all(deletions).await?;

        let mut dirs = fs::read_dir(&vendor_path).await?;
        while let Some(dir) = dirs.next_entry().await? {
            let path = dir.path();
            let mut items = fs::read_dir(&path).await?;
            if !items.any(|_| true).await {
                fs::remove_dir(&path).await?;
            }
        }

        Ok(())
    };

    let clean_up = try_join3(
        deletion,
        fs::remove_file(manifest_path),
        fs::remove_file(format!("{}/composer.lock", display)),
    );
    if let Err(_) = clean_up.await {
        warn!("Failed to clean up Composer stuff at '{}'", display);
    }

    Ok(())
}

pub async fn install_php_dependencies(
    path: impl AsRef<Path>,
    plugins: impl Iterator<Item = (&str, &str)>,
) -> Result<()> {
    let bs_lock = parse_lock(&path).await?;

    info!("Starting to install PHP dependencies...");

    let jobs = plugins
        .map(|(name, _)| {
            info!("Installing dependencies for plugin '{}'...", name);
            install_and_clean(
                &bs_lock,
                format!("{}/plugins/{}", path.as_ref().display(), name),
            )
        })
        .collect::<Vec<_>>();
    try_join_all(jobs).await?;

    info!("Finished to install PHP dependencies.");

    Ok(())
}
