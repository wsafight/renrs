use crate::runtime::{
    CallFrame, DialogueState, RollbackCheckpoint, RuntimeSnapshot, StageState, WaitState,
};
use crate::syntax::Value;
use renrs_model::InstructionId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

mod values;
use values::{Encoder, Node, decode};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    format_version: u32,
    program_fingerprint: String,
    current: Checkpoint,
    language: Option<String>,
    history: Arc<Vec<DialogueState>>,
    rollback: Vec<Checkpoint>,
    values: Vec<Node>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    stages: Vec<Arc<StageState>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    instruction: usize,
    call_stack: Vec<SavedCallFrame>,
    instruction_id: Option<InstructionId>,
    last_instruction_id: Option<InstructionId>,
    instruction_is_interaction_anchor: bool,
    call_stack_ids: Vec<InstructionId>,
    variables: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage: Option<Arc<StageState>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage_index: Option<usize>,
    waiting: Option<WaitState>,
    history_len: usize,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedCallFrame {
    return_address: usize,
    previous_variables: BTreeMap<String, Option<usize>>,
}

impl From<RuntimeSnapshot> for Snapshot {
    fn from(snapshot: RuntimeSnapshot) -> Self {
        let mut encoder = Encoder::default();
        let mut stages = StageInterner::default();
        // Format 8 stores stages in a table; format 7 keeps inline stages for checksums.
        let intern = snapshot.format_version >= 8;
        // Stable traversal and content interning make checksums independent of Arc identities.
        let rollback = snapshot
            .rollback
            .into_iter()
            .map(|checkpoint| {
                let call_stack = encode_call_stack(&mut encoder, &checkpoint.call_stack);
                let (stage, stage_index) = intern_stage(&mut stages, intern, checkpoint.stage);
                Checkpoint {
                    instruction: checkpoint.instruction,
                    call_stack,
                    instruction_id: checkpoint.instruction_id,
                    last_instruction_id: checkpoint.last_instruction_id,
                    instruction_is_interaction_anchor: checkpoint.instruction_is_interaction_anchor,
                    call_stack_ids: checkpoint.call_stack_ids,
                    variables: encoder.record(&checkpoint.variables),
                    stage,
                    stage_index,
                    waiting: Some(checkpoint.waiting),
                    history_len: checkpoint.history_len,
                }
            })
            .collect();
        let call_stack = encode_call_stack(&mut encoder, &snapshot.call_stack);
        let (stage, stage_index) = intern_stage(&mut stages, intern, snapshot.stage);
        let current = Checkpoint {
            instruction: snapshot.instruction,
            call_stack,
            instruction_id: snapshot.instruction_id,
            last_instruction_id: snapshot.last_instruction_id,
            instruction_is_interaction_anchor: snapshot.instruction_is_interaction_anchor,
            call_stack_ids: snapshot.call_stack_ids,
            variables: encoder.record(&snapshot.variables),
            stage,
            stage_index,
            waiting: snapshot.waiting,
            history_len: snapshot.history.len(),
        };
        Self {
            format_version: snapshot.format_version,
            program_fingerprint: snapshot.program_fingerprint,
            current,
            language: snapshot.language,
            history: snapshot.history,
            rollback,
            values: encoder.finish(),
            stages: stages.finish(),
        }
    }
}

fn intern_stage(
    stages: &mut StageInterner,
    intern: bool,
    stage: Arc<StageState>,
) -> (Option<Arc<StageState>>, Option<usize>) {
    if intern {
        (None, Some(stages.intern(&stage)))
    } else {
        (Some(stage), None)
    }
}

#[derive(Default)]
struct StageInterner {
    stages: Vec<Arc<StageState>>,
    pointers: HashMap<usize, usize>,
}

impl StageInterner {
    fn intern(&mut self, stage: &Arc<StageState>) -> usize {
        let address = Arc::as_ptr(stage) as usize;
        if let Some(index) = self.pointers.get(&address) {
            return *index;
        }
        if let Some((index, _)) = self
            .stages
            .iter()
            .enumerate()
            .find(|(_, existing)| existing.as_ref() == stage.as_ref())
        {
            self.pointers.insert(address, index);
            return index;
        }
        let index = self.stages.len();
        self.stages.push(stage.clone());
        self.pointers.insert(address, index);
        index
    }

