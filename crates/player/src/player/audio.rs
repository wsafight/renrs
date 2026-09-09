use super::{
    audio_worker::{AudioWorker, Command, Response},
    settings::Settings,
};
use renrs::{
    ProjectSource,
    runtime::{AudioEvent, MusicState},
};

#[derive(Default)]
pub(super) struct AudioManager {
    worker: Option<AudioWorker>,
    music: Option<MusicState>,
    sound: Option<MusicState>,
    voice: Option<String>,
    replay: Option<String>,
    volumes: Option<[f32; 3]>,
    notices: Vec<String>,
    video: Option<(Option<String>, bool, f32)>,
}

impl AudioManager {
    pub(super) fn sync_video(
        &mut self,
        path: Option<String>,
        position: f32,
        paused: bool,
        relative_volume: f32,
    ) {
        let state = (path, paused, relative_volume);
        if self.video.as_ref() != Some(&state)
            && self.send(Command::Video {
                path: state.0.clone(),
                position,
                paused,
                relative_volume,
            })
        {
            self.video = Some(state);
        }
    }
    pub(super) fn video_position(&self) -> Option<f32> {
        self.worker.as_ref().and_then(AudioWorker::video_position)
    }
    pub(super) fn stop_video(&mut self) {
        self.sync_video(None, 0.0, true, 1.0);
    }
    pub(super) fn prepare(&mut self, source: &ProjectSource) {
        self.worker
            .get_or_insert_with(|| AudioWorker::new(source.clone()));
    }

    fn send(&mut self, command: Command) -> bool {
        if self.worker.as_ref().is_some_and(|worker| !worker.alive()) {
            return false;
        }
        match self.worker.as_ref().map(|worker| worker.submit(command)) {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                if self.notices.len() < 8 {
                    self.notices.push(error);
                }
                false
            }
            None => false,
        }
    }

    pub(super) fn sync_music(
        &mut self,
        state: Option<&MusicState>,
        _source: &ProjectSource,
        settings: &Settings,
    ) {
        self.apply_volume(settings);
        if self.music.as_ref() != state && self.send(Command::Music(state.cloned())) {
            self.music = state.cloned();
        }
    }

    pub(super) fn sync_voice(
        &mut self,
        state: Option<&str>,
        _source: &ProjectSource,
        settings: &Settings,
    ) {
        self.apply_volume(settings);
        let path = self.replay.as_deref().or(state);
        if self.voice.as_deref() != path {
            let path = path.map(str::to_owned);
            if self.send(Command::Voice(path.clone())) {
                self.voice = path;
            }
        }
    }

    pub(super) fn sync_sound(&mut self, state: Option<&MusicState>, settings: &Settings) {
        self.apply_volume(settings);
        if self.sound.as_ref() != state && self.send(Command::Sound(state.cloned())) {
            self.sound = state.cloned();
        }
    }

    pub(super) fn handle(
        &mut self,
        events: Vec<AudioEvent>,
        _source: &ProjectSource,
        settings: &Settings,
    ) -> AudioCompletions {
        for event in events {
            match event {
                AudioEvent::PlaySound {
                    path,
                    volume,
                    repeat,
                } if settings.sound_volume > 0.0 && volume > 0.0 => {
                    let sound = MusicState {
                        path,
                        repeat,
                        fade_in: 0.0,
                        volume,
                    };
                    if self.send(Command::Sound(Some(sound.clone()))) {
                        self.sound = Some(sound);
                    }
                }
                AudioEvent::StopSound { fade_out } => {
                    self.send(Command::StopSound(fade_out));
                    self.sound = None;
                }
                AudioEvent::StopMusic { fade_out } => {
                    self.send(Command::StopMusic(fade_out));
                    self.music = None;
                }
                AudioEvent::PlayVoice { path } if self.replay.is_none() => {
                    if self.send(Command::Voice(Some(path.clone()))) {
                        self.voice = Some(path);
                    }
                }
                AudioEvent::StopVoice { fade_out } if self.replay.is_none() => {
                    self.send(Command::StopVoice(fade_out));
                    self.voice = None;
                }
                _ => {}
            }
        }
        self.apply_volume(settings);
        let mut completed = AudioCompletions::default();
        if let Some(worker) = &self.worker {
            while let Some(response) = worker.poll() {
                match response {
                    Response::MusicEnded(path) => {
                        if self.music.as_ref().is_some_and(|music| music.path == path) {
                            completed.music = true;
                            self.music = None;
                        }
                    }
                    Response::SoundEnded(path) => {
                        if self.sound.as_ref().is_some_and(|sound| sound.path == path) {
                            completed.sound = true;
                            self.sound = None;
                        }
                    }
                    Response::Error(error) => self.notices.push(error),
                }
            }
        }
        completed
    }

    pub(super) fn voice_busy(&self) -> bool {
        self.worker.as_ref().is_some_and(AudioWorker::voice_busy)
    }
    pub(super) fn replay_voice(&mut self, path: &str) {
        self.stop_voice();
        self.replay = Some(path.to_owned());
    }
    pub(super) fn clear_replay(&mut self) {
        if self.replay.take().is_some() {
            self.stop_voice();
        }
    }
    pub(super) fn take_notice(&mut self) -> Option<String> {
        self.notices.pop()
    }
    pub(super) fn apply_volume(&mut self, settings: &Settings) {
        let volumes = [
            settings.music_volume,
            settings.sound_volume,
            settings.voice_volume,
        ];
        if self.volumes != Some(volumes) && self.send(Command::Volume(volumes)) {
            self.volumes = Some(volumes);
        }
    }
    pub(super) fn stop_music(&mut self) {
        if self.send(Command::StopMusic(0.0)) {
            self.music = None;
        }
    }
    pub(super) fn stop_voice(&mut self) {
        if self.send(Command::Voice(None)) {
            self.voice = None;
        }
    }
    pub(super) fn stop_sound(&mut self) {
        if self.send(Command::StopSound(0.0)) {
            self.sound = None;
        }
    }
    pub(super) fn invalidate(&mut self, paths: &[String]) {
        if self
            .music
            .as_ref()
            .is_some_and(|music| paths.contains(&music.path))
        {
            self.stop_music();
        }
        if self
            .voice
            .as_ref()
            .is_some_and(|voice| paths.contains(voice))
        {
            self.stop_voice();
        }
        if self
            .sound
            .as_ref()
            .is_some_and(|sound| paths.contains(&sound.path))
        {
            self.stop_sound();
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct AudioCompletions {
    pub(super) music: bool,
    pub(super) sound: bool,
}
