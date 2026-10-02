use std::sync::Arc;

use super::{AudioEvent, InstructionKind, Runtime, RuntimeError, WaitState, visible_choices};

impl Runtime {
    /// Selects one option from the currently active choice.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no active choice, the choice index is out
    /// of bounds, or executing the selected branch fails.
    pub fn choose(&mut self, index: usize) -> Result<WaitState, RuntimeError> {
        let Some(WaitState::Choice { options: labels }) = &self.waiting else {
            return Err(RuntimeError::NotChoosing);
        };
        let instruction = self
            .program
            .instructions
            .get(self.instruction)
            .ok_or(RuntimeError::InvalidInstruction(self.instruction))?;
        let InstructionKind::Choice { prompt, options } = &instruction.kind else {
            return Err(RuntimeError::NotChoosing);
        };
        let has_prompt = prompt.is_some();
        let line = instruction.span.line;
        let visible = visible_choices(options, &self.variables, line)?;
        let Some(target) = visible.get(index).map(|option| option.target) else {
            return Err(RuntimeError::InvalidChoice {
                index,
                count: labels.len(),
            });
        };
        if has_prompt && self.stage.voice.is_some() {
            Arc::make_mut(&mut self.stage).voice = None;
            self.audio_events
                .push(AudioEvent::StopVoice { fade_out: 0.0 });
        }
        self.instruction = target;
        self.waiting = None;
        self.advance()
    }
}
