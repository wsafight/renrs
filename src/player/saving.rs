use super::app::{App, Overlay, Screen};
use macroquad::prelude::*;
use renrs::Runtime;
use renrs::save::worker::{SaveRequest, SaveResponse, SaveWorker};
use renrs::save::{SaveFile, SaveMetadata, SavePresentation, SaveRepository, SaveSlot};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) enum Confirmation {
    Save(String),
    Load(String),
    Delete(String),
    Quit,
}

#[allow(clippy::struct_excessive_bools)]
pub(super) struct PlayerStorage {
    pub(super) worker: SaveWorker,
    pub(super) slots: Vec<SaveSlot>,
    pub(super) screen_lists: std::collections::HashMap<
        (String, bool),
        std::sync::Arc<[super::ui_screen_lists::ListItem]>,
    >,
    pub(super) loading: bool,
    pub(super) confirmation: Option<Confirmation>,
    pub(super) tools_slot: String,
    pub(super) note: String,
    pub(super) file_path: String,
    pub(super) page: usize,
    pub(super) progress_dirty: bool,
    pub(super) chapter: Option<String>,
    pub(super) quiet: bool,
    pub(super) quit_after_save: bool,
    pub(super) thumbnails: std::collections::HashMap<String, Texture2D>,
    pub(super) accessibility_focus: usize,
    pub(super) profile: renrs::progress::Profile,
    pub(super) profile_path: std::path::PathBuf,
    pub(super) profile_saved: Option<renrs::progress::Profile>,
    pub(super) profile_revision: Option<u64>,
    last_scan: Instant,
    last_auto: Instant,
    last_read: Instant,
}

impl PlayerStorage {
    pub(super) fn new(repository: SaveRepository) -> Self {
        let mut worker = SaveWorker::new(repository);
        let _ = worker.submit(SaveRequest::List);
        Self {
            worker,
            slots: Vec::new(),
            screen_lists: std::collections::HashMap::new(),
            loading: false,
            confirmation: None,
            tools_slot: "slot-1".to_owned(),
            note: String::new(),
            file_path: String::new(),
            page: 0,
            progress_dirty: false,
            chapter: None,
            quiet: false,
            quit_after_save: false,
            thumbnails: std::collections::HashMap::new(),
            accessibility_focus: 0,
            profile: renrs::progress::Profile::default(),
            profile_path: std::path::PathBuf::new(),
            profile_saved: None,
            profile_revision: None,
            last_scan: Instant::now(),
            last_auto: Instant::now(),
            last_read: Instant::now(),
        }
    }
}

impl App {
    pub(super) fn update_storage(&mut self) {
        let mut refresh = false;
        while let Some(response) = self.storage.worker.poll() {
            self.redraw |= match &response {
                SaveResponse::Listed(slots) => slots != &self.storage.slots,
                SaveResponse::Completed(message) => !message.is_empty(),
                _ => true,
            };
            match response {
                SaveResponse::Listed(slots) => {
                    self.update_slot_previews(slots);
                }
                SaveResponse::Loaded(slot, save) => {
                    self.storage.loading = false;
                    if let Err(error) = self.apply_save(*save) {
                        self.notice = Some((error, 6.0));
                    } else if !self.storage.quiet {
                        self.notice = Some((format!("Loaded {slot}"), 2.0));
                    }
                }
                SaveResponse::Saved(slot) => {
                    refresh = true;
                    if slot == "resume" && self.storage.quit_after_save {
                        self.quit = true;
                    }
                    if !self.storage.quiet && slot != "auto" && slot != "resume" {
                        self.notice = Some((format!("Saved {slot}"), 2.0));
                    }
                }
                SaveResponse::Completed(message) => {
                    refresh |= !message.is_empty();
                    if !message.is_empty() && !self.storage.quiet {
                        self.notice = Some((message, 2.0));
                    }
                }
                SaveResponse::Error(error) => {
                    self.storage.profile_saved = None;
                    self.storage.progress_dirty = self.runtime.is_some();
                    self.read_dirty = true;
                    self.storage.loading = false;
                    self.storage.quit_after_save = false;
                    self.notice = Some((error, 8.0));
                }
            }
        }
        if !self.storage.worker.busy()
            && (refresh || self.storage.last_scan.elapsed() > Duration::from_secs(1))
        {
            let _ = self.storage.worker.submit(SaveRequest::List);
            self.storage.last_scan = Instant::now();
        }
        if !self.storage.loading
            && !self.storage.worker.busy()
            && self.storage.progress_dirty
            && self.storage.last_auto.elapsed() >= Duration::from_secs(30)
        {
            self.auto_save();
        }
        if self.read_dirty
            && self.storage.last_read.elapsed() >= Duration::from_secs(5)
            && let Ok(bytes) = serde_json::to_vec(&self.read_state)
            && self
                .storage
                .worker
                .submit(SaveRequest::Persist {
                    path: self.read_path.clone(),
                    bytes,
                })
                .is_ok()
        {
            self.read_dirty = false;
            self.storage.last_read = Instant::now();
        }
        let mut profile_changed = false;
        if let Some(runtime) = &self.runtime
            && self.storage.profile_revision != Some(runtime.profile_revision())
        {
            self.storage.profile = runtime.profile().clone();
            self.storage.profile_revision = Some(runtime.profile_revision());
            profile_changed = true;
        }
        if (profile_changed || self.storage.profile_saved.is_none())
            && !self.storage.profile_path.as_os_str().is_empty()
            && let Ok(bytes) = serde_json::to_vec(&self.storage.profile)
            && self
                .storage
                .worker
                .submit(SaveRequest::Persist {
                    path: self.storage.profile_path.clone(),
                    bytes,
                })
                .is_ok()
        {
            self.storage.profile_saved = Some(self.storage.profile.clone());
        }
    }

