use crate::runtime::RuntimeSnapshot;
use crate::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const LEGACY_CONTAINER_VERSION: u32 = 2;
const EMPTY_CHECKSUM: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const CHECKSUM_KEY: &[u8] = b"\"checksum_sha256\"";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub container_version: u32,
    pub engine_version: String,
    pub saved_at_unix: u64,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub content_version: String,
    #[serde(default)]
    pub play_time_seconds: u64,
    #[serde(default)]
    pub chapter: Option<String>,
    pub snapshot: RuntimeSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<SavePresentation>,
    pub checksum_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavePresentation {
    pub sprite_elapsed_ms: u64,
    pub dialogue_page: usize,
    pub visible_characters: usize,
    pub pause_remaining_ms: u32,
    pub effect_remaining_ms: u32,
    pub note: String,
    pub thumbnail_png: Vec<u8>,
}

#[derive(Serialize)]
struct ChecksumPayload<'a> {
    container_version: u32,
    engine_version: &'a str,
    saved_at_unix: u64,
    project_id: &'a str,
    content_version: &'a str,
    play_time_seconds: u64,
    chapter: &'a Option<String>,
    snapshot: &'a Snapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    presentation: &'a Option<SavePresentation>,
}

/// Hashes the canonical Rust representation on native and WASM platforms.
/// # Errors
/// Returns a serialization error for an invalid payload.
pub fn checksum(save: &SaveFile) -> Result<String, serde_json::Error> {
    let interned = Snapshot::from(save.snapshot.clone());
    if save.container_version == LEGACY_CONTAINER_VERSION {
        checksum_legacy(save, &interned)
    } else {
        encode_current(save, &interned).map(|(hash, _)| hash)
    }
}

fn checksum_legacy(save: &SaveFile, snapshot: &Snapshot) -> Result<String, serde_json::Error> {
    let payload = ChecksumPayload {
        container_version: save.container_version,
        engine_version: &save.engine_version,
        saved_at_unix: save.saved_at_unix,
        project_id: &save.project_id,
        content_version: &save.content_version,
        play_time_seconds: save.play_time_seconds,
        chapter: &save.chapter,
        snapshot,
        presentation: &save.presentation,
    };
    let bytes = serde_json::to_vec(&payload)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

#[derive(Serialize)]
struct EncodedSaveFile<'a> {
    container_version: u32,
    engine_version: &'a str,
    saved_at_unix: u64,
    project_id: &'a str,
    content_version: &'a str,
    play_time_seconds: u64,
    chapter: &'a Option<String>,
    snapshot: &'a Snapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    presentation: &'a Option<SavePresentation>,
    checksum_sha256: &'a str,
}

fn encoded<'a>(
    save: &'a SaveFile,
    snapshot: &'a Snapshot,
    checksum: &'a str,
) -> EncodedSaveFile<'a> {
    EncodedSaveFile {
        container_version: save.container_version,
        engine_version: &save.engine_version,
        saved_at_unix: save.saved_at_unix,
        project_id: &save.project_id,
        content_version: &save.content_version,
        play_time_seconds: save.play_time_seconds,
        chapter: &save.chapter,
        snapshot,
        presentation: &save.presentation,
        checksum_sha256: checksum,
    }
}

fn encode_current(
    save: &SaveFile,
    snapshot: &Snapshot,
) -> Result<(String, Vec<u8>), serde_json::Error> {
    let mut bytes = serde_json::to_vec(&encoded(save, snapshot, EMPTY_CHECKSUM))?;
    let hash = hex::encode(Sha256::digest(&bytes));
    let range = checksum_value_range(&bytes)
        .expect("the encoded save always contains its fixed-width checksum");
    bytes[range].copy_from_slice(hash.as_bytes());
    bytes.push(b'\n');
    Ok((hash, bytes))
}

/// Interns the snapshot once, then hashes and encodes the save container.
/// # Errors
/// Returns a serialization error for an invalid payload.
pub fn encode_save(save: &SaveFile) -> Result<(String, Vec<u8>), serde_json::Error> {
    let interned = Snapshot::from(save.snapshot.clone());
    if save.container_version == LEGACY_CONTAINER_VERSION {
        let hash = checksum_legacy(save, &interned)?;
        let mut bytes = serde_json::to_vec(&encoded(save, &interned, &hash))?;
        bytes.push(b'\n');
        Ok((hash, bytes))
    } else {
        encode_current(save, &interned)
    }
}

/// Verifies a decoded save against its original encoded bytes when possible.
/// Version 2 containers retain their canonical semantic checksum, while version
/// 3 hashes the encoded container directly without reserializing its snapshot.
/// # Errors
/// Returns a serialization error while checking a legacy container.
pub fn checksum_matches_encoded(save: &SaveFile, bytes: &[u8]) -> Result<bool, serde_json::Error> {
    if save.container_version == LEGACY_CONTAINER_VERSION {
        return Ok(checksum(save)? == save.checksum_sha256);
    }
    if save.container_version != SaveFile::CONTAINER_VERSION {
        return Ok(false);
    }
    let Some(range) = checksum_value_range(bytes) else {
        return Ok(false);
    };
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(0, |index| index + 1);
    if end < range.end || !bytes[range.clone()].iter().all(u8::is_ascii_hexdigit) {
        return Ok(false);
    }
    let mut digest = Sha256::new();
    digest.update(&bytes[..range.start]);
    digest.update(EMPTY_CHECKSUM.as_bytes());
    digest.update(&bytes[range.end..end]);
    Ok(hex::encode(digest.finalize()) == save.checksum_sha256)
}

