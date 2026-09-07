use super::app::{App, Screen};
use renrs::{Runtime, WaitState};

impl App {
    pub(super) fn start_new_game(&mut self) {
        match Runtime::new(self.program.clone()) {
            Ok(mut runtime) => {
                if let Some(previous) = &self.runtime {
                    self.storage.profile = previous.profile().clone();
                }
                self.storage.profile_revision = None;
                self.video = None;
                if let Err(error) = runtime.set_profile(self.storage.profile.clone()) {
                    self.fatal_error = Some(error.to_string());
                    return;
                }
                if let Err(error) = runtime.set_localizer(self.localizer.clone()) {
                    self.fatal_error = Some(error.to_string());
                    return;
                }
                let result = runtime.advance();
                self.runtime = Some(runtime);
                self.play_time_seconds = 0.0;
                self.playback.auto = false;
                self.playback.skip_read = false;
                self.screen = Screen::Playing;
                self.overlay = None;
                self.fatal_error = None;
                self.storage.chapter = None;
                self.audio.stop_music();
                self.audio.stop_voice();
                self.handle_wait(result);
            }
            Err(error) => self.fatal_error = Some(error.to_string()),
        }
    }

    pub(super) fn continue_story(&mut self) {
        self.redraw = true;
        if self.storage.loading || self.storage.quit_after_save {
            return;
        }
        if self
            .runtime
            .as_ref()
            .is_some_and(|runtime| matches!(runtime.waiting(), Some(WaitState::Dialogue)))
            && self.dialogue_view.next_page()
        {
            self.visible_characters = 0.0;
            self.auto_remaining = self.settings.auto_delay;
            self.skip_remaining = 0.08;
            self.storage.progress_dirty = true;
            return;
        }
        if let Some(runtime) = &mut self.runtime {
            let result = runtime.continue_story();
            self.handle_wait(result);
        }
    }

    pub(super) fn handle_wait(&mut self, result: Result<WaitState, renrs::RuntimeError>) {
        self.redraw = true;
        self.audio.stop_video();
        self.video = None;
        self.image_hints = self
            .runtime
            .as_ref()
            .map_or_else(Vec::new, |runtime| runtime.upcoming_images(4));
        self.history_view.invalidate();
        self.dialogue_view.invalidate();
        self.storage.progress_dirty = true;
        match result {
            Ok(WaitState::Dialogue) => {
                self.visible_characters = 0.0;
                self.auto_remaining = self.settings.auto_delay;
                self.skip_remaining = 0.08;
                self.track_current_dialogue();
                let chapter = self
                    .runtime
                    .as_ref()
                    .and_then(Runtime::current_label)
                    .map(str::to_owned);
                if chapter != self.storage.chapter {
                    self.storage.chapter = chapter;
                    self.auto_save();
                }
            }
            Ok(WaitState::Choice { .. }) => {
                self.visible_characters = f32::MAX;
                self.selected_choice = 0;
                self.playback.skip_read = false;
                self.focus.clear();
                self.auto_save();
            }
            Ok(WaitState::Pause { seconds }) => self.pause_remaining = seconds,
            Ok(WaitState::Effect { effect }) => {
                self.effect_remaining = if self.theme.reduced_motion
                    && !matches!(effect, renrs::runtime::VisualEffect::Video { .. })
                {
                    0.0
                } else {
                    effect.seconds()
                };
            }
            Ok(WaitState::Finished) => self.auto_save(),
            Err(error) => self.fatal_error = Some(error.to_string()),
        }
    }

    pub(super) fn track_current_dialogue(&mut self) {
        let id = self
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.stage().dialogue.as_ref())
            .and_then(|dialogue| dialogue.statement_id.clone());
        self.playback.current_dialogue_was_read = id
            .as_ref()
            .is_some_and(|id| self.read_state.statements.contains(id));
        if let Some(id) = id
            && self.read_state.statements.insert(id)
        {
            self.read_dirty = true;
        }
    }

    pub(super) fn rollback_story(&mut self) {
        let result = self
            .runtime
            .as_mut()
            .map_or(Err(renrs::RuntimeError::CannotRollback), Runtime::rollback);
        self.handle_wait(result);
    }
}