    fn update_slot_previews(&mut self, slots: Vec<SaveSlot>) {
        if slots != self.storage.slots {
            self.storage.screen_lists.clear();
        }
        self.storage
            .thumbnails
            .retain(|name, _| slots.iter().any(|slot| &slot.name == name));
        for slot in &slots {
            let changed = self
                .storage
                .slots
                .iter()
                .find(|old| old.name == slot.name)
                .is_none_or(|old| old.thumbnail_png != slot.thumbnail_png);
            if changed {
                self.storage.thumbnails.remove(&slot.name);
            }
        }
        self.storage.slots = slots;
    }

    pub(super) fn prepare_slot_previews(&mut self, prefix: &str, start: usize, count: usize) {
        let names: Vec<_> = (1..=count)
            .map(|index| format!("{prefix}-{}", start + index))
            .collect();
        self.storage
            .thumbnails
            .retain(|name, _| names.contains(name));
        for slot in &self.storage.slots {
            if names.contains(&slot.name)
                && !self.storage.thumbnails.contains_key(&slot.name)
                && let Ok(image) =
                    Image::from_file_with_format(&slot.thumbnail_png, Some(ImageFormat::Png))
            {
                self.storage
                    .thumbnails
                    .insert(slot.name.clone(), Texture2D::from_image(&image));
                self.redraw = true;
                break;
            }
        }
    }

    pub(super) fn save_slot(&mut self, slot: &str) {
        if self
            .storage
            .slots
            .iter()
            .any(|existing| existing.name == slot)
            && !self.storage.quiet
        {
            self.storage.confirmation = Some(Confirmation::Save(slot.to_owned()));
            self.overlay = Some(Overlay::Confirm);
        } else {
            self.queue_save(slot, None);
        }
    }

