use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub use crate::animation::{AnimationStep, validate_tracks};
pub use crate::data::Builtin;
use crate::localization::TranslationId;
pub use crate::presentation::{ImageFrame, ImageLayer, LayeredImage};
pub use crate::transform::{
    CropRect, Easing, Position, TransformProperties, TransformState, TransitionKind,
};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
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
    pub transforms: IndexMap<String, NamedTransform>,
    #[serde(default)]
    pub label_parameters: IndexMap<String, Vec<LabelParameter>>,
    pub labels: IndexMap<String, Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedTransform {
    pub properties: TransformProperties,
    pub span: Span,
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
    Window {
        visible: bool,
    },
    ShowScreen {
        name: String,
    },
    HideScreen {
        name: String,
    },
    CallScreen {
        name: String,
    },
    Repeat {
        count: u32,
    },
    Dialogue {
        speaker: Option<String>,
        #[serde(default)]
        attributes: Vec<String>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at_transform: Option<String>,
    },
    Hide {
        alias: String,
    },
    ClearLayer {
        display_layer: String,
    },
    Menu {
        prompt: Option<MenuPrompt>,
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
        #[serde(default)]
        if_changed: bool,
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
        #[serde(default)]
        repeat: bool,
    },
    QueueSound {
        path: String,
        volume: f32,
        #[serde(default)]
        repeat: bool,
    },
    PlayVoice {
        path: String,
    },
    StopMusic {
        fade_out: f32,
    },
    StopSound {
        fade_out: f32,
    },
    StopVoice {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuPrompt {
    pub speaker: Option<String>,
    pub text: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_display_layers_and_value_type_names_are_stable() {
        assert_eq!(builtin_display_layer_order("master"), Some(0));
        assert_eq!(builtin_display_layer_order("transient"), Some(100));
        assert_eq!(builtin_display_layer_order("screens"), Some(200));
        assert_eq!(builtin_display_layer_order("overlay"), Some(300));
        assert_eq!(builtin_display_layer_order("effects"), None);
        assert_eq!(Value::Integer(1).type_name(), "integer");
        assert_eq!(Value::Boolean(true).type_name(), "boolean");
        assert_eq!(Value::String(String::new()).type_name(), "string");
        assert_eq!(
            Value::List(std::sync::Arc::new(Vec::new())).type_name(),
            "list"
        );
        assert_eq!(
            Value::Record(std::sync::Arc::new(std::collections::BTreeMap::new())).type_name(),
            "record"
        );
    }
}
