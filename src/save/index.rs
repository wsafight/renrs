use super::{SaveError, SaveRepository, SaveSlot};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub(super) struct SaveIndex {
    scanned_at: Option<Instant>,
    entries: HashMap<String, (u64, Option<SystemTime>, SaveSlot)>,
    slots: Vec<SaveSlot>,
}

impl SaveIndex {
    pub(super) fn invalidate(&mut self) {
        self.scanned_at = None;
        self.entries.clear();
    }
}

impl SaveRepository {
    /// Lists slots without reading storage on each UI frame. External changes
    /// become visible within one second; writes through this repository invalidate immediately.
    ///
    /// # Errors
    /// Returns a storage error if the directory or file metadata cannot be read.
    pub fn list_cached(&self) -> Result<Vec<SaveSlot>, SaveError> {
        self.list_indexed(false)
    }

    pub(super) fn list_indexed(&self, refresh: bool) -> Result<Vec<SaveSlot>, SaveError> {
        let mut index = self.index();
        if !refresh
            && index
                .scanned_at
                .is_some_and(|time| time.elapsed() < Duration::from_secs(1))
        {
            return Ok(index.slots.clone());
        }
        let mut entries = HashMap::new();
        if self.root.exists() {
            for entry in std::fs::read_dir(&self.root)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let Some(name) = path.file_stem().and_then(|value| value.to_str()) else {
                    continue;
                };
                let metadata = entry.metadata()?;
                let modified = metadata.modified().ok();
                let slot = if let Some((length, timestamp, slot)) = index.entries.get(name)
                    && *length == metadata.len()
                    && *timestamp == modified
                    && modified.is_some()
                {
                    slot.clone()
                } else if let Some(summary) = self.read_summary(name, &metadata) {
                    summary
                } else {
                    self.load(name).map_or_else(
                        |_| SaveSlot {
                            name: name.to_owned(),
                            saved_at_unix: modified
                                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                                .map_or(0, |time| time.as_secs()),
                            title: "Corrupt save".to_owned(),
                            project_id: String::new(),
                            play_time_seconds: 0,
                            chapter: None,
                            corrupt: true,
                            note: String::new(),
                            thumbnail_png: Vec::new(),
                        },
                        |save| {
                            let _ = self.write_summary(name, &save);
                            SaveSlot {
                                name: name.to_owned(),
                                saved_at_unix: save.saved_at_unix,
                                title: save.snapshot.stage.dialogue.as_ref().map_or_else(
                                    || "No dialogue".to_owned(),
                                    |dialogue| dialogue.text.clone(),
                                ),
                                project_id: save.project_id,
                                play_time_seconds: save.play_time_seconds,
                                chapter: save.chapter,
                                corrupt: false,
                                note: save
                                    .presentation
                                    .as_ref()
                                    .map_or_else(String::new, |view| view.note.clone()),
                                thumbnail_png: save
                                    .presentation
                                    .map_or_else(Vec::new, |view| view.thumbnail_png),
                            }
                        },
                    )
                };
                entries.insert(name.to_owned(), (metadata.len(), modified, slot));
            }
        }
        index.slots = entries.values().map(|(_, _, slot)| slot.clone()).collect();
        index.slots.sort_by(|left, right| {
            right
                .saved_at_unix
                .cmp(&left.saved_at_unix)
                .then(left.name.cmp(&right.name))
        });
        index.entries = entries;
        index.scanned_at = Some(Instant::now());
        Ok(index.slots.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Runtime, compile, parse_script};

    #[test]
    fn cached_listing_updates_on_writes_and_external_deletion() {
        fn assert_send_sync<T: Send + Sync>(_: &T) {}
        let root = tempfile::tempdir().unwrap();
        let repository = SaveRepository::new(root.path());
        assert_send_sync(&repository);
        let cloned = repository.clone();
        assert!(repository.list_cached().unwrap().is_empty());
        let mut runtime = Runtime::new(
            compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap(),
        )
        .unwrap();
        runtime.advance().unwrap();
        repository.save("slot-1", &runtime.snapshot()).unwrap();
        assert_eq!(repository.list_cached().unwrap().len(), 1);
        assert_eq!(cloned.list_cached().unwrap().len(), 1);
        std::fs::remove_file(root.path().join("slot-1.json")).unwrap();
        assert_eq!(repository.list_cached().unwrap().len(), 1);
        repository.index().scanned_at = None;
        assert!(repository.list_cached().unwrap().is_empty());
    }
}
