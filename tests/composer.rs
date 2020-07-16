use std::{env::temp_dir, io::ErrorKind};
use tokio::{fs, io::Result};

#[tokio::test]
async fn parse_lock() -> Result<()> {
    let packages = nano::composer::parse_lock("./tests/composer").await?;

    assert!(packages.get("blessing/filter").is_some());

    Ok(())
}

#[tokio::test]
async fn run_composer() -> Result<()> {
    let mut path = temp_dir();
    path.push("composer-test");

    match fs::remove_dir_all(&path).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        e => panic!(e),
    };

    fs::create_dir(&path).await?;
    fs::write(format!("{}/composer.json", path.display()), b"{}").await?;

    nano::composer::run_composer(&path).await
}
