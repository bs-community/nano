use futures::{
    future::{try_join3, try_join_all},
    stream::StreamExt,
};
use reqwest::ClientBuilder;
use serde::Deserialize;
use serde_json::from_slice;
use std::{
    collections::HashSet,
    env,
    io::{Error, ErrorKind},
    path::{self, Path},
};
use tokio::{fs, io::Result, process::Command};
use tokio_stream::wrappers::ReadDirStream;

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
        .map_err(|e| {
            error!("Failed to parse composer.lock ({}).", path);
            e
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

    dedupe(lock, &path, &path_display, &composer_json).await
}

pub async fn dedupe(
    lock: &ComposerPackages,
    path: impl AsRef<Path>,
    display: &path::Display<'_>,
    manifest_path: &str,
) -> Result<()> {
    let local_lock = parse_lock(&path).await?;
    let vendor_path = format!("{display}/vendor");

    let deletion = async {
        let deletions = lock
            .intersection(&local_lock)
            .map(|name| vendor_path.clone() + "/" + name)
            .map(fs::remove_dir_all);
        try_join_all(deletions).await?;

        let mut dirs = fs::read_dir(&vendor_path).await?;
        while let Some(dir) = dirs.next_entry().await? {
            let path = dir.path();
            let items = ReadDirStream::new(fs::read_dir(&path).await?);
            if items.count().await == 0 {
                fs::remove_dir(&path).await?;
            }
        }

        Ok(())
    };

    let clean_up = try_join3(
        deletion,
        fs::remove_file(manifest_path),
        fs::remove_file(format!("{display}/composer.lock")),
    );
    if clean_up.await.is_err() {
        warn!("Failed to clean up Composer stuff at '{display}'");
    }

    Ok(())
}

pub async fn install_php_dependencies<S: AsRef<str>>(
    path: impl AsRef<Path>,
    plugins: impl Iterator<Item = (S, S)>,
) -> Result<()> {
    let bs_lock = fetch_bs_lock().await.unwrap_or_else(|e| {
        warn!("Failed to fetch composer.lock of Blessing Skin Server: {e:?}");
        HashSet::default()
    });

    info!("Starting to install PHP dependencies...");

    let jobs = plugins
        .map(|(name, _)| {
            info!("Installing dependencies for plugin '{}'...", name.as_ref());
            install_and_clean(
                &bs_lock,
                format!("{}/plugins/{}", path.as_ref().display(), name.as_ref()),
            )
        })
        .collect::<Vec<_>>();
    try_join_all(jobs).await?;

    info!("Finished to install PHP dependencies.");

    Ok(())
}

async fn fetch_bs_lock() -> reqwest::Result<ComposerPackages> {
    let mut request = ClientBuilder::new()
        .user_agent("Rust reqwest/0.11")
        .build()?
        .get(
            "https://raw.githubusercontent.com/bs-community/blessing-skin-server/dev/composer.lock",
        );

    if let Ok(token) = env::var("GITHUB_TOKEN") {
        request = request.header("Authorization", format!("Bearer {token}"));
    }

    info!("Fetching composer.lock of Blessing Skin Server...");

    request
        .send()
        .await?
        .json::<ComposerLock>()
        .await
        .map(|lock| {
            lock.packages
                .into_iter()
                .map(|package| package.name)
                .collect()
        })
}
