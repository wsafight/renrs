use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"RENRSAR1";
const FORMAT_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    entries: Vec<ArchiveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub path: String,
    pub offset: u64,
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct ResourceArchive {
    path: PathBuf,
    payload_start: u64,
    entries: Vec<ArchiveEntry>,
    index: HashMap<String, usize>,
}

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("could not access archive data: {0}")]
    Io(#[from] std::io::Error),
    #[error("archive manifest is invalid: {0}")]
    Manifest(#[from] serde_json::Error),
    #[error("file is not a RenRS resource archive")]
    Magic,
    #[error("unsupported resource archive version {0}")]
    Version(u32),
    #[error("archive manifest is too large")]
    ManifestTooLarge,
    #[error("unsafe or duplicate archive path `{0}`")]
    InvalidPath(String),
    #[error("archive entry `{0}` points outside the file")]
    InvalidBounds(String),
    #[error("archive entry `{0}` failed its SHA-256 check")]
    Checksum(String),
    #[error("archive does not contain `{0}`")]
    Missing(String),
    #[error("refusing to overwrite `{0}` while extracting")]
    Exists(String),
    #[error("refusing to extract through symbolic link `{0}`")]
    Symlink(String),
}

/// Packs visible regular files below `root` into one deterministic archive.
///
/// # Errors
///
/// Returns an error for inaccessible files, unsafe paths, serialization
/// failures, or an unwritable destination.
pub fn pack_project(root: &Path, destination: &Path) -> Result<usize, ArchiveError> {
    let mut paths = crate::resources::collect_files(root)?;
    let temporary = destination.with_extension("renrs.tmp");
    paths.retain(|path| path != destination && path != &temporary);
    paths.sort_by_key(|path| relative_name(root, path));

    let mut entries = Vec::with_capacity(paths.len());
    let mut offset = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024];
    for path in &paths {
        let name = relative_name(root, path);
        if !safe_relative_path(&name) {
            return Err(ArchiveError::InvalidPath(name));
        }
        let (length, sha256) =
            copy_hashed(&mut File::open(path)?, &mut std::io::sink(), &mut buffer)?;
        entries.push(ArchiveEntry {
            path: name,
            offset,
            length,
            sha256,
        });
        offset = offset
            .checked_add(length)
            .ok_or_else(|| ArchiveError::InvalidBounds("archive payload".to_owned()))?;
    }

    let entry_count = entries.len();
    let manifest_data = Manifest {
        version: FORMAT_VERSION,
        entries,
    };
    let manifest = serde_json::to_vec(&manifest_data)?;
    let manifest_length =
        u64::try_from(manifest.len()).map_err(|_| ArchiveError::ManifestTooLarge)?;
    if manifest_length > MAX_MANIFEST_BYTES {
        return Err(ArchiveError::ManifestTooLarge);
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)?;
    output.write_all(MAGIC)?;
    output.write_all(&manifest_length.to_le_bytes())?;
    output.write_all(&manifest)?;
    for (path, entry) in paths.iter().zip(&manifest_data.entries) {
        let (length, digest) = copy_hashed(&mut File::open(path)?, &mut output, &mut buffer)?;
        if length != entry.length || digest != entry.sha256 {
            return Err(ArchiveError::Checksum(entry.path.clone()));
        }
    }
    output.flush()?;
    output.sync_all()?;
    drop(output);
    replace_file(&temporary, destination)?;
    Ok(entry_count)
}

