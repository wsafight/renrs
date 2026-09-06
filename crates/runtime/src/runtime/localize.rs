use super::{InstructionKind, Localizer, Runtime, RuntimeError, WaitState};
use super::{execution, interpolate, parse_text_markup, visible_choice_labels};

impl Runtime {
    pub(super) fn replace_localizer(&mut self, localizer: Localizer) -> Result<(), RuntimeError> {
        // Evaluate first so a bad translation cannot partially change the session.
        let mut dialogue = self.stage.dialogue.clone();
        let mut waiting = self.waiting.clone();
        if matches!(waiting, Some(WaitState::Dialogue)) {
            if let Some(current) = &mut dialogue
                && let Some(instruction) = self.program.instructions.iter().find(|instruction| {
                    Some(&instruction.statement_id) == current.statement_id.as_ref()
                        && matches!(instruction.kind, InstructionKind::Dialogue { .. })
                })
                && let InstructionKind::Dialogue {
                    text,
                    translation_id,
                    ..
                } = &instruction.kind
            {
                let text = interpolate(
                    localizer
                        .translate_values(translation_id, text, &self.variables)
                        .map_err(|error| execution(instruction.span.line, error.to_string()))?,
                    &self.variables,
                    instruction.span.line,
                )?;
                let styled = parse_text_markup(&text)
                    .map_err(|error| execution(instruction.span.line, error))?;
                current.text = styled.plain;
                current.runs = styled.runs;
            }
        } else if matches!(waiting, Some(WaitState::Choice { .. })) {
            let instruction = self
                .program
                .instructions
                .get(self.instruction)
                .ok_or(RuntimeError::InvalidInstruction(self.instruction))?;
            let InstructionKind::Choice { options } = &instruction.kind else {
                return Err(RuntimeError::InvalidWaitState);
            };
            waiting = Some(WaitState::Choice {
                options: visible_choice_labels(
                    options,
                    &self.variables,
                    &localizer,
                    instruction.span.line,
                )?,
            });
        }
        self.localizer = localizer;
        self.waiting = waiting;
        if matches!(self.waiting, Some(WaitState::Dialogue)) {
            std::sync::Arc::make_mut(&mut self.stage).dialogue = dialogue;
            if let (Some(last), Some(current)) = (
                std::sync::Arc::make_mut(&mut self.history).last_mut(),
                &self.stage.dialogue,
            ) {
                last.clone_from(current);
            }
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
}
