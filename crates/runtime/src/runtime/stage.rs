use super::{
    AudioEvent, MusicState, Runtime, RuntimeError, SpriteState, TransformState, TransitionKind,
    VisualEffect, WaitState, execution,
};
use crate::text::TextCue;
use std::sync::Arc;

impl Runtime {
    pub(super) fn apply_window(&mut self, visible: bool) {
        Arc::make_mut(&mut self.stage).window = visible;
        self.instruction += 1;
    }

    pub(super) fn show_screen(&mut self, name: String) {
        let stage = Arc::make_mut(&mut self.stage);
        if !stage.shown_screens.contains(&name) {
            stage.shown_screens.push(name);
        }
        self.instruction += 1;
    }

    pub(super) fn hide_screen(&mut self, name: &str) {
        self.remove_screen(name);
        self.instruction += 1;
    }

    pub(super) fn remove_screen(&mut self, name: &str) {
        Arc::make_mut(&mut self.stage)
            .shown_screens
            .retain(|shown| shown != name);
    }

    pub(super) fn call_screen(&mut self, name: String) -> WaitState {
        let stage = Arc::make_mut(&mut self.stage);
        if !stage.shown_screens.contains(&name) {
            stage.shown_screens.push(name.clone());
        }
        self.set_waiting(WaitState::Screen { name })
    }

    pub(super) fn apply_say_attributes(
        &mut self,
        speaker: &str,
        attributes: &[String],
        line: usize,
    ) -> Result<(), RuntimeError> {
        let Some(character) = self.program.characters.get(speaker) else {
            return Ok(());
        };
        let Some(image) = &character.image else {
            return Ok(());
        };
        let variant = if attributes.is_empty() {
            image.clone()
        } else {
            format!("{image}_{}", attributes.join("_"))
        };
        let path = self
            .program
            .images
            .get(&variant)
            .cloned()
            .or_else(|| self.program.images.get(image).cloned())
            .ok_or_else(|| execution(line, format!("unknown image `{variant}`")))?;
        let alias = speaker.to_owned();
        let composition = self.resolve_image(&path, line)?;
        let stage = Arc::make_mut(&mut self.stage);
        if let Some(existing) = stage
            .sprites
            .iter_mut()
            .find(|sprite| sprite.alias == alias)
        {
            existing.path = path;
            existing.composition = composition;
            existing.attributes = attributes.to_vec();
        } else {
            stage.sprites.push(SpriteState {
                composition,
                path,
                alias,
                position: crate::syntax::Position::Center,
                layer: 0,
                display_layer: "master".to_owned(),
                display_order: 0,
                transform: TransformState::identity(),
                attributes: attributes.to_vec(),
            });
        }
        Ok(())
    }

    pub(super) fn dialogue_no_wait(runs: &[crate::text::TextRun]) -> bool {
        runs.iter()
            .any(|run| matches!(run.cue, Some(TextCue::NoWait)))
    }

    pub(super) fn play_sound(&mut self, path: String, volume: f32, repeat: bool, queue: bool) {
        let sound = MusicState {
            path: path.clone(),
            repeat,
            fade_in: 0.0,
            volume,
        };
        let stage = Arc::make_mut(&mut self.stage);
        if queue && stage.sound.is_some() {
            stage.sound_queue.push(sound);
            self.audio_events.push(AudioEvent::QueueSound {
                path,
                volume,
                repeat,
            });
        } else {
            stage.sound = Some(sound);
            stage.sound_queue.clear();
            self.audio_events.push(AudioEvent::PlaySound {
                path,
                volume,
                repeat,
            });
        }
        self.instruction += 1;
    }

    pub(super) fn stop_sound(&mut self, fade_out: f32) {
        let stage = Arc::make_mut(&mut self.stage);
        stage.sound = None;
        stage.sound_queue.clear();
        self.audio_events.push(AudioEvent::StopSound { fade_out });
        self.instruction += 1;
    }

    pub(super) fn stop_voice(&mut self, fade_out: f32) {
        Arc::make_mut(&mut self.stage).voice = None;
        self.audio_events.push(AudioEvent::StopVoice { fade_out });
        self.instruction += 1;
    }

    pub(super) fn play_music_if_changed(
        &mut self,
        path: String,
        repeat: bool,
        fade_in: f32,
        volume: f32,
        if_changed: bool,
    ) {
        if if_changed
            && self
                .stage
                .music
                .as_ref()
                .is_some_and(|music| music.path == path)
        {
            self.instruction += 1;
            return;
        }
        Arc::make_mut(&mut self.stage).music = Some(MusicState {
            path: path.clone(),
            repeat,
            fade_in,
            volume,
        });
        Arc::make_mut(&mut self.stage).music_queue.clear();
        self.audio_events.push(AudioEvent::PlayMusic {
            path,
            repeat,
            fade_in,
            volume,
        });
        self.instruction += 1;
    }

    pub(super) fn transition_effect(&mut self, kind: TransitionKind, seconds: f32) -> VisualEffect {
        let previous = Box::new(self.previous_stage.take().unwrap_or_default());
        match kind {
            TransitionKind::Fade => VisualEffect::Fade { seconds },
            TransitionKind::Dissolve => VisualEffect::Dissolve {
                from: previous,
                seconds,
            },
            TransitionKind::PushLeft => VisualEffect::Push {
                from: previous,
                left: true,
                seconds,
            },
            TransitionKind::PushRight => VisualEffect::Push {
                from: previous,
                left: false,
                seconds,
            },
            TransitionKind::WipeLeft => VisualEffect::Wipe {
                from: previous,
                left: true,
                seconds,
            },
            TransitionKind::WipeRight => VisualEffect::Wipe {
                from: previous,
                left: false,
                seconds,
            },
            TransitionKind::PunchH => VisualEffect::Punch {
                vertical: false,
                seconds,
            },
            TransitionKind::PunchV => VisualEffect::Punch {
                vertical: true,
                seconds,
            },
        }
    }
}