fn copy_hashed(
    input: &mut impl Read,
    output: &mut impl Write,
    buffer: &mut [u8],
) -> Result<(u64, String), std::io::Error> {
    let mut digest = Sha256::new();
    let mut length = 0_u64;
    loop {
        let count = input.read(buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
        length += count as u64;
    }
    Ok((length, format!("{:x}", digest.finalize())))
}

impl ResourceArchive {
    /// Opens and validates an archive manifest and all entry boundaries.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed headers/manifests, unsafe paths,
    /// duplicate entries, unsupported versions, or out-of-bounds entries.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ArchiveError> {
        let path = path.into();
        let mut file = File::open(&path)?;
        let file_length = file.metadata()?.len();
        let mut magic = [0_u8; 8];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(ArchiveError::Magic);
        }
        let mut encoded_length = [0_u8; 8];
        file.read_exact(&mut encoded_length)?;
        let manifest_length = u64::from_le_bytes(encoded_length);
        if manifest_length > MAX_MANIFEST_BYTES {
            return Err(ArchiveError::ManifestTooLarge);
        }
        let encoded_length =
            usize::try_from(manifest_length).map_err(|_| ArchiveError::ManifestTooLarge)?;
        let mut encoded = vec![0; encoded_length];
        file.read_exact(&mut encoded)?;
        let manifest: Manifest = serde_json::from_slice(&encoded)?;
        if manifest.version != FORMAT_VERSION {
            return Err(ArchiveError::Version(manifest.version));
        }
        let payload_start = 16_u64
            .checked_add(manifest_length)
            .ok_or(ArchiveError::ManifestTooLarge)?;
        let mut index = HashMap::new();
        for (entry_index, entry) in manifest.entries.iter().enumerate() {
            if !safe_relative_path(&entry.path)
                || index.insert(entry.path.clone(), entry_index).is_some()
            {
                return Err(ArchiveError::InvalidPath(entry.path.clone()));
            }
            let end = payload_start
                .checked_add(entry.offset)
                .and_then(|start| start.checked_add(entry.length))
                .ok_or_else(|| ArchiveError::InvalidBounds(entry.path.clone()))?;
            if end > file_length {
                return Err(ArchiveError::InvalidBounds(entry.path.clone()));
            }
        }
        Ok(Self {
            path,
            payload_start,
            entries: manifest.entries,
            index,
        })
    }

    #[must_use]
    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.index.contains_key(path)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads and checksum-validates one archived resource.
    ///
    /// # Errors
    ///
    /// Returns an error when the path is missing, the archive cannot be read,
    /// or the payload does not match the manifest checksum.
    pub fn read(&self, path: &str) -> Result<Vec<u8>, ArchiveError> {
        let entry = self
            .index
            .get(path)
            .and_then(|index| self.entries.get(*index))
            .ok_or_else(|| ArchiveError::Missing(path.to_owned()))?;
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.payload_start + entry.offset))?;
        let length = usize::try_from(entry.length)
            .map_err(|_| ArchiveError::InvalidBounds(entry.path.clone()))?;
        let mut bytes = vec![0; length];
        file.read_exact(&mut bytes)?;
        if format!("{:x}", Sha256::digest(&bytes)) != entry.sha256 {
            return Err(ArchiveError::Checksum(entry.path.clone()));
        }
        Ok(bytes)
    }

    /// Opens a verified resource without allocating its entire encoded payload.
    /// # Errors
    /// Returns an error for missing entries, I/O failures or checksum mismatches.
    pub fn open_reader(
        &self,
        path: &str,
    ) -> Result<crate::resource_reader::ResourceReader, ArchiveError> {
        let entry = self
            .index
            .get(path)
            .map(|index| &self.entries[*index])
            .ok_or_else(|| ArchiveError::Missing(path.to_owned()))?;
        let mut reader = crate::resource_reader::ResourceReader::new(
            File::open(&self.path)?,
            self.payload_start + entry.offset,
            entry.length,
        )?;
        let mut digest = Sha256::new();
        let mut buffer = vec![0; 64 * 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if format!("{:x}", digest.finalize()) != entry.sha256 {
            return Err(ArchiveError::Checksum(path.to_owned()));
        }
        reader.seek(SeekFrom::Start(0))?;
        Ok(reader)
    }

    /// Extracts every entry without overwriting existing files.
    ///
    /// # Errors
    ///
    /// Returns an error for checksum failures, unsafe destinations,
    /// inaccessible storage, or an existing output file.
    pub fn extract(&self, destination: &Path) -> Result<(), ArchiveError> {
        prepare_destination(destination)?;
        for entry in &self.entries {
            let output = destination.join(&entry.path);
            if let Some(parent) = output.parent() {
                prepare_entry_parent(destination, parent)?;
            }
            reject_existing_output(&output)?;
            let bytes = self.read(&entry.path)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)
                .map_err(|error| map_create_error(error, &output))?;
            file.write_all(&bytes)?;
        }
        Ok(())
    }
}

