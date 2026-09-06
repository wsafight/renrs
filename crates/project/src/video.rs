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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoStream {
    pub path: String,
    pub seconds: f32,
    pub width: u16,
    pub height: u16,
}

impl VideoClip {
    /// Decodes a portable frame manifest or a bounded streaming video manifest.
    /// # Errors
    /// Rejects invalid rates, unsafe resources and clips longer than 7,200 frames.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, String> {
        let clip: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if clip.audio.as_ref().is_some_and(|path| {
            !crate::resources::visible_path(path)
                || !std::path::Path::new(path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
        }) {
            return Err("video soundtrack must be a safe WAV resource".to_owned());
        }
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
}