    pub(super) fn queue_save(&mut self, slot: &str, rotation: Option<usize>) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let metadata = self.save_metadata();
        let thumbnail = (slot != "auto").then(|| self.capture_save_thumbnail());
        let request = SaveRequest::Save {
            slot: slot.to_owned(),
            rotation,
            snapshot: Box::new(runtime.snapshot()),
            metadata,
            thumbnail,
        };
        match self.storage.worker.submit(request) {
            Ok(()) => {
                self.storage.progress_dirty = false;
                if rotation.is_none() && slot != "resume" {
                    self.overlay = None;
                }
            }
            Err(error) => {
                self.notice = Some((error, 5.0));
                self.storage.quit_after_save = false;
            }
        }
    }

    pub(super) fn load_slot(&mut self, slot: &str) {
        if self.runtime.is_some() && !self.storage.quiet {
            self.storage.confirmation = Some(Confirmation::Load(slot.to_owned()));
            self.overlay = Some(Overlay::Confirm);
        } else {
            self.queue_load(slot);
        }
    }

    pub(super) fn queue_load(&mut self, slot: &str) {
        if self.storage.loading {
            return;
        }
        match self
            .storage
            .worker
            .submit(SaveRequest::Load(slot.to_owned()))
        {
            Ok(()) => {
                self.storage.loading = true;
                self.playback.auto = false;
                self.playback.skip_read = false;
            }
            Err(error) => self.notice = Some((error, 5.0)),
        }
    }

    fn apply_save(&mut self, save: SaveFile) -> Result<(), String> {
        if !save.project_id.is_empty() && save.project_id != self.program.project_id {
            return Err("Save belongs to another game".to_owned());
        }
        let (mut runtime, _) = Runtime::restore_compatible(self.program.clone(), save.snapshot)
            .map_err(|error| error.to_string())?;
        runtime
            .set_profile(self.storage.profile.clone())
            .map_err(|error| error.to_string())?;
        runtime
            .set_localizer(self.localizer.clone())
            .map_err(|error| error.to_string())?;
        let view = save.presentation.unwrap_or_else(|| {
            let mut view = SavePresentation::default();
            match runtime.waiting() {
                Some(renrs::WaitState::Pause { seconds }) => {
                    view.pause_remaining_ms = (seconds * 1000.0) as u32;
                }
                Some(renrs::WaitState::Effect { effect }) => {
                    view.effect_remaining_ms = (effect.seconds() * 1000.0) as u32;
                }
                _ => {}
            }
            view
        });
        self.history_view.invalidate();
        self.dialogue_view.invalidate();
        self.dialogue_cue_remaining = None;
        self.dialogue_view.prepare_reading(&runtime);
        if let Some(dialogue) = &runtime.stage().dialogue {
            let nvl = runtime.nvl_dialogue();
            let mut theme = self.dialogue_theme();
            if runtime.stage().nvl {
                theme.layout.dialogue_rect = renrs::theme::ThemeRect {
                    x: 80.0,
                    y: 80.0,
                    width: 1120.0,
                    height: 580.0,
                };
            }
            self.dialogue_view
                .prepare(nvl.as_ref().unwrap_or(dialogue), &theme);
            self.dialogue_view.restore_page(view.dialogue_page);
        }
        self.visible_characters = view.visible_characters as f32;
        self.pause_remaining = view.pause_remaining_ms as f32 / 1000.0;
        self.effect_remaining = view.effect_remaining_ms as f32 / 1000.0;
        self.selected_choice = 0;
        self.auto_remaining = self.settings.auto_delay;
        self.skip_remaining = 0.08;
        self.audio.stop_music();
        self.audio.stop_voice();
        self.audio.stop_video();
        self.play_time_seconds =
            (view.sprite_elapsed_ms as f64 / 1000.0).max(save.play_time_seconds as f64);
        self.storage.chapter = runtime.current_label().map(str::to_owned);
        self.image_hints = runtime.upcoming_images(4);
        self.runtime = Some(runtime);
        self.video = None;
        self.storage.profile_revision = None;
        self.screen = Screen::Playing;
        self.overlay = None;
        self.storage.progress_dirty = false;
        self.fatal_error = None;
        self.track_current_dialogue();
        Ok(())
    }

    pub(super) fn load_latest(&mut self) {
        if let Some(slot) = self.storage.slots.iter().find(|slot| {
            !slot.corrupt
                && (slot.project_id.is_empty() || slot.project_id == self.program.project_id)
        }) {
            let name = slot.name.clone();
            self.load_slot(&name);
        }
    }
    pub(super) fn quick_save(&mut self) {
        self.queue_save("quick", Some(3));
    }
    pub(super) fn quick_load(&mut self) {
        self.load_slot("quick-1");
    }
    pub(super) fn auto_save(&mut self) {
        self.queue_save("auto", Some(5));
        self.storage.last_auto = Instant::now();
    }
    pub(super) fn request_quit(&mut self) {
        self.redraw = true;
        if self.storage.quit_after_save {
            return;
        }
        if self.runtime.is_none() || self.storage.quiet {
            self.quit = true;
            return;
        }
        self.storage.confirmation = Some(Confirmation::Quit);
        self.overlay = Some(Overlay::Confirm);
    }
    pub(super) fn confirm_pending(&mut self) {
        let Some(action) = self.storage.confirmation.take() else {
            return;
        };
        self.overlay = None;
        match action {
            Confirmation::Save(slot) => self.queue_save(&slot, None),
            Confirmation::Load(slot) => self.queue_load(&slot),
            Confirmation::Delete(slot) => {
                if let Err(error) = self.storage.worker.submit(SaveRequest::Delete(slot)) {
                    self.notice = Some((error, 5.0));
                }
                self.overlay = Some(Overlay::LoadSlots);
            }
            Confirmation::Quit => {
                self.storage.quit_after_save = true;
                self.queue_save("resume", None);
            }
        }
    }
    pub(super) fn save_metadata(&self) -> SaveMetadata {
        SaveMetadata {
            project_id: self.program.project_id.clone(),
            content_version: self.program.fingerprint.clone(),
            play_time_seconds: self.play_time_seconds.max(0.0) as u64,
            chapter: self
                .runtime
                .as_ref()
                .and_then(Runtime::current_label)
                .map(str::to_owned),
            presentation: Some(SavePresentation {
                sprite_elapsed_ms: (self.play_time_seconds * 1000.0) as u64,
                dialogue_page: self.dialogue_view.page(),
                visible_characters: self.visible_characters.max(0.0) as usize,
                pause_remaining_ms: (self.pause_remaining.max(0.0) * 1000.0) as u32,
                effect_remaining_ms: (self.effect_remaining.max(0.0) * 1000.0) as u32,
                note: self.storage.note.clone(),
                thumbnail_png: Vec::new(),
            }),
        }
    }

    fn capture_save_thumbnail(&self) -> renrs::save::worker::SaveThumbnail {
        let target = render_target(240, 135);
        let mut camera = Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            super::CANVAS_WIDTH,
            super::CANVAS_HEIGHT,
        ));
        camera.render_target = Some(target.clone());
        push_camera_state();
        set_camera(&camera);
        clear_background(BLACK);
        if let Some(runtime) = &self.runtime {
            self.draw_stage_on(runtime.stage(), 1.0, Some(&target));
            self.draw_media();
        }
        // Flush the thumbnail pass before reading its small pixel buffer.
        set_default_camera();
        let frame = target.texture.get_texture_data();
        pop_camera_state();
        renrs::save::worker::SaveThumbnail { rgba: frame.bytes }
    }
}