fn prepare_destination(destination: &Path) -> Result<(), ArchiveError> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) => validate_directory(destination, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(destination)?;
            let metadata = fs::symlink_metadata(destination)?;
            validate_directory(destination, &metadata)
        }
        Err(error) => Err(ArchiveError::Io(error)),
    }
}

fn prepare_entry_parent(destination: &Path, parent: &Path) -> Result<(), ArchiveError> {
    let relative = parent
        .strip_prefix(destination)
        .map_err(|_| ArchiveError::InvalidPath(parent.display().to_string()))?;
    let mut directory = destination.to_path_buf();
    for component in relative.components() {
        directory.push(component);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) => validate_directory(&directory, &metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&directory).map_err(|error| map_create_error(error, &directory))?;
                let metadata = fs::symlink_metadata(&directory)?;
                validate_directory(&directory, &metadata)?;
            }
            Err(error) => return Err(ArchiveError::Io(error)),
        }
    }
    Ok(())
}

fn validate_directory(path: &Path, metadata: &fs::Metadata) -> Result<(), ArchiveError> {
    if metadata.file_type().is_symlink() {
        Err(ArchiveError::Symlink(path.display().to_string()))
    } else if metadata.is_dir() {
        Ok(())
    } else {
        Err(ArchiveError::Exists(path.display().to_string()))
    }
}

fn reject_existing_output(output: &Path) -> Result<(), ArchiveError> {
    match fs::symlink_metadata(output) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(ArchiveError::Symlink(output.display().to_string()))
        }
        Ok(_) => Err(ArchiveError::Exists(output.display().to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ArchiveError::Io(error)),
    }
}

fn map_create_error(error: std::io::Error, path: &Path) -> ArchiveError {
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        fs::symlink_metadata(path).map_or_else(
            |_| ArchiveError::Exists(path.display().to_string()),
            |metadata| {
                if metadata.file_type().is_symlink() {
                    ArchiveError::Symlink(path.display().to_string())
                } else {
                    ArchiveError::Exists(path.display().to_string())
                }
            },
        )
    } else {
        ArchiveError::Io(error)
    }
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn replace_file(temporary: &Path, destination: &Path) -> Result<(), std::io::Error> {
    if let Err(error) = fs::rename(temporary, destination) {
        if destination.exists() {
            fs::remove_file(destination)?;
            fs::rename(temporary, destination)?;
        } else {
            return Err(error);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_reads_and_extracts_project_files() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("game");
        fs::create_dir_all(root.join("images")).unwrap();
        fs::write(root.join("script.rns"), b"label start:\n    return\n").unwrap();
        fs::write(root.join("images/pixel.bin"), [1, 2, 3, 4]).unwrap();
        fs::create_dir(root.join(".renrs")).unwrap();
        fs::write(root.join(".renrs/private.json"), b"secret").unwrap();
        let archive_path = temporary.path().join("game.renrs");

        pack_project(&root, &archive_path).unwrap();
        let archive = ResourceArchive::open(&archive_path).unwrap();
        assert_eq!(archive.entries().len(), 2);
        assert!(archive.contains("script.rns"));
        assert!(!archive.contains("missing.rns"));
        assert_eq!(archive.read("images/pixel.bin").unwrap(), [1, 2, 3, 4]);
        assert!(matches!(
            archive.read(".renrs/private.json"),
            Err(ArchiveError::Missing(_))
        ));

        let extracted = temporary.path().join("extracted");
        archive.extract(&extracted).unwrap();
        assert_eq!(
            fs::read(extracted.join("script.rns")).unwrap(),
            b"label start:\n    return\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_extract_through_an_existing_directory_symlink() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("game");
        fs::create_dir_all(root.join("images")).unwrap();
        fs::write(root.join("images/pixel.bin"), [1, 2, 3, 4]).unwrap();
        let archive_path = temporary.path().join("game.renrs");
        pack_project(&root, &archive_path).unwrap();

        let destination = temporary.path().join("extracted");
        let outside = temporary.path().join("outside");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, destination.join("images")).unwrap();

        let archive = ResourceArchive::open(&archive_path).unwrap();
        assert!(matches!(
            archive.extract(&destination),
            Err(ArchiveError::Symlink(_))
        ));
        assert!(!outside.join("pixel.bin").exists());
    }
}
