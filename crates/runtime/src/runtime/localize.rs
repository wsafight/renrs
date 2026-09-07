use super::dialogue::{DialogueRequest, resolve_dialogue};
use super::{InstructionKind, Localizer, Runtime, RuntimeError, WaitState, visible_choice_labels};

impl Runtime {
    pub(super) fn replace_localizer(&mut self, localizer: Localizer) -> Result<(), RuntimeError> {
        // Evaluate first so a bad translation cannot partially change the session.
        let mut dialogue = self.stage.dialogue.clone();
        let mut waiting = self.waiting.clone();
        if matches!(waiting, Some(WaitState::Dialogue)) {
            if let Some(current) = &dialogue
                && let Some(instruction) = self.program.instructions.iter().find(|instruction| {
                    Some(&instruction.statement_id) == current.statement_id.as_ref()
                        && matches!(instruction.kind, InstructionKind::Dialogue { .. })
                })
                && let InstructionKind::Dialogue {
                    speaker,
                    text,
                    translation_id,
                } = &instruction.kind
            {
                dialogue = Some(resolve_dialogue(
                    &self.program,
                    &localizer,
                    &self.variables,
                    DialogueRequest {
                        voice_path: current.voice_path.clone(),
                        statement_id: &instruction.statement_id,
                        speaker: speaker.as_deref(),
                        text,
                        translation_id,
                        line: instruction.span.line,
                    },
                )?);
            }
        } else if matches!(waiting, Some(WaitState::Choice { .. })) {
            let instruction = self
                .program
                .instructions
                .get(self.instruction)
                .ok_or(RuntimeError::InvalidInstruction(self.instruction))?;
            let InstructionKind::Choice { prompt, options } = &instruction.kind else {
                return Err(RuntimeError::InvalidWaitState);
            };
            if let Some(prompt) = prompt {
                dialogue = Some(resolve_dialogue(
                    &self.program,
                    &localizer,
                    &self.variables,
                    DialogueRequest {
                        voice_path: self.stage.voice.clone(),
                        statement_id: &instruction.statement_id,
                        speaker: prompt.speaker.as_deref(),
                        text: &prompt.text,
                        translation_id: &prompt.translation_id,
                        line: instruction.span.line,
                    },
                )?);
            }
            waiting = Some(WaitState::Choice {
                options: visible_choice_labels(
                    options,
                    &self.variables,
                    &localizer,
                    instruction.span.line,
                )?,
            });
        }
        let dialogue_changed = dialogue != self.stage.dialogue;
        self.localizer = localizer;
        self.waiting = waiting;
        if dialogue_changed && let Some(dialogue) = dialogue {
            self.replace_presented_dialogue(dialogue);
        }
        if let Some(checkpoint) = self.rollback.last_mut()
            && let Some(waiting) = &self.waiting
        {
            checkpoint.stage.clone_from(&self.stage);
            checkpoint.waiting.clone_from(waiting);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TranslationCatalog, compile, parse_script};

    #[test]
    fn changing_language_refreshes_dialogue_without_advancing_or_replaying() {
        let script =
            parse_script("label start:\n    @id \"greeting\" \"Hello\"\n", "test.rns").unwrap();
        let mut runtime = Runtime::new(compile(&script).unwrap()).unwrap();
        runtime.advance().unwrap();
        let catalog = TranslationCatalog::from_reader(
            br#"{"language":"fr","messages":{"greeting":"Bonjour"}}"#.as_slice(),
        )
        .unwrap();
        runtime.insert_translation_catalog(catalog).unwrap();
        let before = runtime.snapshot();
        runtime.set_language(Some("fr".to_owned())).unwrap();
        assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Bonjour");
        assert_eq!(runtime.history().len(), before.history.len());
        assert_eq!(runtime.snapshot().instruction, before.instruction);
        assert_eq!(runtime.snapshot().variables, before.variables);
        assert_eq!(runtime.drain_audio_events().count(), 0);
        let restored = Runtime::restore(runtime.program().clone(), runtime.snapshot()).unwrap();
        assert_eq!(restored.stage().dialogue.as_ref().unwrap().text, "Bonjour");
    }

    #[test]
    fn invalid_translation_keeps_previous_language_and_dialogue() {
        let script =
            parse_script("label start:\n    @id \"greeting\" \"Hello\"\n", "test.rns").unwrap();
        let mut runtime = Runtime::new(compile(&script).unwrap()).unwrap();
        runtime.advance().unwrap();
        runtime
            .insert_translation_catalog(
                TranslationCatalog::from_reader(
                    br#"{"language":"fr","messages":{"greeting":"{b}broken"}}"#.as_slice(),
                )
                .unwrap(),
            )
            .unwrap();
        assert!(runtime.set_language(Some("fr".to_owned())).is_err());
        assert_eq!(runtime.localizer().language(), None);
        assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Hello");
    }

    #[test]
    fn changing_language_refreshes_a_menu_prompt_without_an_extra_advance() {
        let script = parse_script(
            "define e = character \"Eileen\"\nlabel start:\n    @id \"question\" menu e \"Choose\":\n        \"A\":\n            return\n        \"B\":\n            return",
            "test.rns",
        )
        .unwrap();
        let mut runtime = Runtime::new(compile(&script).unwrap()).unwrap();
        assert!(matches!(
            runtime.advance().unwrap(),
            WaitState::Choice { .. }
        ));
        let catalog = TranslationCatalog::from_reader(
            br#"{"language":"fr","messages":{"question":"Choisissez"}}"#.as_slice(),
        )
        .unwrap();
        runtime.insert_translation_catalog(catalog).unwrap();
        runtime.set_language(Some("fr".to_owned())).unwrap();
        let prompt = runtime.stage().dialogue.as_ref().unwrap();
        assert_eq!(prompt.speaker_name.as_deref(), Some("Eileen"));
        assert_eq!(prompt.text, "Choisissez");
        assert_eq!(runtime.history().last().unwrap().text, "Choisissez");
    }
}
