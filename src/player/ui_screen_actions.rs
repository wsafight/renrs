use super::app::{App, Overlay, Screen};
use super::ui_slots::SlotGroup;
use macroquad::prelude::*;
use renrs::screens::{Action, Preference};

#[derive(Clone)]
pub(super) enum ScreenCommand {
    Extension(String, String, String),
    Set(String, String),
    Action(Action),
    Save(String),
    Load(String),
    Language(Option<String>),
    Choose(usize),
}

impl App {
    pub(super) fn screen_action_enabled(&self, action: Action) -> bool {
        match action {
            Action::Continue | Action::Load => self.storage.slots.iter().any(|slot| {
                !slot.corrupt
                    && (slot.project_id.is_empty() || slot.project_id == self.program.project_id)
            }),
            Action::QuickLoad => self
                .storage
                .slots
                .iter()
                .any(|slot| slot.name == "quick-1" && !slot.corrupt),
            Action::Save | Action::QuickSave | Action::History | Action::Auto | Action::Skip => {
                self.runtime.is_some()
            }
            Action::Rollback => self
                .runtime
                .as_ref()
                .is_some_and(renrs::Runtime::can_rollback),
            _ => true,
        }
    }

    pub(super) fn screen_command(&mut self, command: ScreenCommand) {
        match command {
            ScreenCommand::Extension(variable, name, input) => {
                if let Some(runtime) = &mut self.runtime {
                    let result = renrs::expression::parse_expression(
                        &input,
                        renrs::screens::SCREENS_FILE,
                        1,
                        1,
                    )
                    .map_err(|error| error.to_string())
                    .and_then(|input| {
                        runtime
                            .apply_extension_expression(&variable, &name, &input)
                            .map_err(|error| error.to_string())
                    });
                    if let Err(error) = result {
                        self.notice = Some((error, 5.0));
                    } else {
                        self.storage.progress_dirty = true;
                    }
                }
            }
            ScreenCommand::Set(variable, expression) => {
                if let Some(runtime) = &mut self.runtime {
                    let result = renrs::expression::parse_expression(
                        &expression,
                        renrs::screens::SCREENS_FILE,
                        1,
                        1,
                    )
                    .map_err(|error| error.to_string())
                    .and_then(|expression| {
                        runtime
                            .apply_screen_expression(&variable, &expression)
                            .map_err(|error| error.to_string())
                    });
                    if let Err(error) = result {
                        self.notice = Some((error, 5.0));
                    } else {
                        self.storage.progress_dirty = true;
                    }
                }
            }
            ScreenCommand::Choose(index) => {
                if let Some(runtime) = &mut self.runtime {
                    let result = runtime.choose(index);
                    self.handle_wait(result);
                }
            }
            ScreenCommand::Save(slot) => self.save_slot(&slot),
            ScreenCommand::Load(slot) => self.load_slot(&slot),
            ScreenCommand::Language(language) => self.select_language(language),
            ScreenCommand::Action(action) => {
                if !self.screen_action_enabled(action) {
                    return;
                }
                match action {
                    Action::Collection => self.overlay = Some(Overlay::Collection),
                    Action::NewGame => self.start_new_game(),
                    Action::Continue => self.load_latest(),
                    Action::Save => self.overlay = Some(Overlay::SaveSlots),
                    Action::Load => self.overlay = Some(Overlay::LoadSlots),
                    Action::ManualSaves | Action::QuickSaves | Action::AutoSaves => {
                        self.slot_group = match action {
                            Action::QuickSaves => SlotGroup::Quick,
                            Action::AutoSaves => SlotGroup::Auto,
                            _ => SlotGroup::Manual,
                        };
                        self.overlay = Some(Overlay::LoadSlots);
                    }
                    Action::Settings => self.overlay = Some(Overlay::Settings),
                    Action::History => self.overlay = Some(Overlay::History),
                    Action::QuickSave => self.quick_save(),
                    Action::QuickLoad => self.quick_load(),
                    Action::Rollback => {
                        self.rollback_story();
                        self.overlay = None;
                    }
                    Action::Auto => self.playback.auto = !self.playback.auto,
                    Action::Skip => self.playback.skip_read = !self.playback.skip_read,
                    Action::Close => {
                        self.persist_settings();
                        self.overlay = None;
                        if self.runtime.is_none() {
                            self.screen = Screen::MainMenu;
                        }
                    }
                    Action::Quit => self.request_quit(),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn screen_slider(
        &mut self,
        setting: Preference,
        label: &str,
        rect: Rect,
        mouse: Vec2,
        focus: usize,
        actions: &crate::frontend::UiActions,
        theme: &renrs::theme::Theme,
    ) {
        let (value, min, max, step) = match setting {
            Preference::TextSpeed => (&mut self.settings.text_speed, 10.0, 100.0, 5.0),
            Preference::AutoDelay => (&mut self.settings.auto_delay, 0.5, 10.0, 0.5),
            Preference::MusicVolume => (&mut self.settings.music_volume, 0.0, 1.0, 0.05),
            Preference::SoundVolume => (&mut self.settings.sound_volume, 0.0, 1.0, 0.05),
            Preference::VoiceVolume => (&mut self.settings.voice_volume, 0.0, 1.0, 0.05),
        };
        let before = *value;
        *value = super::ui_common::slider(
            Rect::new(rect.x + 12.0, rect.y + 30.0, rect.w - 100.0, rect.h - 30.0),
            label,
            *value,
            min,
            max,
            mouse,
            self.focus.is(focus),
            super::ui_common::keyboard_adjustment(actions, self.focus.is(focus)),
            step,
            theme,
        );
        self.settings_dirty |= (*value - before).abs() > f32::EPSILON;
        self.audio.apply_volume(&self.settings);
        if is_mouse_button_released(MouseButton::Left) {
            self.persist_settings();
        }
    }
}
