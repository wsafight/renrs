use crate::compiler::{InstructionKind, Program};
pub use renrs_syntax::localization::*;

#[must_use]
pub fn extract_catalog(program: &Program) -> Vec<TranslationSource> {
    let mut entries = Vec::new();
    for instruction in &program.instructions {
        match &instruction.kind {
            InstructionKind::Dialogue {
                speaker,
                text,
                translation_id,
                ..
            } => entries.push(TranslationSource {
                id: translation_id.clone(),
                text: text.clone(),
                kind: TranslationKind::Dialogue,
                speaker: speaker.clone(),
            }),
            InstructionKind::Choice { prompt, options } => {
                if let Some(prompt) = prompt {
                    entries.push(TranslationSource {
                        id: prompt.translation_id.clone(),
                        text: prompt.text.clone(),
                        kind: TranslationKind::Dialogue,
                        speaker: prompt.speaker.clone(),
                    });
                }
                entries.extend(options.iter().map(|option| TranslationSource {
                    id: option.translation_id.clone(),
                    text: option.text.clone(),
                    kind: TranslationKind::Menu,
                    speaker: None,
                }));
            }
            _ => {}
        }
    }
    entries
}
