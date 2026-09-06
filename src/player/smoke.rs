use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use macroquad::prelude::*;
use renrs::{Runtime, WaitState};
use serde_json::{Value, json};

use super::app::{App, Overlay};
use super::ui_slots::SlotGroup;
use crate::frontend::{UiAction, UiActions};

const STEPS: [&str; 18] = [
    "title",
    "dialogue",
    "dialogue_page_2",
    "choice",
    "choice_last",
    "history_start",
    "history_end",
    "settings",
    "accessibility",
    "collection",
    "save_tools",
    "confirmation",
    "languages",
    "localized_dialogue",
    "manual_saves",
    "quick_saves",
    "auto_saves",
    "quick_load",
];

pub(super) struct SmokeTest {
    output: PathBuf,
    started: Instant,
    step: usize,
    entered: bool,
    stable_frames: usize,
    interactions: usize,
    saved: Option<Value>,
    captures: Vec<Value>,
    skipped: Vec<&'static str>,
}

impl SmokeTest {
    pub(super) fn new(output: PathBuf) -> Result<Self, String> {
        if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::create_dir(&output).map_err(|error| format!("{}: {error}", output.display()))?;
        Ok(Self {
            output,
            started: Instant::now(),
            step: 0,
            entered: false,
            stable_frames: 0,
            interactions: 0,
            saved: None,
            captures: Vec::new(),
            skipped: Vec::new(),
        })
    }

    pub(super) fn data_root(&self) -> PathBuf {
        self.output.join("data")
    }

    pub(super) fn finished(&self) -> bool {
        self.step == STEPS.len()
    }

    pub(super) fn initialize(app: &mut App) -> Result<(), String> {
        if let Some((notice, _)) = &app.notice {
            return Err(notice.clone());
        }
        app.settings.music_volume = 0.0;
        app.settings.sound_volume = 0.0;
        app.settings.voice_volume = 0.0;
        app.storage.quiet = true;
        app.select_language(None);
        app.overlay = None;
        Ok(())
    }

    pub(super) fn prepare(&mut self, app: &mut App) -> Result<UiActions, String> {
        if self.started.elapsed() > Duration::from_secs(45) || self.interactions > 2000 {
            return Err(format!("smoke test timed out at {}", STEPS[self.step]));
        }
        if !self.entered {
            self.enter(app)?;
            self.entered = true;
            app.notice = None;
        }
        if app.storage.worker.busy() {
            return Ok(UiActions::new(Vec::new()));
        }
        let target = match STEPS[self.step] {
            "dialogue" => Some(false),
            "choice" => Some(true),
            _ => None,
        };
        if let Some(choice) = target {
            let waiting = app.runtime.as_ref().and_then(Runtime::waiting);
            let ready = if choice {
                matches!(waiting, Some(WaitState::Choice { .. }))
            } else {
                matches!(waiting, Some(WaitState::Dialogue))
            };
            if !ready {
                if matches!(waiting, Some(WaitState::Finished) | None) {
                    return Err(
                        "smoke project must contain dialogue followed by a choice".to_owned()
                    );
                }
                app.continue_story();
                self.interactions += 1;
                self.stable_frames = 0;
            }
        }
        app.visible_characters = f32::MAX;
        let actions = match STEPS[self.step] {
            "history_start" => vec![UiAction::Home],
            "choice_last" | "history_end" => vec![UiAction::End],
            _ => Vec::new(),
        };
        Ok(UiActions::new(actions))
    }

