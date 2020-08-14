use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub require: HashMap<String, String>,
}