    fn finish(self) -> Vec<Arc<StageState>> {
        self.stages
    }
}

fn checkpoint_stage(
    checkpoint: &Checkpoint,
    stages: &[Arc<StageState>],
) -> Result<Arc<StageState>, String> {
    if let Some(index) = checkpoint.stage_index {
        return stages
            .get(index)
            .cloned()
            .ok_or_else(|| "invalid snapshot stage reference".to_owned());
    }
    checkpoint
        .stage
        .clone()
        .ok_or_else(|| "checkpoint has no stage".to_owned())
}

fn encode_call_stack(encoder: &mut Encoder, call_stack: &[CallFrame]) -> Vec<SavedCallFrame> {
    call_stack
        .iter()
        .map(|frame| SavedCallFrame {
            return_address: frame.return_address,
            previous_variables: frame
                .previous_variables
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        value.as_ref().map(|value| encoder.value(value)),
                    )
                })
                .collect(),
        })
        .collect()
}

fn variables(values: &[Value], index: usize) -> Result<Arc<BTreeMap<String, Value>>, String> {
    match values.get(index) {
        Some(Value::Record(record)) => Ok(record.clone()),
        _ => Err("snapshot variables must reference a record".to_owned()),
    }
}

fn call_stack(values: &[Value], saved: Vec<SavedCallFrame>) -> Result<Vec<CallFrame>, String> {
    if saved.len() > 10_000 {
        return Err("snapshot call stack exceeds its budget".to_owned());
    }
    saved
        .into_iter()
        .map(|frame| {
            let previous_variables = frame
                .previous_variables
                .into_iter()
                .map(|(name, value)| {
                    let value = value
                        .map(|index| {
                            values
                                .get(index)
                                .cloned()
                                .ok_or("invalid snapshot value reference")
                        })
                        .transpose()?;
                    Ok((name, value))
                })
                .collect::<Result<_, String>>()?;
            Ok(CallFrame {
                return_address: frame.return_address,
                previous_variables,
            })
        })
        .collect()
}

impl TryFrom<Snapshot> for RuntimeSnapshot {
    type Error = String;

    fn try_from(snapshot: Snapshot) -> Result<Self, Self::Error> {
        if !(7..=Self::FORMAT_VERSION).contains(&snapshot.format_version) {
            return Err(format!(
                "unsupported snapshot format {}",
                snapshot.format_version
            ));
        }
        if snapshot.rollback.len() > 256 {
            return Err("snapshot exceeds 256 rollback checkpoints".to_owned());
        }
        let values = decode(snapshot.values)?;
        let rollback = snapshot
            .rollback
            .into_iter()
            .map(|checkpoint| {
                if checkpoint.history_len > snapshot.history.len() {
                    return Err("checkpoint history exceeds saved history".to_owned());
                }
                let stage = checkpoint_stage(&checkpoint, &snapshot.stages)?;
                Ok(RollbackCheckpoint {
                    instruction: checkpoint.instruction,
                    call_stack: call_stack(&values, checkpoint.call_stack)?,
                    instruction_id: checkpoint.instruction_id,
                    last_instruction_id: checkpoint.last_instruction_id,
                    instruction_is_interaction_anchor: checkpoint.instruction_is_interaction_anchor,
                    call_stack_ids: checkpoint.call_stack_ids,
                    variables: variables(&values, checkpoint.variables)?,
                    stage,
                    waiting: checkpoint.waiting.ok_or("checkpoint has no wait state")?,
                    history_len: checkpoint.history_len,
                })
            })
            .collect::<Result<_, String>>()?;
        let current = snapshot.current;
        let stage = checkpoint_stage(&current, &snapshot.stages)?;
        Ok(Self {
            format_version: snapshot.format_version,
            program_fingerprint: snapshot.program_fingerprint,
            instruction: current.instruction,
            call_stack: call_stack(&values, current.call_stack)?,
            instruction_id: current.instruction_id,
            last_instruction_id: current.last_instruction_id,
            instruction_is_interaction_anchor: current.instruction_is_interaction_anchor,
            call_stack_ids: current.call_stack_ids,
            variables: variables(&values, current.variables)?,
            stage,
            waiting: current.waiting,
            language: snapshot.language,
            history: snapshot.history,
            rollback,
        })
    }
}

#[cfg(test)]
mod tests;
