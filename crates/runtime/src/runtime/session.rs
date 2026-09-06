use super::{
    AudioEvent, BTreeMap, DialogueState, InstructionId, InstructionKind, Localizer,
    MAX_ROLLBACK_CHECKPOINTS, Program, ReloadReport, RollbackCheckpoint, Runtime, RuntimeError,
    RuntimeSnapshot, StageState, TranslationCatalog, Value, WaitState, localization_error,
    stable_call_stack, visible_choice_labels,
};

impl Runtime {
    #[must_use]
    pub fn snapshot(&self) -> RuntimeSnapshot {
        let (instruction_id, instruction_is_interaction_anchor) = self.snapshot_position();
        RuntimeSnapshot {
            format_version: RuntimeSnapshot::FORMAT_VERSION,
            program_fingerprint: self.program.fingerprint.clone(),
            instruction: self.instruction,
            call_stack: self.call_stack.clone(),
            instruction_id,
            last_instruction_id: self.program.instruction_id(self.last_instruction).cloned(),
            instruction_is_interaction_anchor,
            call_stack_ids: stable_call_stack(&self.program, &self.call_stack),
            variables: self.variables.clone(),
            stage: self.stage.clone(),
            waiting: self.waiting.clone(),
            language: self.localizer.language().map(ToOwned::to_owned),
            history: self.history.clone(),
            rollback: self.rollback.clone(),
        }
    }

    /// Replaces the compiled program while preserving the current session by
    /// mapping every execution position through its stable instruction ID.
    ///
    /// # Errors
    ///
    /// Returns an error without changing the runtime if a structural edit
    /// removed the current instruction or a call-stack return address.
    pub fn reload(&mut self, program: Program) -> Result<(), RuntimeError> {
        self.reload_with_report(program).map(|_| ())
    }

    /// Prepares a reloaded session without changing the current runtime.
    ///
    /// # Errors
    /// Returns an error if execution positions or active menu state cannot be mapped.
    pub fn reloaded(
        &self,
        program: impl Into<std::sync::Arc<Program>>,
    ) -> Result<Self, RuntimeError> {
        self.prepare_reload(program).map(|(runtime, _)| runtime)
    }

    /// Replaces the compiled program and reports any stable-ID alias mappings.
    ///
    /// # Errors
    ///
    /// Returns an error without changing the runtime when session remapping fails.
    pub fn reload_with_report(&mut self, program: Program) -> Result<ReloadReport, RuntimeError> {
        let (restored, report) = self.prepare_reload(program)?;
        *self = restored;
        Ok(report)
    }

    fn prepare_reload(
        &self,
        program: impl Into<std::sync::Arc<Program>>,
    ) -> Result<(Self, ReloadReport), RuntimeError> {
        let localizer = self.localizer.clone();
        let (mut restored, report) = Self::restore_session(program, self.snapshot())?;
        restored.localizer = localizer;
        restored.set_profile(self.profile.clone())?;
        restored.refresh_choice()?;
        Ok((restored, report))
    }

    /// Restores the previous dialogue, choice, or finished interaction.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::CannotRollback`] before a second interaction has
    /// been reached.
    pub fn rollback(&mut self) -> Result<WaitState, RuntimeError> {
        if self.rollback.len() < 2 {
            return Err(RuntimeError::CannotRollback);
        }
        self.rollback.pop();
        let checkpoint = self
            .rollback
            .last()
            .cloned()
            .ok_or(RuntimeError::CannotRollback)?;
        self.instruction = checkpoint.instruction;
        self.last_instruction = checkpoint
            .last_instruction_id
            .as_ref()
            .and_then(|id| self.program.instruction_index(id))
            .unwrap_or_else(|| {
                checkpoint
                    .instruction
                    .saturating_sub(usize::from(checkpoint.instruction_is_interaction_anchor))
            });
        self.call_stack = checkpoint.call_stack;
        self.variables = checkpoint.variables;
        self.restore_persistent_variables();
        self.stage = checkpoint.stage;
        self.waiting = Some(checkpoint.waiting.clone());
        std::sync::Arc::make_mut(&mut self.history).truncate(checkpoint.history_len);
        self.audio_events.clear();
        Ok(checkpoint.waiting)
    }

    #[must_use]
    pub fn can_rollback(&self) -> bool {
        self.rollback.len() >= 2
    }

    #[must_use]
    pub fn stage(&self) -> &StageState {
        &self.stage
    }

    #[must_use]
    pub fn shared_stage(&self) -> std::sync::Arc<StageState> {
        self.stage.clone()
    }

    #[must_use]
    pub fn variables(&self) -> &BTreeMap<String, Value> {
        &self.variables
    }

    #[must_use]
    pub fn history(&self) -> &[DialogueState] {
        &self.history
    }

    #[must_use]
    pub fn waiting(&self) -> Option<&WaitState> {
        self.waiting.as_ref()
    }

