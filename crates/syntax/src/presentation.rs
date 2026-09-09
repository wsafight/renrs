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

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(speaking: bool) -> ImageLayer {
        ImageLayer {
            path: "idle.png".to_owned(),
            when: None,
            x: 0.0,
            y: 0.0,
            frames: vec![
                ImageFrame {
                    path: "a.png".to_owned(),
                    seconds: 0.5,
                },
                ImageFrame {
                    path: "b.png".to_owned(),
                    seconds: 0.5,
                },
            ],
            speaking,
        }
    }

    #[test]
    fn selects_cycled_frames_and_keeps_idle_when_not_speaking() {
        let talking = layer(true);
        assert_eq!(talking.frame(0.2, true), "a.png");
        assert_eq!(talking.frame(0.6, true), "b.png");
        assert_eq!(talking.frame(1.2, true), "a.png");
        assert_eq!(talking.frame(0.6, false), "idle.png");
        assert_eq!(
            talking.paths().collect::<Vec<_>>(),
            ["idle.png", "a.png", "b.png"]
        );

        let still = ImageLayer {
            frames: Vec::new(),
            speaking: false,
            ..layer(false)
        };
        assert_eq!(still.frame(12.0, true), "idle.png");
    }
}
