use serde::Deserialize;

#[derive(Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub title: String,
    pub description: String,
}
