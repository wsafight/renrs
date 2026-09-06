use super::{Engine, js_error};
use renrs_runtime::save_format::{SaveFile, SavePresentation, checksum};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Engine {
    pub fn import_save(&self, text: &str) -> Result<String, JsValue> {
        let save: SaveFile = serde_json::from_str(text).map_err(js_error)?;
        save.validate(&self.runtime.shared_program().project_id)
            .map_err(js_error)?;
        let snapshot = serde_json::to_string(&save.snapshot).map_err(js_error)?;
        // Validate current-build state before the browser writes imported progress.
        renrs_runtime::Runtime::restore(self.runtime.shared_program(), save.snapshot.clone())
            .map_err(js_error)?;
        let view = save.presentation.unwrap_or_default();
        serde_json::to_string(&serde_json::json!({
            "version": 1, "project": self.runtime.shared_program().project_id,
            "time": save.saved_at_unix.saturating_mul(1000), "chapter": save.chapter,
            "text": save.snapshot.stage.dialogue.as_ref().map(|dialogue| &dialogue.text),
            "snapshot": snapshot, "remaining": view.effect_remaining_ms.max(view.pause_remaining_ms),
            "note": view.note, "thumbnail": save.snapshot.stage.background,
            "presentation": {"dialogue_page": view.dialogue_page, "visible_characters": view.visible_characters, "sprite_elapsed_ms": view.sprite_elapsed_ms},
            "play_time_seconds": save.play_time_seconds
        })).map_err(js_error)
    }

    pub fn export_save(&self, snapshot: &str, metadata: &str) -> Result<String, JsValue> {
        let metadata: serde_json::Value = serde_json::from_str(metadata).map_err(js_error)?;
        let mut save = SaveFile {
            container_version: SaveFile::CONTAINER_VERSION,
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            saved_at_unix: metadata["time"].as_u64().unwrap_or_default() / 1000,
            project_id: self.runtime.shared_program().project_id.clone(),
            content_version: self.runtime.shared_program().fingerprint.clone(),
            play_time_seconds: metadata["play_time_seconds"].as_u64().unwrap_or_default(),
            chapter: metadata["chapter"].as_str().map(str::to_owned),
            snapshot: serde_json::from_str(snapshot).map_err(js_error)?,
            checksum_sha256: String::new(),
            presentation: Some(SavePresentation {
                sprite_elapsed_ms: metadata["presentation"]["sprite_elapsed_ms"]
                    .as_u64()
                    .unwrap_or_default(),
                dialogue_page: metadata["presentation"]["dialogue_page"]
                    .as_u64()
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or_default(),
                visible_characters: metadata["presentation"]["visible_characters"]
                    .as_u64()
                    .unwrap_or(u64::from(u32::MAX))
                    .try_into()
                    .unwrap_or(usize::MAX),
                pause_remaining_ms: remaining(&metadata),
                effect_remaining_ms: remaining(&metadata),
                note: metadata["note"].as_str().unwrap_or("").to_owned(),
                thumbnail_png: Vec::new(),
            }),
        };
        renrs_runtime::Runtime::restore(self.runtime.shared_program(), save.snapshot.clone())
            .map_err(js_error)?;
        save.checksum_sha256 = checksum(&save).map_err(js_error)?;
        serde_json::to_string(&save).map_err(js_error)
    }
}

fn remaining(metadata: &serde_json::Value) -> u32 {
    metadata["remaining"]
        .as_f64()
        .unwrap_or(0.0)
        .clamp(0.0, f64::from(u32::MAX))
        .round() as u32
}
