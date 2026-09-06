use super::{SaveError, SaveFile, SaveRepository, SaveSlot};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize)]
struct Summary {
    length: u64,
    modified_ns: u128,
    slot: SaveSlot,
}

impl SaveRepository {
    pub(super) fn write_summary(&self, name: &str, save: &SaveFile) -> Result<(), SaveError> {
        self.write_slot_summary(
            name,
            SaveSlot {
                name: name.to_owned(),
                saved_at_unix: save.saved_at_unix,
                title: save
                    .snapshot
                    .stage
                    .dialogue
                    .as_ref()
                    .map_or_else(|| "No dialogue".to_owned(), |line| line.text.clone()),
                project_id: save.project_id.clone(),
                play_time_seconds: save.play_time_seconds,
                chapter: save.chapter.clone(),
                corrupt: false,
                note: save
                    .presentation
                    .as_ref()
                    .map_or_else(String::new, |view| view.note.clone()),
                thumbnail_png: save
                    .presentation
                    .as_ref()
                    .map_or_else(Vec::new, |view| view.thumbnail_png.clone()),
            },
        )
    }

    pub(super) fn write_slot_summary(&self, name: &str, slot: SaveSlot) -> Result<(), SaveError> {
        let metadata = fs::metadata(self.slot_path(name)?)?;
        let summary = Summary {
            length: metadata.len(),
            modified_ns: metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(|_| SaveError::Clock)?
                .as_nanos(),
            slot,
        };
        crate::storage::write_json(&self.root.join(format!(".{name}.summary")), &summary)?;
        Ok(())
    }

    pub(super) fn read_summary(&self, name: &str, metadata: &fs::Metadata) -> Option<SaveSlot> {
        let file = fs::File::open(self.root.join(format!(".{name}.summary"))).ok()?;
        let summary: Summary = serde_json::from_reader(std::io::BufReader::new(file)).ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        (summary.length == metadata.len()
            && summary.modified_ns == modified
            && summary.slot.name == name)
            .then_some(summary.slot)
    }

    /// Deletes a slot and its optional cached summary.
    ///
    /// # Errors
    /// Returns invalid-slot or filesystem errors.
    pub fn delete(&self, name: &str) -> Result<(), SaveError> {
        let mut index = self.index();
        fs::remove_file(self.slot_path(name)?)?;
        index.invalidate();
        let _ = fs::remove_file(self.root.join(format!(".{name}.summary")));
        Ok(())
    }
}
