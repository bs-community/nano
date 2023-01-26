use futures::future::try_join_all;
use serde_json::json;
use std::{collections::HashSet, env::temp_dir, io::ErrorKind};
use tokio::{
    fs::{self, File},
    io::Result,
};

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
        e => panic!("{e:?}"),
    };

    fs::create_dir(&path).await?;
    fs::write(format!("{}/composer.json", path.display()), b"{}").await?;

    nano::composer::run_composer(&path).await
}

#[tokio::test]
async fn dedupe() -> Result<()> {
    let mut path = temp_dir();
    path.push("dedupe-test");
    let path_display = path.display();

    match fs::remove_dir_all(&path).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        e => panic!("{e:?}"),
    };

    fs::create_dir(&path).await?;

    let mut lock = HashSet::new();
    lock.insert("illuminate/support".to_string());
    lock.insert("blessing/filter".to_string());

    let composer_lock = json!({
        "packages": [
            { "name": "illuminate/support" }
        ]
    });
    let composer_lock_path = format!("{}/composer.lock", path_display);
    fs::write(
        &composer_lock_path,
        &serde_json::to_vec(&composer_lock).unwrap(),
    )
    .await?;
    let composer_json_path = format!("{}/composer.json", path_display);
    fs::write(&composer_json_path, b"").await?;

    let mut vendor_path = path.clone();
    vendor_path.push("vendor");
    fs::create_dir(&vendor_path).await?;
    let creations = lock
        .iter()
        .map(|name| format!("{}/{}", vendor_path.display(), name))
        .map(fs::create_dir_all)
        .collect::<Vec<_>>();
    try_join_all(creations).await?;

    nano::composer::dedupe(&lock, &path, &path_display, &composer_json_path).await?;

    assert_eq!(
        File::open(&composer_json_path).await.unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        File::open(&composer_lock_path).await.unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        File::open(format!("{}/illuminate/support", vendor_path.display()))
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::NotFound
    );
    assert!(
        File::open(format!("{}/blessing/filter", vendor_path.display()))
            .await
            .is_ok()
    );

    Ok(())
}
