use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoClip {
    pub version: u32,
    pub fps: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<VideoStream>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audio_tracks: Vec<VideoAudioTrack>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtitles: Vec<SubtitleTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoStream {
    pub path: String,
    pub seconds: f32,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoAudioTrack {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default = "full_volume")]
    pub volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtitleTrack {
    pub language: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub default: bool,
    pub cues: Vec<SubtitleCue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtitleCue {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

const fn full_volume() -> f32 {
    1.0
}

impl VideoClip {
    /// Decodes a portable frame manifest or a bounded streaming video manifest.
    /// # Errors
    /// Rejects invalid rates, unsafe resources and clips longer than 7,200 frames.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, String> {
        let clip: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if clip.audio.is_some() && !clip.audio_tracks.is_empty() {
            return Err("video must use either audio or audio_tracks, not both".to_owned());
        }
        if clip
            .audio
            .as_ref()
            .is_some_and(|path| !valid_audio_path(path))
        {
            return Err("video soundtrack must be a safe WAV resource".to_owned());
        }
        validate_audio_tracks(&clip.audio_tracks)?;
        if !(1.0..=60.0).contains(&clip.fps) {
            return Err("invalid video version, frame rate or frame count".to_owned());
        }
        match (&clip.stream, clip.version) {
            (None, 1) if !clip.frames.is_empty() && clip.frames.len() <= 7200 => {}
            (Some(stream), 2)
                if clip.frames.is_empty()
                    && crate::resources::visible_path(&stream.path)
                    && std::path::Path::new(&stream.path)
                        .extension()
                        .is_some_and(|ext| {
                            ext.eq_ignore_ascii_case("mp4") || ext.eq_ignore_ascii_case("webm")
                        })
                    && stream.seconds.is_finite()
                    && stream.seconds > 0.0
                    && stream.seconds <= 86400.0
                    && stream.width > 0
                    && stream.width <= 1920
                    && stream.height > 0
                    && stream.height <= 1080 => {}
            _ => return Err("invalid video manifest or stream dimensions".to_owned()),
        }
        if clip
            .frames
            .iter()
            .any(|path| !crate::resources::visible_path(path))
        {
            return Err("unsafe video frame path".to_owned());
        }
        validate_subtitles(&clip.subtitles, clip.duration())?;
        Ok(clip)
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn frame(&self, elapsed: f32) -> usize {
        let count = self.stream.as_ref().map_or(self.frames.len(), |_| {
            (self.duration() * self.fps).ceil() as usize
        });
        ((elapsed.max(0.0) * self.fps) as usize).min(count.saturating_sub(1))
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn duration(&self) -> f32 {
        self.stream.as_ref().map_or_else(
            || self.frames.len() as f32 / self.fps,
            |stream| stream.seconds,
        )
    }

    #[must_use]
    pub fn audio_for(&self, language: Option<&str>) -> Option<(&str, f32)> {
        if let Some(path) = &self.audio {
            return Some((path, 1.0));
        }
        select_language(&self.audio_tracks, language, |track| {
            track.language.as_deref()
        })
        .map(|track| (track.path.as_str(), track.volume))
    }

    #[must_use]
    pub fn subtitle_for(&self, language: Option<&str>) -> Option<&SubtitleTrack> {
        select_language(&self.subtitles, language, |track| {
            Some(track.language.as_str())
        })
    }

    #[must_use]
    pub fn subtitle_at(&self, language: Option<&str>, elapsed: f32) -> Option<&str> {
        self.subtitle_for(language)?
            .cues
            .iter()
            .find(|cue| elapsed >= cue.start && elapsed < cue.end)
            .map(|cue| cue.text.as_str())
    }
}

trait DefaultTrack {
    fn is_default(&self) -> bool;
}

impl DefaultTrack for VideoAudioTrack {
    fn is_default(&self) -> bool {
        self.default
    }
}

impl DefaultTrack for SubtitleTrack {
    fn is_default(&self) -> bool {
        self.default
    }
}

fn select_language<'a, T: DefaultTrack>(
    tracks: &'a [T],
    language: Option<&str>,
    track_language: impl Fn(&T) -> Option<&str>,
) -> Option<&'a T> {
    if let Some(requested) = language.filter(|language| !language.is_empty()) {
        if let Some(track) = tracks.iter().find(|track| {
            track_language(track).is_some_and(|language| language.eq_ignore_ascii_case(requested))
        }) {
            return Some(track);
        }
        let base = requested.split('-').next().unwrap_or(requested);
        if let Some(track) = tracks.iter().find(|track| {
            track_language(track).is_some_and(|language| {
                language.eq_ignore_ascii_case(base)
                    || language
                        .strip_prefix(base)
                        .is_some_and(|suffix| suffix.starts_with('-'))
            })
        }) {
            return Some(track);
        }
    }
    tracks
        .iter()
        .find(|track| track.is_default())
        .or_else(|| tracks.iter().find(|track| track_language(track).is_none()))
        .or_else(|| tracks.first())
}

fn valid_language(language: &str) -> bool {
    !language.is_empty()
        && language.len() <= 35
        && language
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn valid_audio_path(path: &str) -> bool {
    crate::resources::visible_path(path)
        && std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
}

fn validate_audio_tracks(tracks: &[VideoAudioTrack]) -> Result<(), String> {
    if tracks.len() > 16 || tracks.iter().filter(|track| track.default).count() > 1 {
        return Err("video audio_tracks must contain at most 16 tracks and one default".to_owned());
    }
    for track in tracks {
        if !valid_audio_path(&track.path)
            || track
                .language
                .as_deref()
                .is_some_and(|language| !valid_language(language))
            || track.label.as_ref().is_some_and(|label| label.len() > 80)
            || !track.volume.is_finite()
            || !(0.0..=1.0).contains(&track.volume)
        {
            return Err("invalid video audio track".to_owned());
        }
    }
    Ok(())
}

fn validate_subtitles(tracks: &[SubtitleTrack], duration: f32) -> Result<(), String> {
    if tracks.len() > 16 || tracks.iter().filter(|track| track.default).count() > 1 {
        return Err("video subtitles must contain at most 16 tracks and one default".to_owned());
    }
    for track in tracks {
        if !valid_language(&track.language)
            || track.label.as_ref().is_some_and(|label| label.len() > 80)
            || track.cues.len() > 10_000
        {
            return Err("invalid video subtitle track".to_owned());
        }
        let mut previous_end = 0.0;
        for cue in &track.cues {
            if !cue.start.is_finite()
                || !cue.end.is_finite()
                || cue.start < previous_end
                || cue.end <= cue.start
                || cue.end > duration + 0.001
                || cue.text.is_empty()
                || cue.text.len() > 2048
                || cue
                    .text
                    .chars()
                    .any(|character| character.is_control() && character != '\n')
            {
                return Err("invalid or overlapping video subtitle cue".to_owned());
            }
            previous_end = cue.end;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_stream_bounds_and_preserves_frame_manifests() {
        let old =
            VideoClip::from_slice(br#"{"version":1,"fps":24,"frames":["frame.png"]}"#).unwrap();
        assert_eq!(old.frame(10.0), 0);
        let valid = br#"{"version":2,"fps":24,"stream":{"path":"video.mp4","seconds":2,"width":1280,"height":720}}"#;
        let clip = VideoClip::from_slice(valid).unwrap();
        assert_eq!(clip.frame(10.0), 47);
        let mut invalid: serde_json::Value = serde_json::from_slice(valid).unwrap();
        invalid["stream"]["width"] = 8192.into();
        assert!(VideoClip::from_slice(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }

    #[test]
    fn selects_localized_audio_and_subtitle_tracks() {
        let source = br#"{
          "version":2,"fps":24,
          "stream":{"path":"video.mp4","seconds":2,"width":1280,"height":720},
          "audio_tracks":[
            {"path":"audio-en.wav","language":"en","default":true,"volume":0.5},
            {"path":"audio-zh.wav","language":"zh-Hans"}
          ],
          "subtitles":[
            {"language":"en","default":true,"cues":[{"start":0,"end":1,"text":"Hello"}]},
            {"language":"zh","cues":[{"start":0,"end":1,"text":"\u4f60\u597d"}]}
          ]
        }"#;
        let clip = VideoClip::from_slice(source).unwrap();
        assert_eq!(clip.audio_for(Some("zh-CN")), Some(("audio-zh.wav", 1.0)));
        assert_eq!(clip.audio_for(Some("fr")), Some(("audio-en.wav", 0.5)));
        assert_eq!(
            clip.subtitle_at(Some("zh-CN"), 0.5),
            Some("\u{4f60}\u{597d}")
        );
        assert_eq!(clip.subtitle_at(Some("zh-CN"), 1.5), None);
    }

    #[test]
    fn rejects_ambiguous_or_overlapping_tracks() {
        let both = br#"{"version":1,"fps":24,"frames":["frame.png"],"audio":"a.wav","audio_tracks":[{"path":"b.wav"}]}"#;
        assert!(VideoClip::from_slice(both).is_err());
        let overlap = br#"{"version":1,"fps":24,"frames":["frame.png"],"subtitles":[{"language":"en","cues":[{"start":0,"end":0.03,"text":"one"},{"start":0.02,"end":0.04,"text":"two"}]}]}"#;
        assert!(VideoClip::from_slice(overlap).is_err());
    }
}
