use serde::Serialize;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Writes and syncs a same-directory temporary file before atomically replacing the destination.
///
/// # Errors
/// Returns storage errors without first removing an existing destination.
pub fn atomic_write(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

/// Persists compact JSON atomically.
///
/// # Errors
/// Returns serialization or storage errors.
pub fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    atomic_write(path, |file| {
        let mut writer = io::BufWriter::new(file);
        serde_json::to_writer(&mut writer, value).map_err(io::Error::other)?;
        writer.write_all(b"\n")?;
        writer.flush()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_write_preserves_previous_file() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("save.json");
        write_json(&path, &"previous").unwrap();
        assert!(
            atomic_write(&path, |file| {
                file.write_all(b"partial")?;
                Err(io::Error::other("injected failure"))
            })
            .is_err()
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "\"previous\"\n");
    }
}
