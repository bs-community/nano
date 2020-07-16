use std::io::{Error, ErrorKind};
use tokio::{io::Result, process::Command};

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

    let output = Command::new("yarn").arg("build").output().await?;
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

pub async fn build() -> Result<()> {
    yarn().await?;
    webpack().await
}