    fn enter(&mut self, app: &mut App) -> Result<(), String> {
        match STEPS[self.step] {
            "dialogue" => app.start_new_game(),
            "dialogue_page_2" => {
                self.saved = Some(snapshot(app)?);
                app.save_slot("slot-1");
                app.quick_save();
                if !app.dialogue_view.next_page() {
                    self.skipped.push("dialogue_page_2");
                    self.step += 1;
                }
            }
            "history_start" | "history_end" => app.overlay = Some(Overlay::History),
            "settings" => app.overlay = Some(Overlay::Settings),
            "accessibility" => app.overlay = Some(Overlay::Accessibility),
            "collection" => app.overlay = Some(Overlay::Collection),
            "save_tools" => app.overlay = Some(Overlay::SaveTools),
            "confirmation" => {
                app.storage.confirmation = Some(super::saving::Confirmation::Quit);
                app.overlay = Some(Overlay::Confirm);
            }
            "languages" => app.overlay = Some(Overlay::Languages),
            "localized_dialogue" => {
                app.quick_load();
                let language = app.localizer.languages().next().map(ToOwned::to_owned);
                app.select_language(language.clone());
                if app.localizer.language() != language.as_deref() {
                    return Err("language selection did not apply".to_owned());
                }
                app.overlay = None;
            }
            "manual_saves" => {
                app.overlay = Some(Overlay::SaveSlots);
                app.slot_group = SlotGroup::Manual;
            }
            "quick_saves" => {
                app.overlay = Some(Overlay::LoadSlots);
                app.slot_group = SlotGroup::Quick;
            }
            "auto_saves" => app.slot_group = SlotGroup::Auto,
            "quick_load" => {
                app.select_language(None);
                app.quick_load();
                for slot in ["slot-1", "quick-1", "auto-1"] {
                    app.saves.load(slot).map_err(|error| error.to_string())?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn after_draw(&mut self, app: &App) -> Result<bool, String> {
        if let Some(error) = &app.fatal_error {
            return Err(error.clone());
        }
        if let Some((notice, _)) = &app.notice {
            return Err(format!("player warning during smoke test: {notice}"));
        }
        if !app.assets.is_ready() || app.storage.worker.busy() {
            self.stable_frames = 0;
            return Ok(false);
        }
        self.stable_frames += 1;
        if self.stable_frames < 3 {
            return Ok(false);
        }
        if STEPS[self.step] == "quick_load" && Some(snapshot(app)?) != self.saved {
            return Err("quick load did not restore the saved runtime state".to_owned());
        }
        let frame = get_screen_data();
        let distinct: HashSet<_> = frame.bytes.as_chunks::<4>().0.iter().step_by(13).collect();
        if distinct.len() < 32 {
            return Err(format!("blank or incomplete frame at {}", STEPS[self.step]));
        }
        let file = format!("{}.png", STEPS[self.step]);
        let path = self.output.join(&file);
        // Macroquad's readback uses the bottom-left OpenGL origin.
        let image = image::RgbaImage::from_raw(
            u32::from(frame.width),
            u32::from(frame.height),
            frame.bytes.clone(),
        )
        .ok_or("invalid framebuffer dimensions")?;
        image::imageops::flip_vertical(&image)
            .save(&path)
            .map_err(|error| error.to_string())?;
        self.captures.push(json!({
            "screen": STEPS[self.step], "file": file,
            "width": frame.width, "height": frame.height, "sampled_colors": distinct.len(),
            "texture_bytes": app.assets.resident_bytes(), "language": app.localizer.language(),
        }));
        self.step += 1;
        self.entered = false;
        self.stable_frames = 0;
        if self.step < STEPS.len() {
            return Ok(false);
        }
        let report = json!({"passed": true, "project_id": app.program.project_id,
            "performance": app.metrics.report(),
            "platform": std::env::consts::OS, "elapsed_ms": self.started.elapsed().as_millis(),
            "quick_load_restored": true, "muted": true, "captures": self.captures, "skipped": self.skipped});
        fs::write(
            self.output.join("report.json"),
            serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        println!("Smoke test passed: {}", self.output.display());
        Ok(true)
    }
}

fn snapshot(app: &App) -> Result<Value, String> {
    serde_json::to_value(app.runtime.as_ref().ok_or("runtime is missing")?.snapshot())
        .map_err(|error| error.to_string())
}
