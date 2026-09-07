use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub use crate::animation::{AnimationStep, validate_tracks};
pub use crate::data::Builtin;
use crate::localization::TranslationId;
pub use crate::presentation::{ImageFrame, ImageLayer, LayeredImage};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub source: String,
    pub line: usize,
    pub column: usize,
}

impl Span {
    #[must_use]
    pub fn new(line: usize, column: usize) -> Self {
        Self::in_source("", line, column)
    }

    #[must_use]
    pub fn in_source(source: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            source: source.into(),
            line,
            column,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterDef {
    pub name: String,
    pub color: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Script {
    pub source_name: String,
    pub title: String,
    #[serde(default)]
    pub project_id: String,
    pub characters: IndexMap<String, CharacterDef>,
    #[serde(default)]
    pub defaults: IndexMap<String, DefaultDef>,
    #[serde(default)]
    pub images: IndexMap<String, ImageDef>,
    #[serde(default)]
    pub display_layers: IndexMap<String, DisplayLayerDef>,
    #[serde(default)]
    pub label_parameters: IndexMap<String, Vec<LabelParameter>>,
    pub labels: IndexMap<String, Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultDef {
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelParameter {
    pub name: String,
    #[serde(default)]
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallArgument {
    #[serde(default)]
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageDef {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayLayerDef {
    pub order: i32,
    pub span: Span,
}

#[must_use]
pub fn builtin_display_layer_order(name: &str) -> Option<i32> {
    match name {
        "master" => Some(0),
        "transient" => Some(100),
        "screens" => Some(200),
        "overlay" => Some(300),
        _ => None,
    }
}

pub type Block = Vec<Statement>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statement {
    pub span: Span,
    #[serde(default)]
    pub id: Option<TranslationId>,
    #[serde(default)]
    pub aliases: Vec<TranslationId>,
    pub kind: StatementKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatementKind {
    Extension {
        name: String,
        variable: String,
        input: Expr,
    },
    Nvl {
        mode: String,
    },
    Parallel {
        tracks: Vec<Vec<AnimationStep>>,
    },
    Timeline {
        block: Block,
    },
    Video {
        path: String,
        seconds: f32,
    },
    Dialogue {
        speaker: Option<String>,
        text: String,
    },
    Scene {
        path: String,
    },
    Show {
        path: String,
        alias: String,
        position: Position,
        layer: i32,
        display_layer: String,
    },
    Hide {
        alias: String,
    },
    ClearLayer {
        display_layer: String,
    },
    Menu {
        options: Vec<MenuOption>,
    },
    Jump {
        label: String,
    },
    Call {
        label: String,
        #[serde(default)]
        arguments: Vec<CallArgument>,
    },
    Return {
        value: Option<Expr>,
    },
    Set {
        variable: String,
        value: Expr,
    },
    If {
        branches: Vec<(Expr, Block)>,
        else_block: Block,
    },
    PlayMusic {
        path: String,
        repeat: bool,
        fade_in: f32,
        volume: f32,
    },
    QueueMusic {
        path: String,
        repeat: bool,
        fade_in: f32,
        volume: f32,
    },
    PlaySound {
        path: String,
        volume: f32,
    },
    PlayVoice {
        path: String,
    },
    StopMusic {
        fade_out: f32,
    },
    Pause {
        seconds: f32,
    },
    Move {
        alias: String,
        position: Position,
        seconds: f32,
    },
    Transform {
        alias: String,
        properties: TransformProperties,
        seconds: f32,
        easing: Easing,
    },
    Transition {
        kind: TransitionKind,
        seconds: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuOption {
    pub text: String,
    pub block: Block,
    pub span: Span,
    #[serde(default)]
    pub id: Option<TranslationId>,
    #[serde(default)]
    pub condition: Option<Expr>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionKind {
    Fade,
    Dissolve,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformState {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub rotation: f32,
    pub alpha: f32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub crop: Option<CropRect>,
}

impl Default for TransformState {
    fn default() -> Self {
        Self::identity()
    }
}

impl TransformState {
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            rotation: 0.0,
            alpha: 1.0,
            anchor_x: 0.0,
            anchor_y: 1.0,
            crop: None,
        }
    }

    #[must_use]
    pub fn interpolate(self, target: Self, progress: f32) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        Self {
            x: lerp(self.x, target.x, progress),
            y: lerp(self.y, target.y, progress),
            scale: lerp(self.scale, target.scale, progress),
            rotation: lerp(self.rotation, target.rotation, progress),
            alpha: lerp(self.alpha, target.alpha, progress),
            anchor_x: lerp(self.anchor_x, target.anchor_x, progress),
            anchor_y: lerp(self.anchor_y, target.anchor_y, progress),
            crop: match (self.crop, target.crop) {
                (Some(from), Some(to)) => Some(from.interpolate(to, progress)),
                (from, to) => {
                    if progress < 1.0 {
                        from
                    } else {
                        to
                    }
                }
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TransformProperties {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub scale: Option<f32>,
    pub rotation: Option<f32>,
    pub alpha: Option<f32>,
    pub anchor: Option<(f32, f32)>,
    pub crop: Option<Option<CropRect>>,
}

impl TransformProperties {
    #[must_use]
    pub fn apply(self, mut state: TransformState) -> TransformState {
        if let Some(value) = self.x {
            state.x = value;
        }
        if let Some(value) = self.y {
            state.y = value;
        }
        if let Some(value) = self.scale {
            state.scale = value;
        }
        if let Some(value) = self.rotation {
            state.rotation = value;
        }
        if let Some(value) = self.alpha {
            state.alpha = value;
        }
        if let Some((x, y)) = self.anchor {
            state.anchor_x = x;
            state.anchor_y = y;
        }
        if let Some(value) = self.crop {
            state.crop = value;
        }
        state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl CropRect {
    fn interpolate(self, target: Self, progress: f32) -> Self {
        Self {
            x: lerp(self.x, target.x, progress),
            y: lerp(self.y, target.y, progress),
            width: lerp(self.width, target.width, progress),
            height: lerp(self.height, target.height, progress),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Easing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    #[must_use]
    pub fn sample(self, progress: f32) -> f32 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => progress,
            Self::EaseIn => progress * progress,
            Self::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            Self::EaseInOut if progress < 0.5 => 2.0 * progress * progress,
            Self::EaseInOut => 1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0,
        }
    }
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Invoke {
        function: Builtin,
        arguments: Vec<Expr>,
    },
    Value(Value),
    Variable(String),
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Integer(i64),
    Boolean(bool),
    String(String),
    List(std::sync::Arc<Vec<Value>>),
    Record(std::sync::Arc<std::collections::BTreeMap<String, Value>>),
}

impl Value {
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "integer",
            Self::Boolean(_) => "boolean",
            Self::String(_) => "string",
            Self::List(_) => "list",
            Self::Record(_) => "record",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}
