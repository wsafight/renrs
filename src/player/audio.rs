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
    voice: Option<String>,
    replay: Option<String>,
    volumes: Option<[f32; 3]>,
    notices: Vec<String>,
    video: Option<(Option<String>, bool)>,
}

impl AudioManager {
    pub(super) fn sync_video(&mut self, path: Option<String>, position: f32, paused: bool) {
        let state = (path, paused);
        if self.video.as_ref() != Some(&state)
            && self.send(Command::Video {
                path: state.0.clone(),
                position,
                paused,
            })
        {
            self.video = Some(state);
        }
    }
    pub(super) fn video_position(&self) -> Option<f32> {
        self.worker.as_ref().and_then(AudioWorker::video_position)
    }
    pub(super) fn stop_video(&mut self) {
        self.sync_video(None, 0.0, true);
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

    pub(super) fn handle(
        &mut self,
        events: Vec<AudioEvent>,
        _source: &ProjectSource,
        settings: &Settings,
    ) -> bool {
        for event in events {
            match event {
                AudioEvent::PlaySound { path, volume }
                    if settings.sound_volume > 0.0 && volume > 0.0 =>
                {
                    self.send(Command::Sound(path, volume));
                }
                AudioEvent::StopMusic { fade_out } => {
                    self.send(Command::StopMusic(fade_out));
                    self.music = None;
                }
                AudioEvent::PlayVoice { .. } | AudioEvent::StopVoice if self.replay.is_none() => {
                    self.stop_voice();
                }
                _ => {}
            }
        }
        self.apply_volume(settings);
        let mut finished = false;
        if let Some(worker) = &self.worker {
            while let Some(response) = worker.poll() {
                match response {
                    Response::MusicEnded(path) => {
                        if self.music.as_ref().is_some_and(|music| music.path == path) {
                            finished = true;
                            self.music = None;
                        }
                    }
                    Response::Error(error) => self.notices.push(error),
                }
            }
        }
        finished
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
    }
}
