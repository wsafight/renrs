use macroquad::prelude::*;
use renrs::{Runtime, WaitState};

use super::app::{App, Overlay, Screen};

impl App {
    pub(super) fn prepare_assets(&mut self) {
        let asset_revision = self.assets.revision();
        self.audio.prepare(&self.source);
        if self.overlay != Some(Overlay::History) {
            self.audio.clear_replay();
        }
        if let Some(notice) = self.audio.take_notice() {
            self.redraw = true;
            self.notice = Some((notice, 6.0));
        }
        if let Some(dialogue) = self
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.stage().dialogue.as_ref())
        {
            self.dialogue_view
                .prepare_reading(self.runtime.as_ref().unwrap());
            let nvl = self.dialogue_view.nvl.clone();
            self.dialogue_view
                .prepare(nvl.as_deref().unwrap_or(dialogue), &self.dialogue_theme());
        }
        let mut paths = std::mem::take(&mut self.asset_paths);
        paths.extend(self.media_paths());
        if self.overlay == Some(Overlay::Collection) {
            let config = &self.program.progress;
            let profile = &self.storage.profile;
            let page = self.screen_scroll.get("collection").copied().unwrap_or(0);
            paths.extend(
                [
                    (&config.achievements, &profile.achievements),
                    (&config.gallery, &profile.gallery),
                    (&config.endings, &profile.endings),
                ]
                .into_iter()
                .flat_map(|(items, unlocked)| items.iter().map(move |item| (item, unlocked)))
                .skip(page * 6)
                .take(6)
                .filter(|(item, unlocked)| unlocked.contains(&item.id))
                .filter_map(|(item, _)| item.image.clone()),
            );
        }
        if self.screen == Screen::MainMenu
            && let Some(path) = &self.title_background
        {
            paths.push(path.clone());
        }
        if let Some(runtime) = &self.runtime {
            if let Some(path) = &runtime.stage().background {
                paths.push(path.clone());
            }
            paths.extend(
                runtime
                    .stage()
                    .sprites
                    .iter()
                    .flat_map(renrs::runtime::SpriteState::image_paths),
            );
        }
        paths.extend(self.active_screen_images());
        self.assets.prepare(&self.source, &paths, &self.image_hints);
        paths.clear();
        self.asset_paths = paths;
        self.redraw |= asset_revision != self.assets.revision();
        if let Some(notice) = self.assets.take_notice() {
            self.redraw = true;
            self.notice = Some((notice, 6.0));
        }

        if let Some(runtime) = &self.runtime {
            self.audio
                .sync_music(runtime.stage().music.as_ref(), &self.source, &self.settings);
            self.audio.sync_voice(
                runtime.stage().voice.as_deref(),
                &self.source,
                &self.settings,
            );
            self.audio
                .sync_sound(runtime.stage().sound.as_ref(), &self.settings);
        }
    }

    pub(super) fn update_timers(&mut self) {
        let elapsed = get_frame_time();
        let delta = elapsed.min(0.25);
        self.sync_video_audio();
        let mut should_continue = false;
        if self.screen == Screen::Playing
            && self.overlay.is_none()
            && self.assets.is_ready()
            && self.media_ready()
            && !self.storage.loading
            && !self.storage.quit_after_save
        {
            self.play_time_seconds += f64::from(elapsed.min(1.0));
            if matches!(
                self.runtime.as_ref().and_then(Runtime::waiting),
                Some(WaitState::Dialogue)
            ) {
                should_continue = self.update_dialogue_timer(delta);
            }
            if matches!(
                self.runtime.as_ref().and_then(Runtime::waiting),
                Some(WaitState::Pause { .. })
            ) {
                self.pause_remaining -= delta;
                if self.pause_remaining <= 0.0 {
                    self.continue_story();
                }
            }
            if matches!(
                self.runtime.as_ref().and_then(Runtime::waiting),
                Some(WaitState::Effect { .. })
            ) {
                if let Some(position) = self.video_audio_position() {
                    if let Some(WaitState::Effect { effect }) =
                        self.runtime.as_ref().and_then(Runtime::waiting)
                    {
                        self.effect_remaining = (effect.seconds() - position).max(0.0);
                    }
                } else {
                    self.effect_remaining -= delta;
                }
                if self.effect_remaining <= 0.0 {
                    self.continue_story();
                }
            }
        }
        if should_continue {
            self.continue_story();
        }
        if let Some((_, remaining)) = &mut self.notice {
            *remaining -= delta;
            if *remaining <= 0.0 {
                self.notice = None;
                self.redraw = true;
            }
        }
    }

    fn update_dialogue_timer(&mut self, delta: f32) -> bool {
        let dialogue_length = self.dialogue_view.character_count();
        let previous = self.visible_characters as usize;
        if let Some(remaining) = &mut self.dialogue_cue_remaining {
            if remaining.is_finite() {
                *remaining -= delta;
                if *remaining <= 0.0 {
                    self.complete_dialogue_cue();
                }
            }
        } else if !self.playback.skip_read {
            let mut target = (self.visible_characters + delta * self.settings.text_speed)
                .min(dialogue_length as f32);
            while let Some(cue) = self.dialogue_view.cue() {
                if cue.position < previous {
                    self.dialogue_view.consume_cue();
                    continue;
                }
                if cue.position > target as usize {
                    break;
                }
                match cue.kind {
                    renrs::text::TextCue::Fast => {
                        self.dialogue_view.consume_cue();
                        target = dialogue_length as f32;
                    }
                    renrs::text::TextCue::Wait { hundredths }
                    | renrs::text::TextCue::Page { hundredths } => {
                        target = cue.position as f32;
                        self.dialogue_cue_remaining = Some(
                            hundredths.map_or(f32::INFINITY, |value| f32::from(value) / 100.0),
                        );
                        break;
                    }
                    renrs::text::TextCue::NoWait => {
                        self.dialogue_view.consume_cue();
                    }
                }
            }
            self.visible_characters = target;
        }
        self.redraw |= previous != self.visible_characters as usize;
        if self.playback.skip_read {
            if self.playback.current_dialogue_was_read || self.settings.skip_unread {
                self.skip_remaining -= delta;
                return self.skip_remaining <= 0.0;
            }
            self.playback.skip_read = false;
        } else if (self.playback.auto || self.dialogue_view.no_wait())
            && self.dialogue_cue_remaining.is_none()
            && self.visible_characters >= dialogue_length as f32
            && (!self.settings.wait_voice
                || self.settings.voice_volume <= f32::EPSILON
                || !self.audio.voice_busy()
                || self.dialogue_view.no_wait())
        {
            self.auto_remaining -= delta;
            return self.auto_remaining <= 0.0;
        }
        false
    }
}
