use std::collections::{HashMap, HashSet};
use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::localization::TranslationId;
use crate::syntax::{
    CharacterDef, DefaultDef, Easing, Expr, Position, Span, TransformProperties, TransitionKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StatementId(pub(super) String);

impl StatementId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StatementId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstructionId(pub(super) String);

impl InstructionId {
    /// Creates an instruction ID for a sidecar alias map.
    ///
    /// # Errors
    ///
    /// IDs must use the engine's `inst_` prefix and ASCII hex payload.
    pub fn new(value: impl Into<String>) -> Result<Self, CompileError> {
        let value = value.into();
        let valid = value.strip_prefix("inst_").is_some_and(|hash| {
            !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
        if valid {
            Ok(Self(value))
        } else {
            Err(CompileError::InvalidInstructionId(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InstructionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    #[serde(default)]
    pub extensions: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub layered_images: std::collections::BTreeMap<String, crate::syntax::LayeredImage>,
    #[serde(default)]
    pub progress: crate::progress::ProgressConfig,
    pub title: String,
    pub project_id: String,
    pub fingerprint: String,
    pub characters: IndexMap<String, CharacterDef>,
    pub defaults: IndexMap<String, DefaultDef>,
    #[serde(default)]
    pub display_layers: std::collections::BTreeMap<String, i32>,
    pub instructions: Vec<Instruction>,
    pub labels: IndexMap<String, usize>,
    pub label_parameters: IndexMap<String, Vec<String>>,
    pub instruction_by_id: HashMap<InstructionId, usize>,
    #[serde(default)]
    pub instruction_aliases: HashMap<InstructionId, InstructionId>,
    #[serde(default)]
    pub explicit_instruction_ids: HashSet<InstructionId>,
}

impl Program {
    #[must_use]
    pub fn instruction_index(&self, id: &InstructionId) -> Option<usize> {
        let canonical = self.instruction_aliases.get(id).unwrap_or(id);
        self.instruction_by_id.get(canonical).copied()
    }

    #[must_use]
    pub fn instruction_id(&self, index: usize) -> Option<&InstructionId> {
        self.instructions
            .get(index)
            .map(|instruction| &instruction.id)
    }

    #[must_use]
    pub fn canonical_instruction_id<'a>(
        &'a self,
        id: &'a InstructionId,
    ) -> Option<&'a InstructionId> {
        if self.instruction_by_id.contains_key(id) {
            Some(id)
        } else {
            self.instruction_aliases.get(id)
        }
    }

    /// Registers an old stable ID as an alias for a current instruction.
    ///
    /// # Errors
    ///
    /// Returns an error when the target is absent or the alias conflicts.
    pub fn add_instruction_alias(
        &mut self,
        old: InstructionId,
        current: InstructionId,
    ) -> Result<(), CompileError> {
        if !self.instruction_by_id.contains_key(&current) {
            return Err(CompileError::UnknownAliasTarget(current));
        }
        if self.instruction_by_id.contains_key(&old)
            || self
                .instruction_aliases
                .insert(old.clone(), current)
                .is_some()
        {
            return Err(CompileError::DuplicateInstructionId(old));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub id: InstructionId,
    pub statement_id: StatementId,
    pub role: String,
    pub span: Span,
    pub kind: InstructionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstructionKind {
    Extension {
        name: String,
        variable: String,
        input: Expr,
    },
    Nvl {
        mode: String,
    },
    Parallel {
        tracks: Vec<Vec<crate::syntax::AnimationStep>>,
    },
    Video {
        path: String,
        seconds: f32,
    },
    Dialogue {
        speaker: Option<String>,
        text: String,
        translation_id: TranslationId,
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
        display_order: i32,
    },
    Hide {
        alias: String,
    },
    ClearLayer {
        display_layer: String,
    },
    Choice {
        prompt: Option<ChoicePrompt>,
        options: Vec<ChoiceTarget>,
    },
    Jump {
        target: usize,
    },
    Call {
        target: usize,
        arguments: Vec<Expr>,
        parameters: Vec<String>,
    },
    Return {
        value: Option<Expr>,
    },
    Set {
        variable: String,
        value: Expr,
    },
    JumpIfFalse {
        condition: Expr,
        target: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceTarget {
    pub text: String,
    pub target: usize,
    pub translation_id: TranslationId,
    pub condition: Option<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoicePrompt {
    pub speaker: Option<String>,
    pub text: String,
    pub translation_id: TranslationId,
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("unknown label `{label}` referenced at {file}:{line}")]
    UnknownLabel {
        label: String,
        file: String,
        line: usize,
    },
    #[error("stable instruction id collision `{0}")]
    DuplicateInstructionId(InstructionId),
    #[error("invalid instruction id `{0}")]
    InvalidInstructionId(String),
    #[error("instruction alias target `{0}` does not exist")]
    UnknownAliasTarget(InstructionId),
    #[error("translation id `{0}` is used more than once")]
    DuplicateTranslationId(TranslationId),
    #[error("unknown image `{name}` referenced at {file}:{line}")]
    UnknownImage {
        name: String,
        file: String,
        line: usize,
    },
    #[error("unknown display layer `{name}` referenced at {file}:{line}")]
    UnknownDisplayLayer {
        name: String,
        file: String,
        line: usize,
    },
    #[error(
        "label `{label}` accepts at most {maximum} positional arguments but received {found} at {file}:{line}"
    )]
    TooManyLabelArguments {
        label: String,
        maximum: usize,
        found: usize,
        file: String,
        line: usize,
    },
    #[error("label `{label}` has no parameter named `{argument}` at {file}:{line}")]
    UnknownLabelArgument {
        label: String,
        argument: String,
        file: String,
        line: usize,
    },
    #[error("label `{label}` receives parameter `{argument}` more than once at {file}:{line}")]
    DuplicateLabelArgument {
        label: String,
        argument: String,
        file: String,
        line: usize,
    },
    #[error("label `{label}` is missing required argument `{argument}` at {file}:{line}")]
    MissingLabelArgument {
        label: String,
        argument: String,
        file: String,
        line: usize,
    },
    #[error("entry label `start` cannot declare parameters at {file}:{line}")]
    ParameterizedStart { file: String, line: usize },
    #[error("jump cannot enter parameterized label `{label}` at {file}:{line}; use `call`")]
    ParameterizedJump {
        label: String,
        file: String,
        line: usize,
    },
    #[error("failed to fingerprint script: {0}")]
    Fingerprint(serde_json::Error),
}
