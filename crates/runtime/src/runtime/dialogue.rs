use std::collections::BTreeMap;
use std::sync::Arc;

use super::{
    DialogueState, Localizer, Program, Runtime, RuntimeError, StatementId, TranslationId, Value,
    execution, interpolate, parse_text_markup,
};

pub(super) struct DialogueRequest<'a> {
    pub(super) voice_path: Option<String>,
    pub(super) statement_id: &'a StatementId,
    pub(super) speaker: Option<&'a str>,
    pub(super) text: &'a str,
    pub(super) translation_id: &'a TranslationId,
    pub(super) line: usize,
}

pub(super) fn resolve_dialogue(
    program: &Program,
    localizer: &Localizer,
    variables: &BTreeMap<String, Value>,
    request: DialogueRequest<'_>,
) -> Result<DialogueState, RuntimeError> {
    let (speaker_name, speaker_color) = if let Some(id) = request.speaker {
        let character = program
            .characters
            .get(id)
            .ok_or_else(|| execution(request.line, format!("unknown character `{id}`")))?;
        (Some(character.name.clone()), character.color.clone())
    } else {
        (None, "#f4f4f5".to_owned())
    };
    let translated = localizer
        .translate_values(request.translation_id, request.text, variables)
        .map_err(|error| execution(request.line, error.to_string()))?;
    let interpolated = interpolate(translated, variables, request.line)?;
    let styled =
        parse_text_markup(&interpolated).map_err(|message| execution(request.line, message))?;
    Ok(DialogueState {
        voice_path: request.voice_path,
        statement_id: Some(request.statement_id.clone()),
        speaker_id: request.speaker.map(str::to_owned),
        speaker_name,
        speaker_color,
        translation_id: Some(request.translation_id.clone()),
        text: styled.plain,
        runs: styled.runs,
    })
}

impl Runtime {
    pub(super) fn present_dialogue(
        &mut self,
        statement_id: &StatementId,
        speaker: Option<&str>,
        text: &str,
        translation_id: &TranslationId,
        line: usize,
    ) -> Result<(), RuntimeError> {
        if self.stage.nvl && self.history.len().saturating_sub(self.stage.nvl_start) >= 256 {
            return Err(execution(
                line,
                "NVL page exceeds 256 paragraphs; use nvl clear",
            ));
        }
        let dialogue = resolve_dialogue(
            &self.program,
            &self.localizer,
            &self.variables,
            DialogueRequest {
                voice_path: self.stage.voice.clone(),
                statement_id,
                speaker,
                text,
                translation_id,
                line,
            },
        )?;
        Arc::make_mut(&mut self.stage).dialogue = Some(dialogue.clone());
        Arc::make_mut(&mut self.history).push(dialogue);
        Ok(())
    }

    pub(super) fn replace_presented_dialogue(&mut self, dialogue: DialogueState) {
        if let Some(last) = Arc::make_mut(&mut self.history).last_mut()
            && last.statement_id == dialogue.statement_id
        {
            last.clone_from(&dialogue);
        }
        Arc::make_mut(&mut self.stage).dialogue = Some(dialogue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WaitState, compile, parse_script};

    #[test]
    fn unprompted_choice_clears_current_dialogue_but_keeps_history() {
        let script = parse_script(
            "label start:\n    \"Before\"\n    menu:\n        \"One\":\n            return\n        \"Two\":\n            return",
            "test.rns",
        )
        .unwrap();
        let mut runtime = Runtime::new(compile(&script).unwrap()).unwrap();
        assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
        assert!(matches!(
            runtime.continue_story().unwrap(),
            WaitState::Choice { .. }
        ));
        assert!(runtime.stage().dialogue.is_none());
        assert_eq!(runtime.history().len(), 1);
        assert_eq!(runtime.history()[0].text, "Before");
    }

    #[test]
    fn screen_variables_refresh_menu_prompts_and_their_checkpoint() {
        let script = parse_script(
            "default score = 0\nlabel start:\n    menu \"Score {score}\":\n        \"One\":\n            \"One\"\n        \"Two\":\n            \"Two\"",
            "test.rns",
        )
        .unwrap();
        let mut runtime = Runtime::new(compile(&script).unwrap()).unwrap();
        assert!(matches!(
            runtime.advance().unwrap(),
            WaitState::Choice { .. }
        ));
        runtime
            .set_screen_variable("score", Value::Integer(1))
            .unwrap();
        assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Score 1");
        assert_eq!(runtime.history().last().unwrap().text, "Score 1");

        assert_eq!(runtime.choose(0).unwrap(), WaitState::Dialogue);
        assert!(matches!(
            runtime.rollback().unwrap(),
            WaitState::Choice { .. }
        ));
        assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Score 1");
    }
}