    pub fn drain_audio_events(&mut self) -> impl Iterator<Item = AudioEvent> + '_ {
        self.audio_events.drain(..)
    }

    /// Advances the serializable music queue after the frontend reports that
    /// a non-looping track reached its decoded end.
    pub fn complete_music_track(&mut self) {
        if self.stage.music.as_ref().is_some_and(|music| music.repeat) {
            return;
        }
        let stage = std::sync::Arc::make_mut(&mut self.stage);
        stage.music = if stage.music_queue.is_empty() {
            None
        } else {
            Some(stage.music_queue.remove(0))
        };
    }

    #[must_use]
    pub fn program(&self) -> &Program {
        &self.program
    }

    #[must_use]
    pub fn shared_program(&self) -> std::sync::Arc<Program> {
        self.program.clone()
    }

    #[must_use]
    pub fn current_label(&self) -> Option<&str> {
        self.program
            .labels
            .iter()
            .filter(|(_, index)| **index <= self.last_instruction)
            .max_by_key(|(_, index)| **index)
            .map(|(name, _)| name.as_str())
    }

    #[must_use]
    pub const fn localizer(&self) -> &Localizer {
        &self.localizer
    }

    /// Replaces all translation catalogs and refreshes an active menu.
    ///
    /// # Errors
    ///
    /// Returns an execution error if a menu condition or interpolation fails.
    pub fn set_localizer(&mut self, localizer: Localizer) -> Result<(), RuntimeError> {
        self.replace_localizer(localizer)
    }

    /// Installs or replaces one language catalog.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid catalog or active menu expression.
    pub fn insert_translation_catalog(
        &mut self,
        catalog: TranslationCatalog,
    ) -> Result<(), RuntimeError> {
        let mut localizer = self.localizer.clone();
        localizer
            .insert(catalog)
            .map_err(|error| localization_error(&error))?;
        self.replace_localizer(localizer)
    }

    /// Selects a language, or source text when passed `None`.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid language tag or menu expression.
    pub fn set_language(&mut self, language: Option<String>) -> Result<(), RuntimeError> {
        let mut localizer = self.localizer.clone();
        localizer
            .set_language(language)
            .map_err(|error| localization_error(&error))?;
        self.replace_localizer(localizer)
    }

    pub(super) fn refresh_choice(&mut self) -> Result<(), RuntimeError> {
        if !matches!(self.waiting, Some(WaitState::Choice { .. })) {
            return Ok(());
        }
        let instruction = self
            .program
            .instructions
            .get(self.instruction)
            .ok_or(RuntimeError::InvalidInstruction(self.instruction))?;
        let InstructionKind::Choice { options } = &instruction.kind else {
            return Err(RuntimeError::InvalidWaitState);
        };
        let labels = visible_choice_labels(
            options,
            &self.variables,
            &self.localizer,
            instruction.span.line,
        )?;
        self.waiting = Some(WaitState::Choice { options: labels });
        Ok(())
    }

    pub(super) fn set_waiting(&mut self, waiting: WaitState) -> WaitState {
        self.waiting = Some(waiting.clone());
        if matches!(
            waiting,
            WaitState::Dialogue | WaitState::Choice { .. } | WaitState::Finished
        ) {
            self.record_checkpoint();
        }
        waiting
    }

    pub(super) fn record_checkpoint(&mut self) {
        let call_stack_ids = stable_call_stack(&self.program, &self.call_stack);
        let Some(waiting) = self.waiting.clone() else {
            return;
        };
        let (instruction_id, instruction_is_interaction_anchor) = self.snapshot_position();
        self.rollback.push(RollbackCheckpoint {
            instruction: self.instruction,
            call_stack: self.call_stack.clone(),
            instruction_id,
            last_instruction_id: self.program.instruction_id(self.last_instruction).cloned(),
            instruction_is_interaction_anchor,
            call_stack_ids,
            variables: self.variables.clone(),
            stage: self.stage.clone(),
            waiting,
            history_len: self.history.len(),
        });
        if self.rollback.len() > MAX_ROLLBACK_CHECKPOINTS {
            self.rollback.remove(0);
        }
    }

    fn snapshot_position(&self) -> (Option<InstructionId>, bool) {
        if matches!(
            self.waiting,
            Some(WaitState::Pause { .. } | WaitState::Effect { .. })
        ) {
            return (
                self.program.instruction_id(self.last_instruction).cloned(),
                true,
            );
        }
        if matches!(self.waiting, Some(WaitState::Dialogue))
            && let Some(statement_id) = self
                .stage
                .dialogue
                .as_ref()
                .and_then(|dialogue| dialogue.statement_id.as_ref())
            && let Some(instruction) =
                self.program
                    .instructions
                    .get(self.last_instruction)
                    .filter(|instruction| {
                        &instruction.statement_id == statement_id
                            && matches!(instruction.kind, InstructionKind::Dialogue { .. })
                    })
        {
            return (Some(instruction.id.clone()), true);
        }
        (
            self.program.instruction_id(self.instruction).cloned(),
            false,
        )
    }
}
