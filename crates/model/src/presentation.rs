use renrs_syntax::presentation::{ImageFrame, ImageLayer, LayeredImage};
use renrs_syntax::syntax::Expr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledLayeredImage {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<CompiledImageLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledImageLayer {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Expr>,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub frames: Vec<ImageFrame>,
    #[serde(default)]
    pub speaking: bool,
}

impl CompiledLayeredImage {
    #[must_use]
    pub fn resolved(&self, layers: Vec<ImageLayer>) -> LayeredImage {
        LayeredImage {
            width: self.width,
            height: self.height,
            layers,
        }
    }
}

impl CompiledImageLayer {
    #[must_use]
    pub fn source_layer(&self) -> ImageLayer {
        ImageLayer {
            path: self.path.clone(),
            when: None,
            x: self.x,
            y: self.y,
            frames: self.frames.clone(),
            speaking: self.speaking,
        }
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.path.as_str())
            .chain(self.frames.iter().map(|frame| frame.path.as_str()))
    }
}
