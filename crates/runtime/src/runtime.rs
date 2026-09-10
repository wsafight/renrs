use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::localization::{LocalizationError, Localizer, TranslationCatalog, TranslationId};
use crate::syntax::{
    BinaryOp, Easing, Expr, Position, TransformState, TransitionKind, UnaryOp, Value,
};
use crate::text::{TextRun, is_text_tag, parse_text_markup};
use renrs_model::{InstructionId, InstructionKind, Program, StatementId};

const MAX_IMMEDIATE_STEPS: usize = 10_000;
const MAX_ROLLBACK_CHECKPOINTS: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageState {
    #[serde(default)]
    pub camera: TransformState,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nvl: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub nvl_start: usize,
    pub background: Option<String>,
    pub sprites: Vec<SpriteState>,
    pub music: Option<MusicState>,
    #[serde(default)]
    pub music_queue: Vec<MusicState>,
    pub voice: Option<String>,
    pub dialogue: Option<DialogueState>,
    #[serde(default = "default_true")]
    pub window: bool,
    #[serde(default)]
    pub shown_screens: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<MusicState>,
    #[serde(default)]
    pub sound_queue: Vec<MusicState>,
}

const fn default_true() -> bool {
    true
}

impl Default for StageState {
    fn default() -> Self {
        Self {
            camera: TransformState::identity(),
            nvl: false,
            nvl_start: 0,
            background: None,
            sprites: Vec::new(),
            music: None,
            music_queue: Vec::new(),
            voice: None,
            dialogue: None,
            window: true,
            shown_screens: Vec::new(),
            sound: None,
            sound_queue: Vec::new(),
        }
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // Required by serde's skip predicate.
fn is_zero(value: &usize) -> bool {
    *value == 0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition: Option<crate::syntax::LayeredImage>,
    pub path: String,
    pub alias: String,
    pub position: Position,
    #[serde(default)]
    pub layer: i32,
    #[serde(default = "master_display_layer")]
    pub display_layer: String,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub transform: TransformState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<String>,
}

fn master_display_layer() -> String {
    "master".to_owned()
}

impl SpriteState {
    #[must_use]
    pub fn image_paths(&self) -> Vec<String> {
        self.composition.as_ref().map_or_else(
            || vec![self.path.clone()],
            |image| {
                image
                    .layers
                    .iter()
                    .flat_map(crate::syntax::ImageLayer::paths)
                    .map(str::to_owned)
                    .collect()
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusicState {
    pub path: String,
    pub repeat: bool,
    #[serde(default)]
    pub fade_in: f32,
    pub volume: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueState {
    #[serde(default)]
    pub statement_id: Option<StatementId>,
    pub speaker_id: Option<String>,
    pub speaker_name: Option<String>,
    pub speaker_color: String,
    #[serde(default)]
    pub translation_id: Option<TranslationId>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_path: Option<String>,
    #[serde(default)]
    pub runs: Vec<TextRun>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_wait: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WaitState {
    Dialogue,
    Choice { options: Vec<String> },
    Pause { seconds: f32 },
    Effect { effect: VisualEffect },
    Screen { name: String },
    Finished,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VisualEffect {
    Parallel {
        from: Box<StageState>,
        tracks: Vec<Vec<crate::syntax::AnimationStep>>,
        seconds: f32,
    },
    Video {
        path: String,
        seconds: f32,
    },
    Dissolve {
        from: Box<StageState>,
        seconds: f32,
    },
    Fade {
        seconds: f32,
    },
    Tween {
        alias: String,
        from: Position,
        to: Position,
        seconds: f32,
    },
    Transform {
        alias: String,
        from: TransformState,
        to: TransformState,
        seconds: f32,
        easing: Easing,
    },
    Push {
        from: Box<StageState>,
        left: bool,
        seconds: f32,
    },
    Wipe {
        from: Box<StageState>,
        left: bool,
        seconds: f32,
    },
    Punch {
        vertical: bool,
        seconds: f32,
    },
}

impl VisualEffect {
    #[must_use]
    pub const fn seconds(&self) -> f32 {
        match self {
            Self::Fade { seconds }
            | Self::Parallel { seconds, .. }
            | Self::Video { seconds, .. }
            | Self::Dissolve { seconds, .. }
            | Self::Tween { seconds, .. }
            | Self::Transform { seconds, .. }
            | Self::Push { seconds, .. }
            | Self::Wipe { seconds, .. }
            | Self::Punch { seconds, .. } => *seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum AudioEvent {
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
        repeat: bool,
    },
    QueueSound {
        path: String,
        volume: f32,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    into = "super::snapshot::Snapshot",
    try_from = "super::snapshot::Snapshot"
)]
pub struct RuntimeSnapshot {
    pub format_version: u32,
    pub program_fingerprint: String,
    pub instruction: usize,
    pub call_stack: Vec<CallFrame>,
    pub instruction_id: Option<InstructionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_instruction_id: Option<InstructionId>,
    pub instruction_is_interaction_anchor: bool,
    pub call_stack_ids: Vec<InstructionId>,
    pub variables: Arc<BTreeMap<String, Value>>,
    pub stage: Arc<StageState>,
    pub waiting: Option<WaitState>,
    #[serde(default)]
    pub language: Option<String>,
    pub history: Arc<Vec<DialogueState>>,
    pub rollback: Vec<RollbackCheckpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackCheckpoint {
    pub instruction: usize,
    pub call_stack: Vec<CallFrame>,
    pub instruction_id: Option<InstructionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_instruction_id: Option<InstructionId>,
    pub instruction_is_interaction_anchor: bool,
    pub call_stack_ids: Vec<InstructionId>,
    pub variables: Arc<BTreeMap<String, Value>>,
    pub stage: Arc<StageState>,
    pub waiting: WaitState,
    pub history_len: usize,
}

impl RuntimeSnapshot {
    pub const FORMAT_VERSION: u32 = 8;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallFrame {
    pub return_address: usize,
    pub previous_variables: BTreeMap<String, Option<Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdAliasResolution {
    pub saved: InstructionId,
    pub current: InstructionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReloadReport {
    pub alias_resolutions: Vec<IdAliasResolution>,
    pub dropped_rollback_checkpoints: usize,
    pub initialized_defaults: Vec<String>,
}

#[derive(Debug)]
pub struct Runtime {
    pub(crate) extensions: renrs_extensions::Extensions,
    pub(crate) program: Arc<Program>,
    pub(crate) instruction: usize,
    pub(crate) profile: crate::progress::Profile,
    pub(crate) profile_revision: u64,
    last_instruction: usize,
    trace: Option<BTreeSet<usize>>,
    call_stack: Vec<CallFrame>,
    pub(crate) variables: Arc<BTreeMap<String, Value>>,
    stage: Arc<StageState>,
    waiting: Option<WaitState>,
    history: Arc<Vec<DialogueState>>,
    pub(crate) rollback: Vec<RollbackCheckpoint>,
    audio_events: Vec<AudioEvent>,
    localizer: Localizer,
    debug: inspect::DebugControl,
    previous_stage: Option<StageState>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("debug execution paused")]
    DebugPaused,
    #[error("program does not define a `start` label")]
    MissingStart,
    #[error("cannot restore save format {found}; this engine supports format {supported}")]
    SaveVersion { found: u32, supported: u32 },
    #[error("save belongs to a different script version; start a new game")]
    ScriptChanged,
    #[error("saved instruction `{0}` no longer exists in this script")]
    SavedInstructionMissing(InstructionId),
    #[error("save has inconsistent stable instruction positions")]
    InvalidStablePositions,
    #[error("saved stage uses removed display layer `{0}`")]
    SavedDisplayLayerMissing(String),
    #[error(
        "cannot hot-reload automatic position `{0}` after a script edit; restart the preview or use an explicit @id"
    )]
    UnstableSavePosition(InstructionId),
    #[error("saved wait state does not match the restored instruction")]
    InvalidWaitState,
    #[error("saved instruction {0} is outside this program")]
    InvalidInstruction(usize),
    #[error("line {line}: {message}")]
    Execution { line: usize, message: String },
    #[error("the runtime is not waiting for a choice")]
    NotChoosing,
    #[error("choice index {index} is outside the available {count} options")]
    InvalidChoice { index: usize, count: usize },
    #[error("there is no earlier interaction to roll back to")]
    CannotRollback,
    #[error("localization error: {0}")]
    Localization(String),
}

mod builtins;
#[path = "runtime/dialogue.rs"]
mod dialogue;
#[cfg(test)]
#[path = "runtime/display_tests.rs"]
mod display_tests;
#[path = "runtime/execute.rs"]
mod execute;
#[cfg(test)]
#[path = "runtime/execute_tests.rs"]
mod execute_tests;
#[path = "runtime/inspect.rs"]
mod inspect;
pub use inspect::DebugState;
#[path = "runtime/localize.rs"]
mod localize;
#[path = "runtime/prediction.rs"]
mod prediction;
#[cfg(test)]
#[path = "runtime/reload_tests.rs"]
mod reload_tests;
#[path = "runtime/restore.rs"]
mod restore;
#[path = "runtime/restore_state.rs"]
mod restore_state;
#[path = "runtime/session.rs"]
mod session;
#[cfg(test)]
#[path = "runtime/sharing_tests.rs"]
mod sharing_tests;
#[path = "runtime/stage.rs"]
mod stage;
#[path = "runtime/value.rs"]
mod value;

#[cfg(test)]
#[path = "runtime/tests.rs"]
mod tests;

/// Expands variable placeholders using the same rules as story dialogue.
///
/// # Errors
/// Returns an error for a missing variable or malformed placeholder.
pub fn format_text(
    text: &str,
    variables: &BTreeMap<String, Value>,
) -> Result<String, RuntimeError> {
    interpolate(text, variables, 0)
}
use restore::{localization_error, stable_call_stack, visible_choice_labels, visible_choices};
use value::{evaluate, execution, interpolate};

impl Runtime {
    /// Evaluates a deterministic expression against current story variables.
    /// # Errors
    /// Reports invalid types, missing variables and exceeded data budgets.
    pub fn evaluate_expression(&self, expression: &Expr) -> Result<Value, RuntimeError> {
        evaluate(expression, &self.variables, 0)
    }
}
