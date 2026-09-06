use sha2::{Digest, Sha256};

use crate::localization::TranslationId;

use super::{InstructionId, StatementId};

pub(super) fn stable_statement_id(source: &str, label: &str, path: &str) -> StatementId {
    StatementId(format!(
        "stmt_{}",
        stable_hash("renrs-statement-v1", &[source, label, path])
    ))
}

pub(super) fn stable_anchored_statement_id(anchor: &TranslationId) -> StatementId {
    StatementId(format!(
        "stmt_{}",
        stable_hash("renrs-statement-anchor-v1", &[anchor.as_str()])
    ))
}

pub(super) fn translation_id(
    explicit: Option<&TranslationId>,
    statement_id: &StatementId,
    role: &str,
) -> TranslationId {
    explicit.cloned().unwrap_or_else(|| {
        TranslationId::generated(format!(
            "tr_{}",
            stable_hash("renrs-translation-v1", &[statement_id.as_str(), role])
        ))
    })
}

pub(super) fn stable_instruction_id(statement_id: &StatementId, role: &str) -> InstructionId {
    InstructionId(format!(
        "inst_{}",
        stable_hash("renrs-instruction-v1", &[statement_id.as_str(), role])
    ))
}

fn stable_hash(domain: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, domain.as_bytes());
    for part in parts {
        hash_part(&mut hasher, part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn hash_part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(bytes.len().to_le_bytes());
    hasher.update(bytes);
}
