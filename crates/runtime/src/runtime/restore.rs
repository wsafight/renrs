use super::restore_state::{prepare_saved_stage, prepare_saved_wait};
use super::{
    BTreeMap, CallFrame, IdAliasResolution, InstructionId, InstructionKind, LocalizationError,
    Localizer, Program, ReloadReport, RollbackCheckpoint, Runtime, RuntimeError, RuntimeSnapshot,
    StageState, Value, WaitState, evaluate, execution, interpolate, parse_text_markup,
};
use std::sync::Arc;

impl Runtime {
    /// Creates a runtime positioned at the program's `start` label.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::MissingStart`] if the compiled program has no
    /// entry point.
    pub fn new(program: impl Into<std::sync::Arc<Program>>) -> Result<Self, RuntimeError> {
        let program = program.into();
        let instruction = *program
            .labels
            .get("start")
            .ok_or(RuntimeError::MissingStart)?;
        let variables = evaluate_defaults(&program)?;
        Ok(Self {
            extensions: renrs_extensions::Extensions::new(&program.extensions)
                .map_err(|error| execution(0, error))?,
            profile_revision: 0,
            profile: crate::progress::Profile {
                project_id: program.project_id.clone(),
                ..Default::default()
            },
            program,
            instruction,
            last_instruction: instruction,
            trace: None,
            call_stack: Vec::new(),
            variables: Arc::new(variables),
            stage: Arc::new(StageState::default()),
            waiting: None,
            history: std::sync::Arc::default(),
            rollback: Vec::new(),
            audio_events: Vec::new(),
            localizer: Localizer::default(),
            debug: super::inspect::DebugControl::default(),
            previous_stage: None,
        })
    }

    /// Restores a current-format snapshot from the same compiled script.
    ///
    /// # Errors
    ///
    /// Rejects other snapshot formats, script fingerprints, and invalid state.
    pub fn restore(
        program: impl Into<std::sync::Arc<Program>>,
        snapshot: RuntimeSnapshot,
    ) -> Result<Self, RuntimeError> {
        let program = program.into();
        if snapshot.program_fingerprint != program.fingerprint {
            return Err(RuntimeError::ScriptChanged);
        }
        Self::restore_session(program, snapshot).map(|(runtime, _)| runtime)
    }

    /// Restores a persisted save against the current compiled program.
    ///
    /// An identical script is restored directly. After a content update, every
    /// active execution and call-stack position must resolve through an explicit
    /// `@id` or `alias`; automatic positions are rejected instead of guessed.
    ///
    /// # Errors
    ///
    /// Rejects other snapshot formats, removed or automatic saved positions,
    /// inconsistent state, and defaults that cannot be initialized.
    pub fn restore_compatible(
        program: impl Into<std::sync::Arc<Program>>,
        snapshot: RuntimeSnapshot,
    ) -> Result<(Self, ReloadReport), RuntimeError> {
        Self::restore_session(program, snapshot)
    }

