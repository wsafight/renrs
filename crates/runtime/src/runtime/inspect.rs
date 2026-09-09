use super::{
    BTreeMap, InstructionId, InstructionKind, Runtime, RuntimeError, TranslationId, Value,
    WaitState, visible_choices,
};
use crate::syntax::{Expr, Span};
use serde::Serialize;

#[derive(Debug, Default)]
pub(super) struct DebugControl {
    breakpoints: std::collections::HashSet<InstructionId>,
    budget: Option<usize>,
    paused: bool,
    skip_once: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugState {
    pub paused: bool,
    pub next_instruction: usize,
    pub label: Option<String>,
    pub instruction: Option<InstructionId>,
    pub location: Option<Span>,
    pub call_stack: Vec<Span>,
    pub variables: std::sync::Arc<BTreeMap<String, Value>>,
    pub waiting: Option<WaitState>,
}

impl Runtime {
    /// Applies a deterministic expression to a declared screen variable.
    /// # Errors
    /// Rejects non-interactive story states, unsupported functions and invalid values.
    pub fn apply_screen_expression(
        &mut self,
        name: &str,
        expression: &Expr,
    ) -> Result<(), RuntimeError> {
        if !matches!(
            self.waiting(),
            Some(WaitState::Dialogue | WaitState::Choice { .. } | WaitState::Screen { .. })
        ) {
            return Err(super::execution(
                0,
                "screen expressions require dialogue or choices",
            ));
        }
        let value = super::value::evaluate(expression, &self.variables, 0)?;
        self.set_screen_variable(name, value)
    }
    /// Updates a declared variable from a screen and includes it in the current checkpoint.
    /// # Errors
    /// Rejects undeclared variables and type changes.
    pub fn set_screen_variable(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        value
            .validate_data()
            .map_err(|error| super::execution(0, error))?;
        let Some(previous) = self.variables.get(name) else {
            return Err(super::execution(
                0,
                format!("unknown screen variable {name}"),
            ));
        };
        if previous.type_name() != value.type_name() {
            return Err(super::execution(0, "screen variable type mismatch"));
        }
        let previous = self.variables.clone();
        std::sync::Arc::make_mut(&mut self.variables).insert(name.to_owned(), value);
        if let Err(error) = self.refresh_choice() {
            self.variables = previous;
            return Err(error);
        }
        if name.starts_with("persistent_") {
            let value = self.variables[name].clone();
            self.set_persistent_variable(name, &value);
        }
        if let Some(checkpoint) = self.rollback.last_mut() {
            checkpoint.variables = self.variables.clone();
            checkpoint.stage.clone_from(&self.stage);
            if let Some(waiting) = &self.waiting {
                checkpoint.waiting.clone_from(waiting);
            }
        }
        Ok(())
    }
    /// Replaces debugger breakpoints using validated instruction IDs.
    /// # Errors
    /// Rejects IDs absent from the current program.
    pub fn set_breakpoints(&mut self, ids: Vec<InstructionId>) -> Result<(), RuntimeError> {
        for id in &ids {
            if self.program.instruction_index(id).is_none() {
                return Err(RuntimeError::SavedInstructionMissing(id.clone()));
            }
        }
        self.debug.breakpoints = ids.into_iter().collect();
        Ok(())
    }

    /// Resumes at a breakpoint, optionally stopping after one instruction.
    /// # Errors
    /// `DebugPaused` indicates a normal debugger stop; other errors are execution failures.
    pub fn debug_resume(&mut self, single_step: bool) -> Result<WaitState, RuntimeError> {
        self.debug.skip_once = self.debug.paused;
        self.debug.paused = false;
        self.debug.budget = single_step.then_some(1);
        self.continue_story()
    }

    pub(super) fn check_debug_stop(&mut self, id: &InstructionId) -> Result<(), RuntimeError> {
        let skip = std::mem::take(&mut self.debug.skip_once);
        if self.debug.budget == Some(0) || !skip && self.debug.breakpoints.contains(id) {
            self.debug.paused = true;
            return Err(RuntimeError::DebugPaused);
        }
        if let Some(budget) = &mut self.debug.budget {
            *budget -= 1;
        }
        Ok(())
    }
    /// Returns stable identities of the currently visible menu options.
    ///
    /// # Errors
    /// Returns an error outside a menu or when evaluating a condition fails.
    pub fn choice_ids(&self) -> Result<Vec<TranslationId>, RuntimeError> {
        if !matches!(self.waiting, Some(WaitState::Choice { .. })) {
            return Err(RuntimeError::NotChoosing);
        }
        let instruction = self
            .program
            .instructions
            .get(self.instruction)
            .ok_or(RuntimeError::NotChoosing)?;
        let InstructionKind::Choice { options, .. } = &instruction.kind else {
            return Err(RuntimeError::NotChoosing);
        };
        Ok(
            visible_choices(options, &self.variables, instruction.span.line)?
                .into_iter()
                .map(|choice| choice.translation_id.clone())
                .collect(),
        )
    }

    /// Enables a bounded set of visited instruction indices for coverage reporting.
    pub fn enable_tracing(&mut self) {
        self.trace = Some(std::collections::BTreeSet::default());
    }

    /// Returns distinct instructions executed since tracing was enabled.
    pub fn visited_instructions(&self) -> impl Iterator<Item = usize> + '_ {
        self.trace.iter().flat_map(|trace| trace.iter().copied())
    }

    #[must_use]
    pub const fn debug_paused(&self) -> bool {
        self.debug.paused
    }

    /// Evaluates a deterministic expression against the current story variables.
    /// # Errors
    /// Returns execution errors for invalid types or missing names.
    pub fn evaluate_condition(&self, expression: &Expr) -> Result<Value, RuntimeError> {
        super::value::evaluate(expression, &self.variables, 0)
    }

    #[must_use]
    pub fn debug_state(&self) -> DebugState {
        let instruction = self.program.instructions.get(if self.debug.paused {
            self.instruction
        } else {
            self.last_instruction
        });
        DebugState {
            paused: self.debug.paused,
            next_instruction: self.instruction,
            label: self.current_label().map(ToOwned::to_owned),
            instruction: instruction.map(|item| item.id.clone()),
            location: instruction.map(|item| item.span.clone()),
            call_stack: self
                .call_stack
                .iter()
                .filter_map(|frame| {
                    self.program
                        .instructions
                        .get(frame.return_address.saturating_sub(1))
                        .map(|item| item.span.clone())
                })
                .collect(),
            variables: self.variables.clone(),
            waiting: self.waiting.clone(),
        }
    }
}
