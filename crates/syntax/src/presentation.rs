use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayeredImage {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<ImageLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageLayer {
    pub path: String,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub frames: Vec<ImageFrame>,
    #[serde(default)]
    pub speaking: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageFrame {
    pub path: String,
    pub seconds: f32,
}

impl ImageLayer {
    #[must_use]
    pub fn frame(&self, elapsed: f64, speaking: bool) -> &str {
        if self.frames.is_empty() || self.speaking && !speaking {
            return &self.path;
        }
        let duration: f64 = self
            .frames
            .iter()
            .map(|frame| f64::from(frame.seconds))
            .sum();
        let mut remaining = elapsed.rem_euclid(duration);
        for frame in &self.frames {
            if remaining < f64::from(frame.seconds) {
                return &frame.path;
            }
            remaining -= f64::from(frame.seconds);
        }
        &self.path
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.path.as_str())
            .chain(self.frames.iter().map(|frame| frame.path.as_str()))
    }
}
