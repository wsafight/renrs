use crate::compiler::InstructionId;
use crate::runtime::{DialogueState, RollbackCheckpoint, RuntimeSnapshot, StageState, WaitState};
use crate::syntax::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    instruction: usize,
    call_stack: Vec<usize>,
    instruction_id: Option<InstructionId>,
    last_instruction_id: Option<InstructionId>,
    instruction_is_interaction_anchor: bool,
    call_stack_ids: Vec<InstructionId>,
    variables: usize,
    stage: Arc<StageState>,
    waiting: Option<WaitState>,
    history_len: usize,
}

impl From<RuntimeSnapshot> for Snapshot {
    fn from(snapshot: RuntimeSnapshot) -> Self {
        let mut encoder = Encoder::default();
        // Stable traversal and content interning make checksums independent of Arc identities.
        let rollback = snapshot
            .rollback
            .into_iter()
            .map(|checkpoint| Checkpoint {
                instruction: checkpoint.instruction,
                call_stack: checkpoint.call_stack,
                instruction_id: checkpoint.instruction_id,
                last_instruction_id: checkpoint.last_instruction_id,
                instruction_is_interaction_anchor: checkpoint.instruction_is_interaction_anchor,
                call_stack_ids: checkpoint.call_stack_ids,
                variables: encoder.record(&checkpoint.variables),
                stage: checkpoint.stage,
                waiting: Some(checkpoint.waiting),
                history_len: checkpoint.history_len,
            })
            .collect();
        let current = Checkpoint {
            instruction: snapshot.instruction,
            call_stack: snapshot.call_stack,
            instruction_id: snapshot.instruction_id,
            last_instruction_id: snapshot.last_instruction_id,
            instruction_is_interaction_anchor: snapshot.instruction_is_interaction_anchor,
            call_stack_ids: snapshot.call_stack_ids,
            variables: encoder.record(&snapshot.variables),
            stage: snapshot.stage,
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
        }
    }
}

fn variables(values: &[Value], index: usize) -> Result<Arc<BTreeMap<String, Value>>, String> {
    match values.get(index) {
        Some(Value::Record(record)) => Ok(record.clone()),
        _ => Err("snapshot variables must reference a record".to_owned()),
    }
}

impl TryFrom<Snapshot> for RuntimeSnapshot {
    type Error = String;

    fn try_from(snapshot: Snapshot) -> Result<Self, Self::Error> {
        if snapshot.format_version != Self::FORMAT_VERSION {
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
                Ok(RollbackCheckpoint {
                    instruction: checkpoint.instruction,
                    call_stack: checkpoint.call_stack,
                    instruction_id: checkpoint.instruction_id,
                    last_instruction_id: checkpoint.last_instruction_id,
                    instruction_is_interaction_anchor: checkpoint.instruction_is_interaction_anchor,
                    call_stack_ids: checkpoint.call_stack_ids,
                    variables: variables(&values, checkpoint.variables)?,
                    stage: checkpoint.stage,
                    waiting: checkpoint.waiting.ok_or("checkpoint has no wait state")?,
                    history_len: checkpoint.history_len,
                })
            })
            .collect::<Result<_, String>>()?;
        let current = snapshot.current;
        Ok(Self {
            format_version: snapshot.format_version,
            program_fingerprint: snapshot.program_fingerprint,
            instruction: current.instruction,
            call_stack: current.call_stack,
            instruction_id: current.instruction_id,
            last_instruction_id: current.last_instruction_id,
            instruction_is_interaction_anchor: current.instruction_is_interaction_anchor,
            call_stack_ids: current.call_stack_ids,
            variables: variables(&values, current.variables)?,
            stage: current.stage,
            waiting: current.waiting,
            language: snapshot.language,
            history: snapshot.history,
            rollback,
        })
    }
}

#[cfg(test)]
mod tests;
