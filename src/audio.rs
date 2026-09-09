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
    let reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes)).ok()?;
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    if sample_rate == 0 {
        return None;
    }
    let granule = last_ogg_granule(bytes)?;
    (granule > 0).then(|| granule as f32 / sample_rate as f32)
}

fn last_ogg_granule(bytes: &[u8]) -> Option<i64> {
    let mut offset = 0;
    let mut granule = None;
    while offset + 27 <= bytes.len() {
        if bytes[offset..offset + 4] != *b"OggS" || bytes[offset + 4] != 0 {
            offset += 1;
            continue;
        }
        let segments = usize::from(bytes[offset + 26]);
        if offset + 27 + segments > bytes.len() {
            break;
        }
        let body: usize = bytes[offset + 27..offset + 27 + segments]
            .iter()
            .map(|size| usize::from(*size))
            .sum();
        let value = i64::from_le_bytes(bytes[offset + 6..offset + 14].try_into().ok()?);
        if value >= 0 {
            granule = Some(value);
        }
        let Some(next) = offset.checked_add(27 + segments + body) else {
            break;
        };
        offset = next;
    }
    granule
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

    #[test]
    fn reads_ogg_granule_without_decoding_packets() {
        let mut page = vec![0_u8; 27];
        page[..4].copy_from_slice(b"OggS");
        page[6..14].copy_from_slice(48_000_i64.to_le_bytes().as_ref());
        page[26] = 0;
        assert_eq!(last_ogg_granule(&page), Some(48_000));
        assert_eq!(last_ogg_granule(b"not ogg"), None);
    }
}
