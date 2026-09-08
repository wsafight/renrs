use renrs::{ProjectSource, runtime::MusicState};
use rodio::{Decoder, DeviceSinkBuilder, Player};
use std::io::BufReader;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::time::{Duration, Instant};

pub(super) enum Command {
    Video {
        path: Option<String>,
        position: f32,
        paused: bool,
        relative_volume: f32,
    },
    Music(Option<MusicState>),
    Voice(Option<String>),
    Sound(String, f32),
    StopMusic(f32),
    Volume([f32; 3]),
}
pub(super) enum Response {
    MusicEnded(String),
    Error(String),
}

pub(super) struct AudioWorker {
    requests: SyncSender<Command>,
    responses: Receiver<Response>,
    voice_busy: Arc<AtomicBool>,
    pending_voice: Arc<AtomicUsize>,
    pending_video: Arc<AtomicUsize>,
    alive: Arc<AtomicBool>,
    video_position: Arc<std::sync::atomic::AtomicU32>,
}

struct Track {
    path: String,
    player: Player,
    repeat: bool,
    fade_in: f32,
    started: Instant,
    fade_out: Option<(Instant, f32)>,
    volume: f32,
}

impl Track {
    fn apply_volume(&self, volume: f32) -> f32 {
        let fade_in = if self.fade_in > 0.0 {
            (self.started.elapsed().as_secs_f32() / self.fade_in).min(1.0)
        } else {
            1.0
        };
        let fade_out = self.fade_out.map_or(1.0, |(start, seconds)| {
            (1.0 - start.elapsed().as_secs_f32() / seconds).max(0.0)
        });
        self.player
            .set_volume(mixed_volume(volume, self.volume) * fade_in * fade_out);
        fade_out
    }
}

fn mixed_volume(channel: f32, relative: f32) -> f32 {
    channel * relative
}

impl AudioWorker {
    pub(super) fn new(source: ProjectSource) -> Self {
        let (requests, incoming) = mpsc::sync_channel(64);
        let (outgoing, responses) = mpsc::channel();
        let voice_busy = Arc::new(AtomicBool::new(false));
        let busy = voice_busy.clone();
        let pending_voice = Arc::new(AtomicUsize::new(0));
        let pending = pending_voice.clone();
        let pending_video = Arc::new(AtomicUsize::new(0));
        let video_pending = pending_video.clone();
        let alive = Arc::new(AtomicBool::new(true));
        let active = alive.clone();
        let video_position = Arc::new(std::sync::atomic::AtomicU32::new(f32::NAN.to_bits()));
        let clock = video_position.clone();
        std::thread::spawn(move || {
            if let Err(error) = run(
                &source,
                &incoming,
                &outgoing,
                &busy,
                &pending,
                &clock,
                &video_pending,
            ) {
                let _ = outgoing.send(Response::Error(error));
            }
            busy.store(false, Ordering::Release);
            active.store(false, Ordering::Release);
        });
        Self {
            requests,
            responses,
            voice_busy,
            pending_voice,
            pending_video,
            alive,
            video_position,
        }
    }
    pub(super) fn submit(&self, command: Command) -> Result<(), String> {
        let voice = matches!(command, Command::Voice(Some(_)));
        let video = matches!(command, Command::Video { .. });
        if video {
            self.pending_video.fetch_add(1, Ordering::AcqRel);
        }
        if voice {
            self.pending_voice.fetch_add(1, Ordering::AcqRel);
        }
        if let Err(error) = self.requests.try_send(command) {
            if video {
                self.pending_video.fetch_sub(1, Ordering::AcqRel);
            }
            if voice {
                self.pending_voice.fetch_sub(1, Ordering::AcqRel);
            }
            return Err(format!("audio queue unavailable: {error}"));
        }
        Ok(())
    }
    pub(super) fn poll(&self) -> Option<Response> {
        self.responses.try_recv().ok()
    }
    pub(super) fn voice_busy(&self) -> bool {
        self.alive()
            && (self.pending_voice.load(Ordering::Acquire) > 0
                || self.voice_busy.load(Ordering::Acquire))
    }
    pub(super) fn alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }
    pub(super) fn video_position(&self) -> Option<f32> {
        if self.pending_video.load(Ordering::Acquire) != 0 {
            return None;
        }
        let position = f32::from_bits(self.video_position.load(Ordering::Acquire));
        (self.alive() && position.is_finite()).then_some(position)
    }
}

fn track(
    source: &ProjectSource,
    mixer: &rodio::mixer::Mixer,
    path: String,
    repeat: bool,
    fade_in: f32,
    volume: f32,
    relative_volume: f32,
) -> Result<Track, String> {
    let reader = BufReader::with_capacity(
        64 * 1024,
        source
            .open_reader(&path)
            .map_err(|error| format!("{path}: {error}"))?,
    );
    let player = Player::connect_new(mixer);
    player.set_volume(if fade_in > 0.0 {
        0.0
    } else {
        mixed_volume(volume, relative_volume)
    });
    if repeat {
        player.append(Decoder::new_looped(reader).map_err(|error| format!("{path}: {error}"))?);
    } else {
        player.append(Decoder::new(reader).map_err(|error| format!("{path}: {error}"))?);
    }
    Ok(Track {
        path,
        player,
        repeat,
        fade_in,
        started: Instant::now(),
        fade_out: None,
        volume: relative_volume,
    })
}

