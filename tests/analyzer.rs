use git2::{Error, Repository, Signature};
use std::{env::temp_dir, io::ErrorKind, path::Path};
use tokio::fs;

#[tokio::test]
async fn no_plugins_updated() -> anyhow::Result<()> {
    let mut path = temp_dir();
    path.push("bs-plugins_no_updated");

    let repo = init_repo(&path).await?;

    fs::write(format!("{}/text", path.display()), b"1").await?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let sig = signature()?;
    let commit_id = repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])?;

    fs::write(format!("{}/text", path.display()), b"2").await?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let commit = repo.find_commit(commit_id)?;
    let sig = signature()?;
    repo.commit(Some("HEAD"), &sig, &sig, "update", &tree, &[&commit])?;

    let (msg, updated) = nano::analyzer::analyze(&path)?;
    assert_eq!(&msg, "update");
    assert!(updated.is_empty());

    Ok(())
}

#[tokio::test]
async fn some_plugins_updated() -> anyhow::Result<()> {
    let mut path = temp_dir();
    path.push("bs-plugins_has_updated");

    let repo = init_repo(&path).await?;

    fs::write(format!("{}/text", path.display()), b"1").await?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let sig = signature()?;
    let commit_id = repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])?;

    fs::create_dir(format!("{}/plugins", path.display())).await?;
    let plugin_dir = format!("{}/plugins/test", path.display());
    fs::create_dir(&plugin_dir).await?;
    fs::write(
        format!("{}/package.json", plugin_dir),
        b"{\n  \"version\": \"1.0.0\"\n}",
    )
    .await?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let commit = repo.find_commit(commit_id)?;
    let sig = signature()?;
    repo.commit(Some("HEAD"), &sig, &sig, "update", &tree, &[&commit])?;

    let (msg, updated) = nano::analyzer::analyze(&path)?;
    assert_eq!(&msg, "update");
    assert_eq!(updated.get("test").unwrap().as_str(), "1.0.0");

    Ok(())
}

#[cfg(test)]
async fn init_repo(path: impl AsRef<Path>) -> Result<Repository, Error> {
    match fs::remove_dir_all(&path).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        e => panic!("{e:?}"),
    };

    Repository::init(&path)
}

#[cfg(test)]
fn signature() -> Result<Signature<'static>, Error> {
    Signature::now("git", "m@git.me")
}
