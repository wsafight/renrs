use renrs::ProjectSource;
use rodio::{Decoder, Player, Source};
use std::{
    io::BufReader,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

#[derive(Default)]
pub(super) struct VideoAudio {
    path: Option<String>,
    player: Option<Player>,
    offset: f32,
}

impl VideoAudio {
    pub(super) fn sync(
        &mut self,
        source: &ProjectSource,
        mixer: &rodio::mixer::Mixer,
        path: Option<String>,
        position: f32,
        paused: bool,
        volume: f32,
    ) -> Result<(), String> {
        let restart = self.path != path
            || self.player.as_ref().is_some_and(|player| {
                (self.offset + player.get_pos().as_secs_f32() - position).abs() > 0.25
            });
        if restart {
            self.player = None;
            self.path = None;
            if let Some(path) = path {
                let mut decoder = Decoder::new(BufReader::new(
                    source
                        .open_reader(&path)
                        .map_err(|error| error.to_string())?,
                ))
                .map_err(|error| error.to_string())?;
                decoder
                    .try_seek(Duration::from_secs_f32(position.max(0.0)))
                    .map_err(|error| error.to_string())?;
                let player = Player::connect_new(mixer);
                player.pause();
                player.set_volume(volume);
                player.append(decoder);
                self.path = Some(path);
                self.player = Some(player);
                self.offset = position;
            }
        }
        if let Some(player) = &self.player {
            if paused {
                player.pause();
            } else {
                player.play();
            }
        }
        Ok(())
    }
    pub(super) fn update(&self, volume: f32, clock: &AtomicU32) {
        let position = self.player.as_ref().map_or(f32::NAN, |player| {
            player.set_volume(volume);
            if player.empty() {
                f32::MAX
            } else {
                self.offset + player.get_pos().as_secs_f32()
            }
        });
        clock.store(position.to_bits(), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn soundtrack_clock_seeks_pauses_finishes_and_restarts_without_a_device() {
        let root = tempfile::tempdir().unwrap();
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut wav = hound::WavWriter::create(root.path().join("audio.wav"), spec).unwrap();
        for _ in 0..192_000 {
            wav.write_sample(1000_i16).unwrap();
        }
        wav.finalize().unwrap();
        let source = ProjectSource::Directory(root.path().to_owned());
        let (mixer, mut output) =
            rodio::mixer::mixer(2.try_into().unwrap(), 48000.try_into().unwrap());
        let mut audio = VideoAudio::default();
        let clock = AtomicU32::new(f32::NAN.to_bits());
        audio
            .sync(&source, &mixer, Some("audio.wav".into()), 1.0, true, 1.0)
            .unwrap();
        output.by_ref().take(9600).for_each(drop);
        audio.update(1.0, &clock);
        assert!((f32::from_bits(clock.load(Ordering::Acquire)) - 1.0).abs() < 0.01);
        audio
            .sync(&source, &mixer, Some("audio.wav".into()), 1.0, false, 1.0)
            .unwrap();
        output.by_ref().take(19200).for_each(drop);
        audio.update(1.0, &clock);
        assert!((f32::from_bits(clock.load(Ordering::Acquire)) - 1.2).abs() < 0.03);
        output.by_ref().take(100_000).for_each(drop);
        audio.update(1.0, &clock);
        assert_eq!(clock.load(Ordering::Acquire), f32::MAX.to_bits());
        audio.sync(&source, &mixer, None, 0.0, true, 1.0).unwrap();
        audio
            .sync(&source, &mixer, Some("audio.wav".into()), 0.0, true, 1.0)
            .unwrap();
        audio.update(1.0, &clock);
        assert_eq!(clock.load(Ordering::Acquire), 0_f32.to_bits());
    }
}
