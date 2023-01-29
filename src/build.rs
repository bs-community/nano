use futures::future::join_all;
use std::{
    io::{Error, ErrorKind},
    path::Path,
};
use tokio::{fs, io::Result, process::Command};

async fn pnpm(root: impl AsRef<Path>) -> Result<()> {
    info!("Running pnpm to install dependencies...");

    let output = Command::new("pnpm")
        .arg("i")
        .current_dir(root)
        .output()
        .await?;
    let status = output.status;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        error!(
            "Failed to run pnpm to install dependencies. Detail: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        return Err(Error::new(ErrorKind::Other, format!("exit code: {code}")));
    }

    Ok(())
}

async fn webpack(root: impl AsRef<Path>) -> Result<()> {
    info!("Running webpack...");

    let output = Command::new("pnpm")
        .arg("build")
        .current_dir(root)
        .env("NODE_ENV", "production")
        .output()
        .await?;
    let status = output.status;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        error!(
            "Failed to run webpack. Detail: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        return Err(Error::new(ErrorKind::Other, format!("exit code: {code}")));
    }

    Ok(())
}

async fn remove_source_files(path: impl AsRef<Path>) -> Result<()> {
    let mut items = fs::read_dir(&path).await?;
    while let Some(item) = items.next_entry().await? {
        let path = item.path();
        let file_type = item.file_type().await?;
        if file_type.is_file() && path.extension().map(is_source_file).unwrap_or_default() {
            fs::remove_file(path).await?;
        }
    }

    Ok(())
}

fn is_source_file(ext: &std::ffi::OsStr) -> bool {
    ext == "ts" || ext == "tsx" || ext == "scss"
}

pub async fn clean_up(path: impl AsRef<Path>) {
    let path = path.as_ref().display();

    let node_modules = format!("{path}/node_modules");
    if fs::File::open(&node_modules).await.is_ok()
        && fs::remove_dir_all(&node_modules).await.is_err()
    {
        warn!("Failed to clean 'node_modules' directory at '{path}'.");
    }

    let git_ignore = format!("{path}/.gitignore");
    if fs::File::open(&git_ignore).await.is_ok() && fs::remove_file(&git_ignore).await.is_err() {
        warn!("Failed to delete '.gitignore' file at '{path}'.");
    }

    let source_files = format!("{path}/assets");
    if fs::File::open(&source_files).await.is_ok()
        && remove_source_files(&source_files).await.is_err()
    {
        warn!("Failed to clean source files at '{path}'.");
    }
}

pub async fn build<S: AsRef<str>>(
    root: impl AsRef<Path>,
    plugins: impl Iterator<Item = (S, S)>,
) -> Result<()> {
    pnpm(&root).await?;
    webpack(&root).await?;

    let root = root.as_ref();

    let cleans = plugins.map(|(name, _)| {
        let name = name.as_ref();
        info!("Cleaning up for plugin '{name}'...");
        let path = format!("{}/plugins/{name}", root.display());
        async move { clean_up(&path).await }
    });
    join_all(cleans.collect::<Vec<_>>()).await;

    Ok(())
}
