use super::{
    AudioEvent, CallFrame, InstructionKind, MAX_IMMEDIATE_STEPS, MusicState, Runtime, RuntimeError,
    SpriteState, TransformState, TransitionKind, Value, VisualEffect, WaitState, evaluate,
    execution, visible_choice_labels, visible_choices,
};
use std::sync::Arc;

impl Runtime {
    /// Executes immediate instructions until the story reaches an interaction.
    ///
    /// # Errors
    ///
    /// Returns an execution error for invalid types, missing variables, broken
    /// compiled references, arithmetic faults, or a probable infinite loop.
    #[allow(clippy::too_many_lines)]
    pub fn advance(&mut self) -> Result<WaitState, RuntimeError> {
        if let Some(waiting) = &self.waiting {
            return Ok(waiting.clone());
        }

        for _ in 0..MAX_IMMEDIATE_STEPS {
            let Some(instruction) = self.program.instructions.get(self.instruction).cloned() else {
                self.waiting = Some(WaitState::Finished);
                return Ok(WaitState::Finished);
            };
            let line = instruction.span.line;
            self.check_debug_stop(&instruction.id)?;
            self.observe_progress();
            self.last_instruction = self.instruction;
            if let Some(trace) = &mut self.trace {
                trace.insert(self.instruction);
            }
            match instruction.kind {
                InstructionKind::Nvl { mode } => {
                    if mode == "on" {
                        Arc::make_mut(&mut self.stage).nvl = true;
                    }
                    if mode == "off" {
                        Arc::make_mut(&mut self.stage).nvl = false;
                    }
                    Arc::make_mut(&mut self.stage).nvl_start = self.history.len();
                    self.instruction += 1;
                }
                InstructionKind::Parallel { tracks } => {
                    let seconds = crate::animation::validate(&self.stage, &tracks)
                        .map_err(|message| execution(line, message))?;
                    let from = Box::new(self.stage.as_ref().clone());
                    self.stage = Arc::new(crate::animation::sample(&from, &tracks, seconds));
                    self.instruction += 1;
                    return Ok(self.set_waiting(WaitState::Effect {
                        effect: VisualEffect::Parallel {
                            from,
                            tracks,
                            seconds,
                        },
                    }));
                }
                InstructionKind::Video { path, seconds } => {
                    self.instruction += 1;
                    return Ok(self.set_waiting(WaitState::Effect {
                        effect: VisualEffect::Video { path, seconds },
                    }));
                }
                InstructionKind::Dialogue {
                    speaker,
                    text,
                    translation_id,
                } => {
                    self.present_dialogue(
                        &instruction.statement_id,
                        speaker.as_deref(),
                        &text,
                        &translation_id,
                        line,
                    )?;
                    self.instruction += 1;
                    return Ok(self.set_waiting(WaitState::Dialogue));
                }
                InstructionKind::Scene { path } => {
                    self.previous_stage = Some(self.stage.as_ref().clone());
                    let stage = Arc::make_mut(&mut self.stage);
                    stage.background = Some(path);
                    stage.sprites.clear();
                    self.instruction += 1;
                }
                InstructionKind::Show {
                    path,
                    alias,
                    position,
                    layer,
                    display_layer,
                    display_order,
                } => {
                    let sprite = SpriteState {
                        composition: self.resolve_image(&path, line)?,
                        path,
                        alias: alias.clone(),
                        position,
                        layer,
                        display_layer,
                        display_order,
                        transform: TransformState::identity(),
                    };
                    if let Some(existing) = Arc::make_mut(&mut self.stage)
                        .sprites
                        .iter_mut()
                        .find(|item| item.alias == alias)
                    {
                        *existing = sprite;
                    } else {
                        Arc::make_mut(&mut self.stage).sprites.push(sprite);
                    }
                    self.instruction += 1;
                }
                InstructionKind::Hide { alias } => {
                    Arc::make_mut(&mut self.stage)
                        .sprites
                        .retain(|item| item.alias != alias);
                    self.instruction += 1;
                }
                InstructionKind::ClearLayer { display_layer } => {
                    Arc::make_mut(&mut self.stage)
                        .sprites
                        .retain(|item| item.display_layer != display_layer);
                    self.instruction += 1;
                }
                InstructionKind::Choice { prompt, options } => {
                    if let Some(prompt) = prompt {
                        self.present_dialogue(
                            &instruction.statement_id,
                            prompt.speaker.as_deref(),
                            &prompt.text,
                            &prompt.translation_id,
                            line,
                        )?;
                    } else {
                        Arc::make_mut(&mut self.stage).dialogue = None;
                    }
                    let labels =
                        visible_choice_labels(&options, &self.variables, &self.localizer, line)?;
                    return Ok(self.set_waiting(WaitState::Choice { options: labels }));
                }
                InstructionKind::Jump { target } => self.instruction = target,
                InstructionKind::Call {
                    target,
                    arguments,
                    parameters,
                } => {
                    let values = arguments
                        .iter()
                        .map(|argument| evaluate(argument, &self.variables, line))
                        .collect::<Result<Vec<_>, _>>()?;
                    let previous_variables = parameters
                        .iter()
                        .map(|parameter| {
                            (parameter.clone(), self.variables.get(parameter).cloned())
                        })
                        .collect();
                    for (parameter, value) in parameters.into_iter().zip(values) {
                        Arc::make_mut(&mut self.variables).insert(parameter, value);
                    }
                    self.call_stack.push(CallFrame {
                        return_address: self.instruction + 1,
                        previous_variables,
                    });
                    self.instruction = target;
                }
                InstructionKind::Return { value } => {
                    let returned = value
                        .map(|expression| evaluate(&expression, &self.variables, line))
                        .transpose()?;
                    if let Some(frame) = self.call_stack.pop() {
                        let variables = Arc::make_mut(&mut self.variables);
                        for (name, previous) in frame.previous_variables {
                            if let Some(previous) = previous {
                                variables.insert(name, previous);
                            } else {
                                variables.remove(&name);
                            }
                        }
                        if let Some(returned) = returned {
                            variables.insert("_return".to_owned(), returned);
                        }
                        self.instruction = frame.return_address;
                    } else {
                        if let Some(returned) = returned {
                            Arc::make_mut(&mut self.variables)
                                .insert("_return".to_owned(), returned);
                        }
                        self.instruction = self.program.instructions.len();
                        return Ok(self.set_waiting(WaitState::Finished));
                    }
                }
                InstructionKind::Set { variable, value } => {
                    let value = evaluate(&value, &self.variables, line)?;
                    self.set_persistent_variable(&variable, &value);
                    Arc::make_mut(&mut self.variables).insert(variable, value);
                    self.instruction += 1;
                }
                InstructionKind::Extension {
                    name,
                    variable,
                    input,
                } => {
                    let input = evaluate(&input, &self.variables, line)?;
                    let value = self
                        .extensions
                        .invoke(&name, &input)
                        .map_err(|error| execution(line, error))?;
                    self.set_persistent_variable(&variable, &value);
                    Arc::make_mut(&mut self.variables).insert(variable, value);
                    self.instruction += 1;
                }
                InstructionKind::JumpIfFalse { condition, target } => {
                    let value = evaluate(&condition, &self.variables, line)?;
                    match value {
                        Value::Boolean(true) => self.instruction += 1,
                        Value::Boolean(false) => self.instruction = target,
                        other => {
                            return Err(RuntimeError::Execution {
                                line,
                                message: format!(
                                    "condition must be boolean, found {}",
                                    other.type_name()
                                ),
                            });
                        }
                    }
                }
                InstructionKind::PlayMusic {
                    path,
                    repeat,
                    fade_in,
                    volume,
                } => {
                    Arc::make_mut(&mut self.stage).music = Some(MusicState {
                        path: path.clone(),
                        repeat,
                        fade_in,
                        volume,
                    });
                    Arc::make_mut(&mut self.stage).music_queue.clear();
                    self.audio_events.push(AudioEvent::PlayMusic {
                        path,
                        repeat,
                        fade_in,
                        volume,
                    });
                    self.instruction += 1;
                }
                InstructionKind::QueueMusic {
                    path,
                    repeat,
                    fade_in,
                    volume,
                } => {
                    let music = MusicState {
                        path: path.clone(),
                        repeat,
                        fade_in,
                        volume,
                    };
                    if self.stage.music.is_none() {
                        Arc::make_mut(&mut self.stage).music = Some(music);
                        self.audio_events.push(AudioEvent::PlayMusic {
                            path,
                            repeat,
                            fade_in,
                            volume,
                        });
                    } else {
                        Arc::make_mut(&mut self.stage).music_queue.push(music);
                        self.audio_events.push(AudioEvent::QueueMusic {
                            path,
                            repeat,
                            fade_in,
                            volume,
                        });
                    }
                    self.instruction += 1;
                }
                InstructionKind::PlaySound { path, volume } => {
                    self.audio_events
                        .push(AudioEvent::PlaySound { path, volume });
                    self.instruction += 1;
                }
                InstructionKind::PlayVoice { path } => {
                    Arc::make_mut(&mut self.stage).voice = Some(path.clone());
                    self.audio_events.push(AudioEvent::PlayVoice { path });
                    self.instruction += 1;
                }
                InstructionKind::StopMusic { fade_out } => {
                    let stage = Arc::make_mut(&mut self.stage);
                    stage.music = None;
                    stage.music_queue.clear();
                    self.audio_events.push(AudioEvent::StopMusic { fade_out });
                    self.instruction += 1;
                }
                InstructionKind::Pause { seconds } => {
                    self.instruction += 1;
                    return Ok(self.set_waiting(WaitState::Pause { seconds }));
                }
                InstructionKind::Move {
                    alias,
                    position,
                    seconds,
                } => {
                    let Some(sprite) = Arc::make_mut(&mut self.stage)
                        .sprites
                        .iter_mut()
                        .find(|sprite| sprite.alias == alias)
                    else {
                        return Err(RuntimeError::Execution {
                            line,
                            message: format!("cannot move unknown image alias `{alias}`"),
                        });
                    };
                    let from = sprite.position;
                    sprite.position = position;
                    self.instruction += 1;
                    return Ok(self.set_waiting(WaitState::Effect {
                        effect: VisualEffect::Tween {
                            alias,
                            from,
                            to: position,
                            seconds,
                        },
                    }));
                }
                InstructionKind::Transform {
                    alias,
                    properties,
                    seconds,
                    easing,
                } => {
                    let stage = Arc::make_mut(&mut self.stage);
                    let transform = if alias == "camera" {
                        &mut stage.camera
                    } else {
                        &mut stage
                            .sprites
                            .iter_mut()
                            .find(|sprite| sprite.alias == alias)
                            .ok_or_else(|| {
                                execution(
                                    line,
                                    format!("cannot transform unknown image alias `{alias}`"),
                                )
                            })?
                            .transform
                    };
                    let from = *transform;
                    let to = properties.apply(from);
                    *transform = to;
                    self.instruction += 1;
                    return Ok(self.set_waiting(WaitState::Effect {
                        effect: VisualEffect::Transform {
                            alias,
                            from,
                            to,
                            seconds,
                            easing,
                        },
                    }));
                }
                InstructionKind::Transition { kind, seconds } => {
                    self.instruction += 1;
                    let effect = match kind {
                        TransitionKind::Fade => VisualEffect::Fade { seconds },
                        TransitionKind::Dissolve => VisualEffect::Dissolve {
                            from: Box::new(self.previous_stage.take().unwrap_or_default()),
                            seconds,
                        },
                    };
                    return Ok(self.set_waiting(WaitState::Effect { effect }));
                }
            }
        }
        Err(RuntimeError::Execution {
            line: self
                .program
                .instructions
                .get(self.instruction)
                .map_or(0, |instruction| instruction.span.line),
            message: "possible infinite loop: no interaction after 10,000 instructions".to_owned(),
        })
    }

    /// Releases a dialogue or pause wait and advances to the next interaction.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::NotChoosing`] if called while a choice is active,
    /// or any execution error produced by subsequent instructions.
    pub fn continue_story(&mut self) -> Result<WaitState, RuntimeError> {
        let was_dialogue = matches!(self.waiting, Some(WaitState::Dialogue));
        match self.waiting {
            Some(WaitState::Dialogue | WaitState::Pause { .. } | WaitState::Effect { .. }) => {
                if was_dialogue && self.stage.voice.is_some() {
                    Arc::make_mut(&mut self.stage).voice = None;
                    self.audio_events.push(AudioEvent::StopVoice);
                }
                self.waiting = None;
                self.advance()
            }
            Some(WaitState::Finished) => Ok(WaitState::Finished),
            Some(WaitState::Choice { .. }) => Err(RuntimeError::NotChoosing),
            None => self.advance(),
        }
    }

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
            self.audio_events.push(AudioEvent::StopVoice);
        }
        self.instruction = target;
        self.waiting = None;
        self.advance()
    }
}