    // Hot reload and compatible saves share stable-position remapping.
    pub(super) fn restore_session(
        program: impl Into<std::sync::Arc<Program>>,
        mut snapshot: RuntimeSnapshot,
    ) -> Result<(Self, ReloadReport), RuntimeError> {
        let program = program.into();
        if snapshot.format_version != RuntimeSnapshot::FORMAT_VERSION {
            return Err(RuntimeError::SaveVersion {
                found: snapshot.format_version,
                supported: RuntimeSnapshot::FORMAT_VERSION,
            });
        }
        let mut report = ReloadReport {
            alias_resolutions: Vec::new(),
            dropped_rollback_checkpoints: 0,
            initialized_defaults: Vec::new(),
        };
        let changed = snapshot.program_fingerprint != program.fingerprint;
        let (instruction, mut call_stack) =
            restore_stable_positions(&program, &snapshot, &mut report, changed)?;
        let localizer = Localizer::new(snapshot.language.clone());
        if changed {
            initialize_missing_defaults(
                &program,
                Arc::make_mut(&mut snapshot.variables),
                &mut call_stack,
                &mut report,
            )?;
        }
        prepare_saved_stage(&program, Arc::make_mut(&mut snapshot.stage))?;
        if let Some(waiting) = &mut snapshot.waiting {
            prepare_saved_wait(&program, waiting)?;
        }
        let waiting = restore_wait_state(
            &program,
            instruction,
            snapshot.waiting,
            &snapshot.variables,
            &localizer,
        )?;
        let mut rollback = Vec::new();
        for checkpoint in snapshot.rollback {
            match restore_checkpoint(
                &program,
                checkpoint,
                snapshot.history.len(),
                &mut report,
                changed,
            ) {
                Ok(checkpoint) => rollback.push(checkpoint),
                Err(_) if changed => report.dropped_rollback_checkpoints += 1,
                Err(error) => return Err(error),
            }
        }
        let last_instruction = snapshot
            .last_instruction_id
            .as_ref()
            .and_then(|id| program.instruction_index(id))
            .unwrap_or_else(|| {
                instruction.saturating_sub(usize::from(snapshot.instruction_is_interaction_anchor))
            });
        let mut runtime = Self {
            extensions: renrs_extensions::Extensions::new(&program.extensions)
                .map_err(|error| execution(0, error))?,
            profile_revision: 0,
            profile: crate::progress::Profile {
                project_id: program.project_id.clone(),
                ..Default::default()
            },
            program,
            instruction,
            last_instruction,
            trace: None,
            call_stack,
            variables: snapshot.variables,
            stage: snapshot.stage,
            waiting,
            history: snapshot.history,
            rollback,
            audio_events: Vec::new(),
            localizer,
            debug: super::inspect::DebugControl::default(),
            previous_stage: None,
        };
        let latest_matches_current = runtime.rollback.last().is_some_and(|checkpoint| {
            checkpoint.instruction == runtime.instruction
                && Some(&checkpoint.waiting) == runtime.waiting.as_ref()
        });
        if !latest_matches_current
            && matches!(
                runtime.waiting,
                Some(
                    WaitState::Dialogue
                        | WaitState::Choice { .. }
                        | WaitState::Screen { .. }
                        | WaitState::Finished,
                )
            )
        {
            runtime.record_checkpoint();
        }
        Ok((runtime, report))
    }
}

pub(super) fn stable_call_stack(program: &Program, call_stack: &[CallFrame]) -> Vec<InstructionId> {
    call_stack
        .iter()
        .filter_map(|frame| {
            frame
                .return_address
                .checked_sub(1)
                .and_then(|site| program.instruction_id(site))
                .cloned()
        })
        .collect()
}

fn restore_stable_positions(
    program: &Program,
    snapshot: &RuntimeSnapshot,
    report: &mut ReloadReport,
    changed: bool,
) -> Result<(usize, Vec<CallFrame>), RuntimeError> {
    if snapshot.call_stack.len() != snapshot.call_stack_ids.len() {
        return Err(RuntimeError::InvalidStablePositions);
    }
    let mut instruction = snapshot.instruction_id.as_ref().map_or_else(
        || Ok(program.instructions.len()),
        |id| resolve_saved_instruction(program, id, report, changed),
    )?;
    if changed
        && snapshot.instruction_id.is_none()
        && !matches!(snapshot.waiting, Some(WaitState::Finished))
    {
        if let Some(id) = &snapshot.last_instruction_id {
            resolve_saved_instruction(program, id, report, true)?;
        } else {
            return Err(RuntimeError::ScriptChanged);
        }
    }
    if snapshot.instruction_is_interaction_anchor {
        instruction = instruction
            .saturating_add(1)
            .min(program.instructions.len());
    }
    let call_stack = restore_call_stack(
        program,
        &snapshot.call_stack_ids,
        snapshot.call_stack.clone(),
        report,
        changed,
    )?;
    Ok((instruction, call_stack))
}

