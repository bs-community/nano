use futures::future::try_join;
use std::{env::temp_dir, io::ErrorKind};
use tokio::{fs, io::Result};

#[tokio::test]
async fn clean_up() -> Result<()> {
    let mut path = temp_dir();
    path.push("clean_up-test");

    match fs::remove_dir_all(&path).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        e => panic!("{e:?}"),
    };

    let display = path.display();

    let node_modules = format!("{}/node_modules", display);
    fs::create_dir_all(&node_modules).await?;

    let git_ignore = format!("{}/.gitignore", display);
    fs::write(&git_ignore, b"").await?;

    let assets = format!("{}/assets", display);
    fs::create_dir(&assets).await?;
    let js = format!("{}/file.js", assets);
    let ts = format!("{}/file.ts", assets);
    try_join(fs::write(&js, b""), fs::write(&ts, b"")).await?;

    nano::build::clean_up(&path).await;

    assert_eq!(
        fs::File::open(&node_modules).await.unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        fs::File::open(&git_ignore).await.unwrap_err().kind(),
        ErrorKind::NotFound
    );
    assert!(fs::File::open(&js).await.is_ok());
    assert_eq!(
        fs::File::open(&ts).await.unwrap_err().kind(),
        ErrorKind::NotFound
    );

    Ok(())
}
