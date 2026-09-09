use crate::runtime::RuntimeSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    pub checksum_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<SavePresentation>,
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
    snapshot: &'a RuntimeSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    presentation: &'a Option<SavePresentation>,
}

/// Hashes the canonical Rust representation on native and WASM platforms.
/// # Errors
/// Returns a serialization error for an invalid payload.
pub fn checksum(save: &SaveFile) -> Result<String, serde_json::Error> {
    struct HashWriter(Sha256);
    impl std::io::Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let payload = ChecksumPayload {
        container_version: save.container_version,
        engine_version: &save.engine_version,
        saved_at_unix: save.saved_at_unix,
        project_id: &save.project_id,
        content_version: &save.content_version,
        play_time_seconds: save.play_time_seconds,
        chapter: &save.chapter,
        snapshot: &save.snapshot,
        presentation: &save.presentation,
    };
    let mut writer = std::io::BufWriter::with_capacity(64 * 1024, HashWriter(Sha256::new()));
    serde_json::to_writer(&mut writer, &payload)?;
    let hash = writer
        .into_inner()
        .map_err(|error| serde_json::Error::io(error.into_error()))?;
    Ok(format!("{:x}", hash.0.finalize()))
}

impl SaveFile {
    pub const CONTAINER_VERSION: u32 = 2;

    /// Checks the container before offering it for import or restore.
    /// # Errors
    /// Rejects unknown versions, another project, and altered payloads.
    pub fn validate(&self, project: &str) -> Result<(), String> {
        if self.container_version != Self::CONTAINER_VERSION {
            return Err(format!(
                "unsupported save container version {}",
                self.container_version
            ));
        }
        if !self.project_id.is_empty() && self.project_id != project {
            return Err("Save belongs to another game".to_owned());
        }
        if checksum(self).map_err(|error| error.to_string())? != self.checksum_sha256 {
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
            checksum_sha256: String::new(),
            presentation: Some(SavePresentation {
                note: "desk".to_owned(),
                ..SavePresentation::default()
            }),
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
}
