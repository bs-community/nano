use futures::future::{join_all, try_join3};
use std::{
    io::{Error, ErrorKind},
    path::Path,
};
use tokio::{fs, io::Result, process::Command};

async fn yarn() -> Result<()> {
    info!("Running Yarn to install dependencies...");

    let output = Command::new("yarn").output().await?;
    let status = output.status;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        error!(
            "Failed to run Yarn to install dependencies. Detail: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        return Err(Error::new(ErrorKind::Other, format!("exit code: {}", code)));
    }

    Ok(())
}

async fn webpack() -> Result<()> {
    info!("Running webpack...");

    let output = Command::new("yarn")
        .arg("build")
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
        return Err(Error::new(ErrorKind::Other, format!("exit code: {}", code)));
    }

    Ok(())
}

async fn remove_ts_files(path: impl AsRef<Path>) -> Result<()> {
    let mut items = fs::read_dir(&path).await?;
    while let Some(item) = items.next_entry().await? {
        let path = item.path();
        let file_type = item.file_type().await?;
        if file_type.is_file() && path.extension().map(|ext| ext == "ts").unwrap_or(false) {
            fs::remove_file(path).await?;
        }
    }

    Ok(())
}

pub async fn clean_up(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref().display();

    let node_modules = fs::remove_dir_all(format!("{}/node_modules", path));
    let git_ignore = fs::remove_file(format!("{}/.gitignore", path));
    let ts_files = remove_ts_files(format!("{}/assets", path));

    try_join3(node_modules, git_ignore, ts_files)
        .await
        .map(|_| ())
}

pub async fn build<S: AsRef<str>>(
    root: impl AsRef<Path>,
    plugins: impl Iterator<Item = (S, S)>,
) -> Result<()> {
    yarn().await?;
    webpack().await?;

    let root = root.as_ref();

    let cleans = plugins.map(|(name, _)| {
        let name = name.as_ref();
        info!("Cleaning up for plugin '{}'...", name);
        let path = format!("{}/plugins/{}", root.display(), name);
        async move {
            if let Err(_) = clean_up(&path).await {
                warn!("Failed to clean up files at '{}'", path);
            }
        }
    });
    join_all(cleans.collect::<Vec<_>>()).await;

    Ok(())
}
