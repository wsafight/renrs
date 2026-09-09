use crate::syntax::{Block, Statement, StatementKind};

use super::ids::translation_id;
use super::{InstructionKind, StatementId};

pub(super) fn unroll_timeline(block: &Block) -> Result<Block, String> {
    let mut output = Vec::new();
    let mut body = Vec::new();
    for statement in block {
        if let StatementKind::Repeat { count } = statement.kind {
            if body.is_empty() {
                return Err("repeat requires preceding timeline steps".to_owned());
            }
            for _ in 0..count {
                output.extend(body.iter().cloned());
            }
            body.clear();
        } else {
            body.push(statement.clone());
        }
    }
    output.extend(body);
    Ok(output)
}

pub(super) fn dialogue_kind(
    statement: &Statement,
    statement_id: &StatementId,
    speaker: Option<String>,
    attributes: Vec<String>,
    text: String,
) -> InstructionKind {
    InstructionKind::Dialogue {
        speaker,
        attributes,
        text,
        translation_id: translation_id(statement.id.as_ref(), statement_id, "dialogue"),
    }
}