fn restore_wait_state(
    program: &Program,
    instruction: usize,
    waiting: Option<WaitState>,
    variables: &BTreeMap<String, Value>,
    localizer: &Localizer,
) -> Result<Option<WaitState>, RuntimeError> {
    match waiting {
        Some(WaitState::Dialogue)
            if !instruction
                .checked_sub(1)
                .and_then(|index| program.instructions.get(index))
                .is_some_and(|item| matches!(item.kind, InstructionKind::Dialogue { .. })) =>
        {
            Err(RuntimeError::InvalidWaitState)
        }
        Some(WaitState::Choice { .. }) => {
            let Some(InstructionKind::Choice { options, .. }) =
                program.instructions.get(instruction).map(|item| &item.kind)
            else {
                return Err(RuntimeError::InvalidWaitState);
            };
            Ok(Some(WaitState::Choice {
                options: visible_choice_labels(
                    options,
                    variables,
                    localizer,
                    program.instructions[instruction].span.line,
                )?,
            }))
        }
        Some(WaitState::Finished) if instruction != program.instructions.len() => {
            Err(RuntimeError::InvalidWaitState)
        }
        waiting => Ok(waiting),
    }
}

fn restore_checkpoint(
    program: &Program,
    mut checkpoint: RollbackCheckpoint,
    total_history: usize,
    report: &mut ReloadReport,
    changed: bool,
) -> Result<RollbackCheckpoint, RuntimeError> {
    if checkpoint.call_stack.len() != checkpoint.call_stack_ids.len() {
        return Err(RuntimeError::InvalidStablePositions);
    }
    checkpoint.instruction = checkpoint.instruction_id.as_ref().map_or_else(
        || Ok(program.instructions.len()),
        |id| resolve_saved_instruction(program, id, report, changed),
    )?;
    if checkpoint.instruction_is_interaction_anchor {
        checkpoint.instruction = checkpoint
            .instruction
            .saturating_add(1)
            .min(program.instructions.len());
    }
    checkpoint.call_stack = restore_call_stack(
        program,
        &checkpoint.call_stack_ids,
        checkpoint.call_stack,
        report,
        changed,
    )?;
    checkpoint.call_stack_ids = stable_call_stack(program, &checkpoint.call_stack);
    if changed {
        initialize_missing_defaults(
            program,
            Arc::make_mut(&mut checkpoint.variables),
            &mut checkpoint.call_stack,
            report,
        )?;
    }
    prepare_saved_stage(program, Arc::make_mut(&mut checkpoint.stage))?;
    prepare_saved_wait(program, &mut checkpoint.waiting)?;
    checkpoint.waiting = restore_wait_state(
        program,
        checkpoint.instruction,
        Some(checkpoint.waiting),
        &checkpoint.variables,
        &Localizer::default(),
    )?
    .ok_or(RuntimeError::InvalidWaitState)?;
    if checkpoint.history_len > total_history {
        return Err(RuntimeError::InvalidWaitState);
    }
    Ok(checkpoint)
}

fn resolve_saved_instruction(
    program: &Program,
    id: &InstructionId,
    report: &mut ReloadReport,
    changed: bool,
) -> Result<usize, RuntimeError> {
    let Some(canonical) = program.canonical_instruction_id(id) else {
        return Err(RuntimeError::SavedInstructionMissing(id.clone()));
    };
    if changed && !program.explicit_instruction_ids.contains(canonical) {
        return Err(RuntimeError::UnstableSavePosition(id.clone()));
    }
    if canonical != id {
        let resolution = IdAliasResolution {
            saved: id.clone(),
            current: canonical.clone(),
        };
        if !report.alias_resolutions.contains(&resolution) {
            report.alias_resolutions.push(resolution);
        }
    }
    program
        .instruction_index(canonical)
        .ok_or_else(|| RuntimeError::SavedInstructionMissing(id.clone()))
}

