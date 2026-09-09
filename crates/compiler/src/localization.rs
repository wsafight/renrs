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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, parse_script};

    #[test]
    fn extracts_dialogue_prompts_and_menu_choices() {
        let program = compile(
            &parse_script(
                "define e = character \"Eileen\"\nlabel start:\n    @id \"hello\" e \"Hi\"\n    menu \"Choose\":\n        \"Stay\":\n            return\n        \"Leave\":\n            return\n",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
        let entries = extract_catalog(&program);
        assert!(entries.iter().any(|entry| {
            entry.kind == TranslationKind::Dialogue
                && entry.speaker.as_deref() == Some("e")
                && entry.text == "Hi"
        }));
        assert!(
            entries
                .iter()
                .any(|entry| entry.kind == TranslationKind::Dialogue && entry.text == "Choose")
        );
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.kind == TranslationKind::Menu)
                .count(),
            2
        );
    }
}