fn checksum_value_range(bytes: &[u8]) -> Option<std::ops::Range<usize>> {
    let key = bytes
        .windows(CHECKSUM_KEY.len())
        .rposition(|candidate| candidate == CHECKSUM_KEY)?;
    let mut cursor = key + CHECKSUM_KEY.len();
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b':') {
        return None;
    }
    cursor += 1;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'\"') {
        return None;
    }
    let start = cursor + 1;
    let end = start.checked_add(EMPTY_CHECKSUM.len())?;
    (bytes.get(end) == Some(&b'\"')).then_some(start..end)
}

impl SaveFile {
    pub const CONTAINER_VERSION: u32 = 3;

    fn validate_metadata(&self, project: &str) -> Result<(), String> {
        if !matches!(
            self.container_version,
            LEGACY_CONTAINER_VERSION | Self::CONTAINER_VERSION
        ) {
            return Err(format!(
                "unsupported save container version {}",
                self.container_version
            ));
        }
        if !self.project_id.is_empty() && self.project_id != project {
            return Err("Save belongs to another game".to_owned());
        }
        Ok(())
    }

    /// Checks the container before offering it for import or restore.
    /// # Errors
    /// Rejects unknown versions, another project, and altered payloads.
    pub fn validate(&self, project: &str) -> Result<(), String> {
        self.validate_metadata(project)?;
        if checksum(self).map_err(|error| error.to_string())? != self.checksum_sha256 {
            return Err("Save checksum mismatch".to_owned());
        }
        Ok(())
    }

    /// Checks metadata and hashes an encoded container without reserializing it.
    /// # Errors
    /// Rejects unknown versions, another project, altered bytes, and legacy
    /// payloads that cannot be serialized for their canonical checksum.
    pub fn validate_encoded(&self, project: &str, encoded: &[u8]) -> Result<(), String> {
        self.validate_metadata(project)?;
        if !checksum_matches_encoded(self, encoded).map_err(|error| error.to_string())? {
            return Err("Save checksum mismatch".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Runtime, compile, parse_script};

    fn sample() -> SaveFile {
        let program =
            compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap();
        let mut runtime = Runtime::new(program).unwrap();
        runtime.advance().unwrap();
        let mut save = SaveFile {
            container_version: SaveFile::CONTAINER_VERSION,
            engine_version: "test".to_owned(),
            saved_at_unix: 1,
            project_id: "org.test".to_owned(),
            content_version: String::new(),
            play_time_seconds: 12,
            chapter: Some("start".to_owned()),
            snapshot: runtime.snapshot(),
            presentation: Some(SavePresentation {
                note: "desk".to_owned(),
                ..SavePresentation::default()
            }),
            checksum_sha256: String::new(),
        };
        save.checksum_sha256 = checksum(&save).unwrap();
        save
    }

    #[test]
    fn checksum_covers_presentation_and_rejects_other_games() {
        let save = sample();
        save.validate("org.test").unwrap();
        assert_eq!(
            save.validate("org.other").unwrap_err(),
            "Save belongs to another game"
        );

        let mut other_version = save.clone();
        other_version.container_version = 1;
        assert!(
            other_version
                .validate("org.test")
                .unwrap_err()
                .contains("unsupported save container")
        );

        let mut tampered = save;
        tampered.presentation.as_mut().unwrap().note = "changed".to_owned();
        assert_eq!(
            tampered.validate("org.test").unwrap_err(),
            "Save checksum mismatch"
        );
    }

    #[test]
    fn encode_save_hash_matches_checksum_and_round_trips() {
        let save = sample();
        let (hash, bytes) = encode_save(&save).unwrap();
        assert_eq!(hash, checksum(&save).unwrap());
        let decoded: SaveFile = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.checksum_sha256, hash);
        assert_eq!(checksum(&decoded).unwrap(), hash);
        assert!(checksum_matches_encoded(&decoded, &bytes).unwrap());
        decoded.validate_encoded("org.test", &bytes).unwrap();
        decoded.validate("org.test").unwrap();
    }

    #[test]
    fn encoded_checksum_detects_raw_container_changes() {
        let save = sample();
        let (_, mut bytes) = encode_save(&save).unwrap();
        let decoded: SaveFile = serde_json::from_slice(&bytes).unwrap();
        let index = bytes.windows(4).position(|value| value == b"desk").unwrap();
        bytes[index] = b'D';
        assert!(!checksum_matches_encoded(&decoded, &bytes).unwrap());
    }

    #[test]
    fn version_two_checksum_remains_supported() {
        let mut save = sample();
        save.container_version = LEGACY_CONTAINER_VERSION;
        save.checksum_sha256 = checksum(&save).unwrap();
        let (_, bytes) = encode_save(&save).unwrap();
        let decoded: SaveFile = serde_json::from_slice(&bytes).unwrap();
        assert!(checksum_matches_encoded(&decoded, &bytes).unwrap());
        decoded.validate("org.test").unwrap();
    }
}