#[allow(clippy::too_many_lines)]
fn run(
    source: &ProjectSource,
    incoming: &Receiver<Command>,
    outgoing: &mpsc::Sender<Response>,
    busy: &AtomicBool,
    pending_voice: &AtomicUsize,
    video_position: &std::sync::atomic::AtomicU32,
    pending_video: &AtomicUsize,
) -> Result<(), String> {
    let mut device = DeviceSinkBuilder::open_default_sink().map_err(|error| error.to_string())?;
    device.log_on_drop(false);
    let mixer = device.mixer();
    let mut music: Option<Track> = None;
    let mut voice: Option<Track> = None;
    let mut sounds = std::collections::VecDeque::<Track>::new();
    let mut volumes = [0.0; 3];
    let mut video = super::video_audio::VideoAudio::default();
    loop {
        let command = match incoming.recv_timeout(Duration::from_millis(5)) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let requested_voice = matches!(command, Some(Command::Voice(Some(_))));
        let requested_video = matches!(command, Some(Command::Video { .. }));
        let result = match command {
            Some(Command::Video {
                path,
                position,
                paused,
                relative_volume,
            }) => video.sync(
                source,
                mixer,
                path,
                position,
                paused,
                super::video_audio::VideoGain {
                    channel: volumes[1],
                    track: relative_volume,
                },
            ),
            Some(Command::Music(Some(state))) => {
                music = None;
                track(
                    source,
                    mixer,
                    state.path,
                    state.repeat,
                    state.fade_in,
                    volumes[0],
                    state.volume,
                )
                .map(|track| music = Some(track))
            }
            Some(Command::Music(None)) => {
                if music.as_ref().is_some_and(|track| track.fade_out.is_none()) {
                    music = None;
                }
                Ok(())
            }
            Some(Command::StopMusic(seconds)) => {
                if seconds > 0.0 {
                    if let Some(music) = &mut music {
                        music.fade_out = Some((Instant::now(), seconds));
                    }
                } else {
                    music = None;
                }
                Ok(())
            }
            Some(Command::Voice(path)) => {
                voice = None;
                path.map_or(Ok(()), |path| {
                    track(source, mixer, path, false, 0.0, volumes[2], 1.0)
                        .map(|track| voice = Some(track))
                })
            }
            Some(Command::Sound(path, relative_volume)) => {
                if sounds.len() >= 16 {
                    sounds.pop_front();
                }
                track(source, mixer, path, false, 0.0, volumes[1], relative_volume)
                    .map(|track| sounds.push_back(track))
            }
            Some(Command::Volume(next)) => {
                volumes = next;
                Ok(())
            }
            None => Ok(()),
        };
        if let Err(error) = result {
            let _ = outgoing.send(Response::Error(error));
        }
        sounds.retain(|track| !track.player.empty());
        video.update(volumes[1], video_position);
        if requested_video {
            pending_video.fetch_sub(1, Ordering::AcqRel);
        }
        for sound in &sounds {
            sound
                .player
                .set_volume(mixed_volume(volumes[1], sound.volume));
        }
        if let Some(track) = &music {
            let fade_out = track.apply_volume(volumes[0]);
            if !track.repeat && track.player.empty() && track.fade_out.is_none() {
                let _ = outgoing.send(Response::MusicEnded(track.path.clone()));
                music = None;
            } else if fade_out <= 0.0 {
                music = None;
            }
        }
        if let Some(track) = &voice {
            track.player.set_volume(volumes[2]);
        }
        busy.store(
            voice.as_ref().is_some_and(|track| !track.player.empty()),
            Ordering::Release,
        );
        if requested_voice {
            pending_voice.fetch_sub(1, Ordering::AcqRel);
        }
        if voice.as_ref().is_some_and(|track| track.player.empty()) {
            voice = None;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;

    #[test]
    fn relative_volume_multiplies_the_channel_preference() {
        assert!((mixed_volume(0.8, 0.25) - 0.2).abs() < f32::EPSILON);
        assert!(mixed_volume(0.0, 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn long_wav_decodes_incrementally_without_a_device() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("long.wav");
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 8000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..8000 * 60 {
            writer.write_sample(1200_i16).unwrap();
        }
        writer.finalize().unwrap();
        let source = ProjectSource::Directory(root.path().to_owned());
        let reader = BufReader::with_capacity(64 * 1024, source.open_reader("long.wav").unwrap());
        let mut decoder = Decoder::new(reader).unwrap();
        assert_eq!(decoder.total_duration(), Some(Duration::from_secs(60)));
        assert!(decoder.by_ref().take(800).all(|sample| sample > 0.0));
        assert!(decoder.next().is_some());
    }
}