fn restore_call_stack(
    program: &Program,
    ids: &[InstructionId],
    frames: Vec<CallFrame>,
    report: &mut ReloadReport,
    changed: bool,
) -> Result<Vec<CallFrame>, RuntimeError> {
    if ids.len() != frames.len() {
        return Err(RuntimeError::InvalidStablePositions);
    }
    let mut restored = Vec::with_capacity(frames.len());
    for (id, mut frame) in ids.iter().zip(frames) {
        let index = resolve_saved_instruction(program, id, report, changed)?;
        let Some(InstructionKind::Call { parameters, .. }) = program
            .instructions
            .get(index)
            .map(|instruction| &instruction.kind)
        else {
            return Err(RuntimeError::InvalidStablePositions);
        };
        if parameters.len() != frame.previous_variables.len()
            || parameters
                .iter()
                .any(|parameter| !frame.previous_variables.contains_key(parameter))
        {
            return Err(RuntimeError::InvalidStablePositions);
        }
        frame.return_address = index + 1;
        restored.push(frame);
    }
    Ok(restored)
}

fn initialize_missing_defaults(
    program: &Program,
    variables: &mut BTreeMap<String, Value>,
    call_stack: &mut [CallFrame],
    report: &mut ReloadReport,
) -> Result<(), RuntimeError> {
    let mut default_scope = variables.clone();
    for frame in call_stack.iter().rev() {
        for (name, previous) in &frame.previous_variables {
            if let Some(previous) = previous {
                default_scope.insert(name.clone(), previous.clone());
            } else {
                default_scope.remove(name);
            }
        }
    }
    for (name, definition) in &program.defaults {
        if !default_scope.contains_key(name) {
            let value = evaluate(&definition.value, &default_scope, definition.span.line)?;
            default_scope.insert(name.clone(), value.clone());
            if let Some(frame) = call_stack
                .iter_mut()
                .find(|frame| frame.previous_variables.contains_key(name))
            {
                frame.previous_variables.insert(name.clone(), Some(value));
            } else {
                variables.insert(name.clone(), value);
            }
            if !report.initialized_defaults.contains(name) {
                report.initialized_defaults.push(name.clone());
            }
        }
    }
    Ok(())
}

pub(super) fn evaluate_defaults(
    program: &Program,
) -> Result<BTreeMap<String, Value>, RuntimeError> {
    let mut variables = BTreeMap::new();
    for (name, definition) in &program.defaults {
        let value = evaluate(&definition.value, &variables, definition.span.line)?;
        variables.insert(name.clone(), value);
    }
    Ok(variables)
}

pub(super) fn visible_choices<'a>(
    options: &'a [renrs_model::ChoiceTarget],
    variables: &BTreeMap<String, Value>,
    line: usize,
) -> Result<Vec<&'a renrs_model::ChoiceTarget>, RuntimeError> {
    options
        .iter()
        .filter_map(|option| match &option.condition {
            None => Some(Ok(option)),
            Some(condition) => match evaluate(condition, variables, line) {
                Ok(Value::Boolean(true)) => Some(Ok(option)),
                Ok(Value::Boolean(false)) => None,
                Ok(value) => Some(Err(execution(
                    line,
                    format!(
                        "menu condition must be boolean, found {}",
                        value.type_name()
                    ),
                ))),
                Err(error) => Some(Err(error)),
            },
        })
        .collect()
}

pub(super) fn visible_choice_labels(
    options: &[renrs_model::ChoiceTarget],
    variables: &BTreeMap<String, Value>,
    localizer: &Localizer,
    line: usize,
) -> Result<Vec<String>, RuntimeError> {
    visible_choices(options, variables, line)?
        .into_iter()
        .map(|option| {
            let translated = localizer
                .translate_values(&option.translation_id, &option.text, variables)
                .map_err(|error| execution(line, error.to_string()))?;
            let interpolated = interpolate(translated, variables, line)?;
            parse_text_markup(&interpolated)
                .map(|styled| styled.plain)
                .map_err(|message| execution(line, message))
        })
        .collect()
}

pub(super) fn localization_error(error: &LocalizationError) -> RuntimeError {
    RuntimeError::Localization(error.to_string())
}
