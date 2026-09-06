use std::io::Cursor;

/// Returns the decoded duration of a supported WAV or Ogg Vorbis resource.
///
/// Invalid/unsupported data returns `None`; the playback backend reports the
/// actual decode error when it attempts to load the same bytes.
#[must_use]
pub fn duration_seconds(bytes: &[u8]) -> Option<f32> {
    wav_duration(bytes).or_else(|| ogg_duration(bytes))
}

#[allow(clippy::cast_precision_loss)]
fn wav_duration(bytes: &[u8]) -> Option<f32> {
    let reader = hound::WavReader::new(Cursor::new(bytes)).ok()?;
    let sample_rate = reader.spec().sample_rate;
    (sample_rate > 0).then(|| reader.duration() as f32 / sample_rate as f32)
}

#[allow(clippy::cast_precision_loss)]
fn ogg_duration(bytes: &[u8]) -> Option<f32> {
    let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes)).ok()?;
    let channels = usize::from(reader.ident_hdr.audio_channels);
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    if channels == 0 || sample_rate == 0 {
        return None;
    }
    let mut samples = 0_usize;
    while let Some(packet) = reader.read_dec_packet_itl().ok()? {
        samples = samples.checked_add(packet.len())?;
    }
    Some(samples as f32 / channels as f32 / sample_rate as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_wav_without_playing_it() {
        let mut bytes = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        {
            let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
            for _ in 0..4_000 {
                writer.write_sample(0_i16).unwrap();
            }
            writer.finalize().unwrap();
        }
        let duration = duration_seconds(bytes.get_ref()).unwrap();
        assert!((duration - 0.5).abs() < 0.001);
        assert_eq!(duration_seconds(b"not audio"), None);
    }
}
