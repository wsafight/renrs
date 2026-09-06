use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::localization::TranslationId;
use crate::syntax::{Block, Span, StatementKind};

use super::ids::{
    stable_anchored_statement_id, stable_instruction_id, stable_statement_id, translation_id,
};
use super::{ChoiceTarget, CompileError, Instruction, InstructionId, InstructionKind, StatementId};

pub(super) struct Compiler<'a> {
    pub(super) instructions: Vec<Instruction>,
    pub(super) labels: IndexMap<String, usize>,
    pub(super) unresolved: Vec<(usize, String, Span)>,
    pub(super) current_label: String,
    pub(super) label_parameters: &'a IndexMap<String, Vec<String>>,
    pub(super) instruction_aliases: HashMap<InstructionId, InstructionId>,
    pub(super) alias_collision: Option<InstructionId>,
    pub(super) explicit_instruction_ids: HashSet<InstructionId>,
}

impl Compiler<'_> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn block(&mut self, block: &Block, parent_path: &str) {
        for (statement_index, statement) in block.iter().enumerate() {
            let path = child_path(parent_path, &format!("statement:{statement_index}"));
            let span = &statement.span;
            let statement_id = statement.id.as_ref().map_or_else(
                || stable_statement_id(&span.source, &self.current_label, &path),
                stable_anchored_statement_id,
            );
            let emitted_before = self.instructions.len();
            match &statement.kind {
                StatementKind::Extension {
                    name,
                    variable,
                    input,
                } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Extension {
                            name: name.clone(),
                            variable: variable.clone(),
                            input: input.clone(),
                        },
                    );
                }
                StatementKind::Nvl { mode } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Nvl { mode: mode.clone() },
                    );
                }
                StatementKind::Parallel { tracks } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Parallel {
                            tracks: tracks.clone(),
                        },
                    );
                }
                StatementKind::Timeline { block } => {
                    self.block(block, &child_path(&path, "timeline"));
                }
                StatementKind::Video { path, seconds } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Video {
                            path: path.clone(),
                            seconds: *seconds,
                        },
                    );
                }
                StatementKind::Dialogue { speaker, text } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Dialogue {
                            speaker: speaker.clone(),
                            text: text.clone(),
                            translation_id: translation_id(
                                statement.id.as_ref(),
                                &statement_id,
                                "dialogue",
                            ),
                        },
                    );
                }
                StatementKind::Scene { path } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Scene { path: path.clone() },
                    );
                }
                StatementKind::Show {
                    path,
                    alias,
                    position,
                    layer,
                } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Show {
                            path: path.clone(),
                            alias: alias.clone(),
                            position: *position,
                            layer: *layer,
                        },
                    );
                }
                StatementKind::Hide { alias } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Hide {
                            alias: alias.clone(),
                        },
                    );
                }
                StatementKind::Menu { options } => {
                    let choice_index = self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Choice {
                            options: Vec::new(),
                        },
                    );
                    let mut compiled_options = Vec::with_capacity(options.len());
                    let mut exits = Vec::with_capacity(options.len());
                    for (option_index, option) in options.iter().enumerate() {
                        compiled_options.push(ChoiceTarget {
                            text: option.text.clone(),
                            target: self.instructions.len(),
                            translation_id: translation_id(
                                option.id.as_ref(),
                                &statement_id,
                                &format!("menu-option:{option_index}"),
                            ),
                            condition: option.condition.clone(),
                        });
                        let option_path = child_path(&path, &format!("menu-option:{option_index}"));
                        self.block(&option.block, &option_path);
                        exits.push(self.emit(
                            option.span.clone(),
                            statement_id.clone(),
                            &format!("option-exit:{option_index}"),
                            InstructionKind::Jump { target: 0 },
                        ));
                    }
                    let end = self.instructions.len();
                    if let InstructionKind::Choice { options } =
                        &mut self.instructions[choice_index].kind
                    {
                        *options = compiled_options;
                    }
                    for exit in exits {
                        self.patch_target(exit, end);
                    }
                }
                StatementKind::Jump { label } => {
                    let index = self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Jump { target: 0 },
                    );
                    self.unresolved.push((index, label.clone(), span.clone()));
                }
                StatementKind::Call { label, arguments } => {
                    let index = self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Call {
                            target: 0,
                            arguments: arguments.clone(),
                            parameters: Vec::new(),
                        },
                    );
                    self.unresolved.push((index, label.clone(), span.clone()));
                }
                StatementKind::Return { value } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Return {
                            value: value.clone(),
                        },
                    );
                }
                StatementKind::Set { variable, value } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Set {
                            variable: variable.clone(),
                            value: value.clone(),
                        },
                    );
                }
                StatementKind::If {
                    branches,
                    else_block,
                } => {
                    let mut exits = Vec::new();
                    for (branch_index, (condition, body)) in branches.iter().enumerate() {
                        let condition_index = self.emit(
                            span.clone(),
                            statement_id.clone(),
                            &format!("guard:{branch_index}"),
                            InstructionKind::JumpIfFalse {
                                condition: condition.clone(),
                                target: 0,
                            },
                        );
                        let branch_path = child_path(&path, &format!("if-branch:{branch_index}"));
                        self.block(body, &branch_path);
                        exits.push(self.emit(
                            span.clone(),
                            statement_id.clone(),
                            &format!("branch-exit:{branch_index}"),
                            InstructionKind::Jump { target: 0 },
                        ));
                        let next_branch = self.instructions.len();
                        self.patch_target(condition_index, next_branch);
                    }
                    let else_path = child_path(&path, "else");
                    self.block(else_block, &else_path);
                    let end = self.instructions.len();
                    for exit in exits {
                        self.patch_target(exit, end);
                    }
                }
                StatementKind::PlayMusic {
                    path,
                    repeat,
                    fade_in,
                } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::PlayMusic {
                            path: path.clone(),
                            repeat: *repeat,
                            fade_in: *fade_in,
                        },
                    );
                }
                StatementKind::QueueMusic {
                    path,
                    repeat,
                    fade_in,
                } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::QueueMusic {
                            path: path.clone(),
                            repeat: *repeat,
                            fade_in: *fade_in,
                        },
                    );
                }
                StatementKind::PlaySound { path } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::PlaySound { path: path.clone() },
                    );
                }
                StatementKind::PlayVoice { path } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::PlayVoice { path: path.clone() },
                    );
                }
                StatementKind::StopMusic { fade_out } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::StopMusic {
                            fade_out: *fade_out,
                        },
                    );
                }
                StatementKind::Pause { seconds } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Pause { seconds: *seconds },
                    );
                }
                StatementKind::Move {
                    alias,
                    position,
                    seconds,
                } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Move {
                            alias: alias.clone(),
                            position: *position,
                            seconds: *seconds,
                        },
                    );
                }
                StatementKind::Transform {
                    alias,
                    properties,
                    seconds,
                    easing,
                } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Transform {
                            alias: alias.clone(),
                            properties: *properties,
                            seconds: *seconds,
                            easing: *easing,
                        },
                    );
                }
                StatementKind::Transition { kind, seconds } => {
                    self.emit(
                        span.clone(),
                        statement_id.clone(),
                        "main",
                        InstructionKind::Transition {
                            kind: *kind,
                            seconds: *seconds,
                        },
                    );
                }
            }
            self.record_aliases(emitted_before, &statement_id, &statement.aliases);
            if statement.id.is_some() {
                self.explicit_instruction_ids.extend(
                    self.instructions[emitted_before..]
                        .iter()
                        .filter(|instruction| instruction.statement_id == statement_id)
                        .map(|instruction| instruction.id.clone()),
                );
            }
        }
    }

    pub(super) fn emit(
        &mut self,
        span: Span,
        statement_id: StatementId,
        role: &str,
        kind: InstructionKind,
    ) -> usize {
        let index = self.instructions.len();
        let id = stable_instruction_id(&statement_id, role);
        self.instructions.push(Instruction {
            id,
            statement_id,
            role: role.to_owned(),
            span,
            kind,
        });
        index
    }

    fn patch_target(&mut self, index: usize, target: usize) {
        match &mut self.instructions[index].kind {
            InstructionKind::Jump { target: current }
            | InstructionKind::Call {
                target: current, ..
            }
            | InstructionKind::JumpIfFalse {
                target: current, ..
            } => *current = target,
            _ => unreachable!("only branch instructions have patchable targets"),
        }
    }

    pub(super) fn resolve(&mut self) -> Result<(), CompileError> {
        let labels: HashMap<_, _> = self
            .labels
            .iter()
            .map(|(name, index)| (name.clone(), *index))
            .collect();
        for (index, label, span) in std::mem::take(&mut self.unresolved) {
            let Some(target) = labels.get(&label) else {
                return Err(CompileError::UnknownLabel {
                    label,
                    file: span.source,
                    line: span.line,
                });
            };
            if let InstructionKind::Call {
                arguments,
                parameters,
                ..
            } = &mut self.instructions[index].kind
            {
                let expected = self
                    .label_parameters
                    .get(&label)
                    .map_or(&[][..], Vec::as_slice);
                if arguments.len() != expected.len() {
                    return Err(CompileError::LabelArity {
                        label,
                        expected: expected.len(),
                        found: arguments.len(),
                        file: span.source,
                        line: span.line,
                    });
                }
                parameters.extend_from_slice(expected);
            }
            self.patch_target(index, *target);
        }
        Ok(())
    }

    fn record_aliases(
        &mut self,
        emitted_before: usize,
        statement_id: &StatementId,
        aliases: &[TranslationId],
    ) {
        for instruction in &self.instructions[emitted_before..] {
            if &instruction.statement_id != statement_id {
                continue;
            }
            for alias in aliases {
                let old_statement = stable_anchored_statement_id(alias);
                let old = stable_instruction_id(&old_statement, &instruction.role);
                if self
                    .instruction_aliases
                    .insert(old.clone(), instruction.id.clone())
                    .is_some()
                {
                    self.alias_collision = Some(old);
                }
            }
        }
    }
}

fn child_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_owned()
    } else {
        format!("{parent}/{child}")
    }
}
