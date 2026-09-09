use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use renrs_syntax::localization::TranslationId;
use renrs_syntax::syntax::{
    AnimationStep, CharacterDef, DefaultDef, Easing, Expr, Position, Span, TransformProperties,
    TransitionKind,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod presentation;
pub mod progress;

pub use presentation::{CompiledImageLayer, CompiledLayeredImage};
pub use progress::{PROGRESS_FILE, ProgressConfig, Unlock};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StatementId(String);

impl StatementId {
    #[doc(hidden)]
    #[must_use]
    pub fn generated(value: String) -> Self {
        Self(value)
    }

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
pub struct InstructionId(String);

impl InstructionId {
    /// Creates an instruction ID for a sidecar alias map.
    ///
    /// # Errors
    ///
    /// IDs must use the engine's `inst_` prefix and ASCII hex payload.
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let valid = value.strip_prefix("inst_").is_some_and(|hash| {
            !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
        if valid {
            Ok(Self(value))
        } else {
            Err(ModelError::InvalidInstructionId(value))
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn generated(value: String) -> Self {
        Self(value)
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
pub struct CompiledProgram {
    pub title: String,
    pub project_id: String,
    pub fingerprint: String,
    pub characters: IndexMap<String, CharacterDef>,
    pub defaults: IndexMap<String, DefaultDef>,
    #[serde(default)]
    pub display_layers: BTreeMap<String, i32>,
    #[serde(default)]
    pub images: IndexMap<String, String>,
    #[serde(default)]
    pub transforms: IndexMap<String, TransformProperties>,
    pub instructions: Vec<Instruction>,
    pub labels: IndexMap<String, usize>,
    pub label_parameters: IndexMap<String, Vec<String>>,
    pub instruction_by_id: HashMap<InstructionId, usize>,
    #[serde(default)]
    pub instruction_aliases: HashMap<InstructionId, InstructionId>,
    #[serde(default)]
    pub explicit_instruction_ids: HashSet<InstructionId>,
}

impl CompiledProgram {
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
    ) -> Result<(), ModelError> {
        if !self.instruction_by_id.contains_key(&current) {
            return Err(ModelError::UnknownAliasTarget(current));
        }
        if self.instruction_by_id.contains_key(&old)
            || self
                .instruction_aliases
                .insert(old.clone(), current)
                .is_some()
        {
            return Err(ModelError::DuplicateInstructionId(old));
        }
        Ok(())
    }
}

/// A compiled script plus the project resources needed by every runtime frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBundle {
    #[serde(flatten)]
    pub compiled: CompiledProgram,
    #[serde(default)]
    pub extensions: BTreeMap<String, String>,
    #[serde(default)]
    pub layered_images: BTreeMap<String, CompiledLayeredImage>,
    #[serde(default)]
    pub progress: ProgressConfig,
}

/// Compatibility name for the runtime's complete project contract.
pub type Program = ProjectBundle;

impl From<CompiledProgram> for ProjectBundle {
    fn from(compiled: CompiledProgram) -> Self {
        Self {
            compiled,
            extensions: BTreeMap::new(),
            layered_images: BTreeMap::new(),
            progress: ProgressConfig::default(),
        }
    }
}

impl Deref for ProjectBundle {
    type Target = CompiledProgram;

    fn deref(&self) -> &Self::Target {
        &self.compiled
    }
}

impl DerefMut for ProjectBundle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.compiled
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
        tracks: Vec<Vec<AnimationStep>>,
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
    Dialogue {
        speaker: Option<String>,
        #[serde(default)]
        attributes: Vec<String>,
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
pub enum ModelError {
    #[error("stable instruction id collision `{0}")]
    DuplicateInstructionId(InstructionId),
    #[error("invalid instruction id `{0}")]
    InvalidInstructionId(String),
    #[error("instruction alias target `{0}` does not exist")]
    UnknownAliasTarget(InstructionId),
}

#[cfg(test)]
mod tests;
