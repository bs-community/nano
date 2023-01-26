use std::{fs::File, io, path::Path};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

pub fn create_zip<P: AsRef<Path>>(dir: P, dest: P) -> anyhow::Result<()> {
    info!(
        "Zipping files for '{}' to '{}'.",
        dir.as_ref().display(),
        dest.as_ref().display()
    );

    let dest = File::create(dest)?;
    let mut zip = ZipWriter::new(dest);
    let options = FileOptions::default();

    let walk = WalkDir::new(&dir).into_iter();
    for entry in walk {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(dir.as_ref().parent().ok_or_else(|| {
            anyhow::Error::msg(format!(
                "Cannot find parent directory of '{}'.",
                dir.as_ref().display()
            ))
        })?)?;

        if path.is_file() {
            zip.start_file(name.to_string_lossy(), options)?;
            let mut f = File::open(path)?;
            io::copy(&mut f, &mut zip)?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name.to_string_lossy(), options)?;
        }
    }

    zip.finish()?;
    Ok(())
}
