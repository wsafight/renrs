use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runtime::RuntimeSnapshot;
use std::sync::{Arc, Mutex, MutexGuard};

mod index;
mod summary;
pub mod worker;
pub use renrs_runtime::save_format::{SaveFile, SavePresentation};
use renrs_runtime::save_format::{checksum_matches_encoded, encode_save};

const MAX_ENCODED_SAVE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveMetadata {
    pub project_id: String,
    pub content_version: String,
    pub play_time_seconds: u64,
    pub chapter: Option<String>,
    pub presentation: Option<SavePresentation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSlot {
    pub name: String,
    pub saved_at_unix: u64,
    pub title: String,
    pub project_id: String,
    pub play_time_seconds: u64,
    pub chapter: Option<String>,
    pub corrupt: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub thumbnail_png: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SaveRepository {
    root: PathBuf,
    index: Arc<Mutex<index::SaveIndex>>,
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("invalid save slot `{0}`; use letters, numbers, `_`, or `-`")]
    InvalidSlot(String),
    #[error("save slot `{0}` does not exist")]
    Missing(String),
    #[error("could not access save storage: {0}")]
    Io(#[from] std::io::Error),
    #[error("save file is invalid: {0}")]
    Format(#[from] serde_json::Error),
    #[error("save slot `{0}` failed its SHA-256 integrity check")]
    Checksum(String),
    #[error("save file is too large ({found} bytes; maximum {limit} bytes)")]
    TooLarge { found: u64, limit: u64 },
    #[error("save belongs to another game")]
    ProjectMismatch,
    #[error("unsupported save container version {found}; this engine supports version {current}")]
    UnsupportedContainerVersion { found: u32, current: u32 },
    #[error("rotation count must be between 1 and 99")]
    InvalidRotation,
    #[error("refusing to overwrite exported save `{0}`")]
    ExportExists(String),
    #[error("system clock is before the Unix epoch")]
    Clock,
}

impl SaveRepository {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            index: Arc::default(),
        }
    }

    /// Writes a runtime snapshot to a named slot using a temporary file.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe slot name, inaccessible storage, an
    /// invalid system clock, or snapshot serialization failure.
    pub fn save(&self, slot: &str, snapshot: &RuntimeSnapshot) -> Result<(), SaveError> {
        self.save_with_metadata(slot, snapshot, SaveMetadata::default())
    }

    /// Writes a snapshot and player-facing metadata with an integrity checksum.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe names, inaccessible storage, clock failure,
    /// or serialization failure.
    pub fn save_with_metadata(
        &self,
        slot: &str,
        snapshot: &RuntimeSnapshot,
        metadata: SaveMetadata,
    ) -> Result<(), SaveError> {
        self.save_owned(slot, snapshot.clone(), metadata)
    }

    /// Writes an owned snapshot without duplicating the worker's payload.
    /// # Errors
    /// Returns the same validation and storage errors as `save_with_metadata`.
    pub fn save_owned(
        &self,
        slot: &str,
        snapshot: RuntimeSnapshot,
        metadata: SaveMetadata,
    ) -> Result<(), SaveError> {
        let destination = self.slot_path(slot)?;
        fs::create_dir_all(&self.root)?;
        let saved_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SaveError::Clock)?
            .as_secs();
        let mut save = SaveFile {
            container_version: SaveFile::CONTAINER_VERSION,
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            saved_at_unix,
            project_id: metadata.project_id,
            content_version: metadata.content_version,
            play_time_seconds: metadata.play_time_seconds,
            chapter: metadata.chapter,
            snapshot,
            checksum_sha256: String::new(),
            presentation: metadata.presentation,
        };
        let (hash, bytes) = encode_save(&save)?;
        save.checksum_sha256 = hash;
        crate::storage::atomic_write(&destination, |file| file.write_all(&bytes))?;
        let _ = self.write_summary(slot, &save);
        self.index().invalidate();
        Ok(())
    }

    /// Rotates `prefix-1` through `prefix-N` and saves a new `prefix-1`.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid count or any save/storage failure.
    pub fn save_rotating(
        &self,
        prefix: &str,
        count: usize,
        snapshot: &RuntimeSnapshot,
        metadata: SaveMetadata,
    ) -> Result<(), SaveError> {
        self.save_rotating_owned(prefix, count, snapshot.clone(), metadata)
    }

    /// Rotates slots while transferring snapshot ownership to storage.
    /// # Errors
    /// Returns the same validation and storage errors as `save_rotating`.
    pub fn save_rotating_owned(
        &self,
        prefix: &str,
        count: usize,
        snapshot: RuntimeSnapshot,
        metadata: SaveMetadata,
    ) -> Result<(), SaveError> {
        self.index().invalidate();
        if !(1..=99).contains(&count) {
            return Err(SaveError::InvalidRotation);
        }
        self.slot_path(prefix)?;
        fs::create_dir_all(&self.root)?;
        for index in (1..count).rev() {
            let source_name = format!("{prefix}-{index}");
            let source = self.slot_path(&source_name)?;
            if !source.exists() {
                continue;
            }
            let destination_name = format!("{prefix}-{}", index + 1);
            let destination = self.slot_path(&destination_name)?;
            let summary = self.read_summary(&source_name, &fs::metadata(&source)?);
            crate::storage::atomic_write(&destination, |file| {
                std::io::copy(&mut File::open(&source)?, file).map(|_| ())
            })?;
            if let Some(mut summary) = summary {
                summary.name.clone_from(&destination_name);
                let _ = self.write_slot_summary(&destination_name, summary);
            }
        }
        self.save_owned(&format!("{prefix}-1"), snapshot, metadata)
    }

    /// Reads and decodes a named save slot.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe or missing slot, inaccessible storage, or
    /// malformed JSON.
    pub fn load(&self, slot: &str) -> Result<SaveFile, SaveError> {
        self.load_encoded(slot).map(|(save, _)| save)
    }

    fn load_encoded(&self, slot: &str) -> Result<(SaveFile, Vec<u8>), SaveError> {
        let path = self.slot_path(slot)?;
        let bytes = read_save_bytes(&path).map_err(|error| match error {
            SaveError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
                SaveError::Missing(slot.to_owned())
            }
            error => error,
        })?;
        let save = decode_save(&bytes, slot)?;
        Ok((save, bytes))
    }

    /// Lists slots using persistent summaries, checking changed files on demand.
    ///
    /// # Errors
    ///
    /// Returns an error when the save directory cannot be inspected. Individual
    /// malformed save files are returned as corrupt slots so players can see
    /// and replace them without hiding valid saves.
    pub fn list(&self) -> Result<Vec<SaveSlot>, SaveError> {
        self.list_indexed(true)
    }

    fn slot_path(&self, slot: &str) -> Result<PathBuf, SaveError> {
        if slot.is_empty()
            || !slot
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(SaveError::InvalidSlot(slot.to_owned()));
        }
        Ok(self.root.join(format!("{slot}.json")))
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Copies a save container to an external path without overwriting it.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid slot, missing/corrupt source, an
    /// existing destination, or filesystem failure.
    pub fn export(&self, slot: &str, destination: &Path) -> Result<(), SaveError> {
        let (_, bytes) = self.load_encoded(slot)?;
        if destination.exists() {
            return Err(SaveError::ExportExists(destination.display().to_string()));
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        output.write_all(&bytes)?;
        output.flush()?;
        output.sync_all()?;
        Ok(())
    }

    /// Imports a checksum-valid save container into a named local slot.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed input, an invalid slot, or storage
    /// failure. The runtime verifies the script fingerprint before restoring.
    pub fn import(&self, source: &Path, slot: &str) -> Result<(), SaveError> {
        self.import_checked(source, slot, None)
    }

    /// Imports a save after verifying that it belongs to the expected project.
    ///
    /// # Errors
    /// Returns the same errors as `import`, plus a project mismatch error.
    pub fn import_for_project(
        &self,
        source: &Path,
        slot: &str,
        project_id: &str,
    ) -> Result<(), SaveError> {
        self.import_checked(source, slot, Some(project_id))
    }

    fn import_checked(
        &self,
        source: &Path,
        slot: &str,
        project_id: Option<&str>,
    ) -> Result<(), SaveError> {
        let destination = self.slot_path(slot)?;
        let bytes = read_save_bytes(source)?;
        let save = decode_save(&bytes, slot)?;
        if project_id
            .is_some_and(|expected| !save.project_id.is_empty() && save.project_id != expected)
        {
            return Err(SaveError::ProjectMismatch);
        }
        fs::create_dir_all(&self.root)?;
        crate::storage::atomic_write(&destination, |file| file.write_all(&bytes))?;
        let _ = self.write_summary(slot, &save);
        self.index().invalidate();
        Ok(())
    }

    fn index(&self) -> MutexGuard<'_, index::SaveIndex> {
        self.index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn read_save_bytes(path: &Path) -> Result<Vec<u8>, SaveError> {
    let file = File::open(path)?;
    let length = file.metadata()?.len();
    let limit = usize::try_from(MAX_ENCODED_SAVE_BYTES).map_err(|_| SaveError::TooLarge {
        found: length,
        limit: MAX_ENCODED_SAVE_BYTES,
    })?;
    if length > MAX_ENCODED_SAVE_BYTES {
        return Err(SaveError::TooLarge {
            found: length,
            limit: MAX_ENCODED_SAVE_BYTES,
        });
    }
    let capacity = usize::try_from(length).map_err(|_| SaveError::TooLarge {
        found: length,
        limit: MAX_ENCODED_SAVE_BYTES,
    })?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(MAX_ENCODED_SAVE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(SaveError::TooLarge {
            found: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            limit: MAX_ENCODED_SAVE_BYTES,
        });
    }
    Ok(bytes)
}

fn decode_save(bytes: &[u8], slot: &str) -> Result<SaveFile, SaveError> {
    let save: SaveFile = serde_json::from_slice(bytes)?;
    validate_container_version(save.container_version)?;
    if !checksum_matches_encoded(&save, bytes)? {
        return Err(SaveError::Checksum(slot.to_owned()));
    }
    Ok(save)
}

fn validate_container_version(version: u32) -> Result<(), SaveError> {
    if matches!(version, 2 | SaveFile::CONTAINER_VERSION) {
        Ok(())
    } else {
        Err(SaveError::UnsupportedContainerVersion {
            found: version,
            current: SaveFile::CONTAINER_VERSION,
        })
    }
}

#[cfg(test)]
#[path = "save/tests.rs"]
mod tests;
