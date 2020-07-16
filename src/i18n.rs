use std::path::Path;
use tokio::{fs, io};
use yaml_rust::{ScanError, YamlLoader};

pub async fn trans(path: impl AsRef<Path>, key: &str, lang: &'static str) -> io::Result<String> {
    let key = match key.split("::").last() {
        Some(key) => key,
        None => {
            warn!("I18n key '{}' is incorrect. Translation will fail.", key);
            return Ok(key.to_owned());
        }
    };
    let mut components = key.split('.');
    let path = format!(
        "{}/lang/{}/{}.yml",
        path.as_ref().display(),
        lang,
        components.next().unwrap_or_default()
    );

    let content = fs::read_to_string(&path).await?;
    let text = extract(&content, components)
        .unwrap_or_else(|_| {
            warn!("Failed to parse YAML file: {}", path);
            Some(key.to_owned())
        })
        .unwrap_or_else(|| {
            warn!("Cannot find translation of key '{}'.", key);
            key.to_owned()
        });

    Ok(text)
}

fn extract<'a>(
    content: &str,
    components: impl Iterator<Item = &'a str>,
) -> Result<Option<String>, ScanError> {
    let result = components.fold(
        YamlLoader::load_from_str(content)?[0].clone(),
        |yaml, current| yaml[current].clone(),
    );

    Ok(result.into_string())
}
