#[path = "analysis/graph.rs"]
mod graph;

use std::collections::{HashSet, VecDeque};

use crate::compiler::{InstructionKind, Program, StatementId};
use crate::diagnostic::Diagnostic;
use crate::syntax::{Expr, Span};
use crate::text::{is_text_tag, validate_text_source};

use graph::{ControlFlowGraph, strongly_connected_components};

/// Performs whole-program control-flow and definite-assignment analysis.
#[must_use]
pub fn analyze(program: &Program) -> Vec<Diagnostic> {
    let graph = ControlFlowGraph::new(program);
    let reachable = graph.reachable(program.labels.get("start").copied());
    let mut diagnostics = unreachable_diagnostics(program, &reachable);
    diagnostics.extend(definite_assignment_diagnostics(program, &graph, &reachable));
    diagnostics.extend(immediate_cycle_diagnostics(program, &graph, &reachable));
    diagnostics
}

fn unreachable_diagnostics(program: &Program, reachable: &[bool]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut reachable_labels = HashSet::new();
    for (name, index) in &program.labels {
        if reachable.get(*index).copied().unwrap_or(false) {
            reachable_labels.insert(name.as_str());
        } else if let Some(instruction) = program.instructions.get(*index) {
            diagnostics.push(warning_at(
                &instruction.span,
                format!("label `{name}` is unreachable from `start`"),
            ));
        }
    }

    let mut reported = HashSet::<StatementId>::new();
    for (index, instruction) in program.instructions.iter().enumerate() {
        if reachable[index] || instruction.role != "main" {
            continue;
        }
        let Some(label) = containing_label(program, index) else {
            continue;
        };
        if reachable_labels.contains(label) && reported.insert(instruction.statement_id.clone()) {
            diagnostics.push(warning_at(
                &instruction.span,
                "statement is unreachable because control flow cannot reach it",
            ));
        }
    }
    diagnostics
}

fn containing_label(program: &Program, instruction: usize) -> Option<&str> {
    program
        .labels
        .iter()
        .take_while(|(_, start)| **start <= instruction)
        .last()
        .map(|(name, _)| name.as_str())
}

#[allow(clippy::too_many_lines)]
fn definite_assignment_diagnostics(
    program: &Program,
    graph: &ControlFlowGraph,
    reachable: &[bool],
) -> Vec<Diagnostic> {
    let Some(start) = program.labels.get("start").copied() else {
        return Vec::new();
    };
    let mut inputs = vec![None::<HashSet<String>>; program.instructions.len()];
    inputs[start] = Some(program.defaults.keys().cloned().collect());
    let mut pending = VecDeque::from([start]);
    while let Some(index) = pending.pop_front() {
        let mut output = inputs[index].clone().unwrap_or_default();
        match &program.instructions[index].kind {
            InstructionKind::Set { variable, .. } | InstructionKind::Extension { variable, .. } => {
                output.insert(variable.clone());
            }
            InstructionKind::Call { parameters, .. } => {
                output.extend(parameters.iter().cloned());
            }
            InstructionKind::Return { value: Some(_) } => {
                output.insert("_return".to_owned());
            }
            _ => {}
        }
        for successor in &graph.successors[index] {
            if !reachable[*successor] {
                continue;
            }
            let changed = match &mut inputs[*successor] {
                Some(existing) => {
                    let intersection = existing.intersection(&output).cloned().collect();
                    if *existing == intersection {
                        false
                    } else {
                        *existing = intersection;
                        true
                    }
                }
                slot @ None => {
                    *slot = Some(output.clone());
                    true
                }
            };
            if changed {
                pending.push_back(*successor);
            }
        }
    }

    let mut diagnostics = Vec::new();
    let mut reported = HashSet::new();
    for (index, instruction) in program.instructions.iter().enumerate() {
        if !reachable[index] {
            continue;
        }
        let assigned = inputs[index].clone().unwrap_or_default();
        let mut used = HashSet::new();
        match &instruction.kind {
            InstructionKind::Set { value, .. }
            | InstructionKind::Extension { input: value, .. }
            | InstructionKind::JumpIfFalse {
                condition: value, ..
            } => expression_variables(value, &mut used),
            InstructionKind::Dialogue { text, .. } => match interpolation_variables(text) {
                Ok(variables) => {
                    used.extend(variables);
                    if let Err(message) = validate_text_source(text) {
                        diagnostics.push(error_at(&instruction.span, message));
                    }
                }
                Err(message) => diagnostics.push(error_at(&instruction.span, message)),
            },
            InstructionKind::Call { arguments, .. } => {
                for argument in arguments {
                    expression_variables(argument, &mut used);
                }
            }
            InstructionKind::Return { value: Some(value) } => {
                expression_variables(value, &mut used);
            }
            InstructionKind::Choice { options } => {
                for option in options {
                    if let Some(condition) = &option.condition {
                        expression_variables(condition, &mut used);
                    }
                    match interpolation_variables(&option.text) {
                        Ok(variables) => used.extend(variables),
                        Err(message) => diagnostics.push(error_at(&instruction.span, message)),
                    }
                    if let Err(message) = validate_text_source(&option.text) {
                        diagnostics.push(error_at(&instruction.span, message));
                    }
                }
            }
            _ => {}
        }
        for variable in used.difference(&assigned) {
            if reported.insert((instruction.id.clone(), variable.clone())) {
                diagnostics.push(error_at(
                    &instruction.span,
                    format!("variable `{variable}` may be unassigned on this path"),
                ));
            }
        }
    }
    diagnostics
}

fn expression_variables(expression: &Expr, variables: &mut HashSet<String>) {
    match expression {
        Expr::Variable(name) => {
            variables.insert(name.clone());
        }
        Expr::Unary { value, .. } => expression_variables(value, variables),
        Expr::Invoke { arguments, .. } => {
            for argument in arguments {
                expression_variables(argument, variables);
            }
        }
        Expr::Binary { left, right, .. } => {
            expression_variables(left, variables);
            expression_variables(right, variables);
        }
        Expr::Value(_) => {}
    }
}

fn interpolation_variables(input: &str) -> Result<HashSet<String>, &'static str> {
    let mut variables = HashSet::new();
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '{' {
            if character == '}' && characters.peek() == Some(&'}') {
                characters.next();
            }
            continue;
        }
        if characters.peek() == Some(&'{') {
            characters.next();
            continue;
        }
        let mut name = String::new();
        loop {
            match characters.next() {
                Some('}') => break,
                Some(current) => name.push(current),
                None => return Err("unclosed `{` in dialogue interpolation"),
            }
        }
        let name = name.trim();
        if is_text_tag(name) {
            continue;
        }
        if name.is_empty()
            || !name.chars().enumerate().all(|(index, character)| {
                character == '_'
                    || character.is_ascii_alphabetic()
                    || (index > 0 && character.is_ascii_digit())
            })
        {
            return Err("dialogue interpolation must contain a variable name");
        }
        variables.insert(name.to_owned());
    }
    Ok(variables)
}

fn immediate_cycle_diagnostics(
    program: &Program,
    graph: &ControlFlowGraph,
    reachable: &[bool],
) -> Vec<Diagnostic> {
    let components = strongly_connected_components(graph, reachable);
    components
        .into_iter()
        .filter(|component| {
            let members = component.iter().copied().collect::<HashSet<_>>();
            let cyclic = component.len() > 1
                || component
                    .iter()
                    .any(|index| graph.successors[*index].iter().any(|next| next == index));
            let has_interaction = component.iter().any(|index| {
                matches!(
                    program.instructions[*index].kind,
                    InstructionKind::Dialogue { .. }
                        | InstructionKind::Choice { .. }
                        | InstructionKind::Pause { .. }
                        | InstructionKind::Transition { .. }
                        | InstructionKind::Move { .. }
                        | InstructionKind::Transform { .. }
                        | InstructionKind::Parallel { .. }
                )
            });
            let has_exit = component.iter().any(|index| {
                graph.successors[*index]
                    .iter()
                    .any(|next| !members.contains(next))
            });
            cyclic && !has_interaction && !has_exit
        })
        .filter_map(|component| component.into_iter().min())
        .map(|index| {
            error_at(
                &program.instructions[index].span,
                "reachable instruction cycle has no interaction or exit",
            )
            .with_hint("add dialogue, a menu, a pause, or a path that leaves the cycle")
        })
        .collect()
}

fn error_at(span: &Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(&span.source, span.line, span.column, message)
}

fn warning_at(span: &Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::warning(&span.source, span.line, span.column, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, parse_script};

    fn diagnostics(source: &str) -> Vec<Diagnostic> {
        let script = parse_script(source, "test.rns").unwrap();
        analyze(&compile(&script).unwrap())
    }

    #[test]
    fn reports_unreachable_label_and_statement_as_warnings() {
        let found = diagnostics(
            "label start:\n    jump done\n    \"Dead\"\nlabel done:\n    return\nlabel unused:\n    \"Never\"",
        );
        assert!(
            found
                .iter()
                .any(|item| item.message.contains("statement is unreachable"))
        );
        assert!(
            found
                .iter()
                .any(|item| item.message.contains("label `unused`"))
        );
        assert!(found.iter().all(|item| !item.is_error()));
    }

    #[test]
    fn reports_path_sensitive_unassigned_variables() {
        let found = diagnostics(
            "label start:\n    menu:\n        \"Gain\":\n            set score = 1\n        \"Wait\":\n            \"No score\"\n    \"Score {score}\"",
        );
        assert!(
            found.iter().any(|item| {
                item.is_error() && item.message.contains("`score` may be unassigned")
            })
        );
    }

    #[test]
    fn accepts_assignment_on_every_branch() {
        let found = diagnostics(
            "label start:\n    menu:\n        \"One\":\n            set score = 1\n        \"Two\":\n            set score = 2\n    \"Score {score}\"",
        );
        assert!(!found.iter().any(Diagnostic::is_error));
    }

    #[test]
    fn propagates_definite_assignment_through_called_labels() {
        let found = diagnostics(
            "label start:\n    call initialize\n    \"Value {answer}\"\n    return\nlabel initialize:\n    set answer = 42\n    return\n",
        );
        assert!(!found.iter().any(|item| item.message.contains("answer")));
    }

    #[test]
    fn reports_immediate_infinite_cycle() {
        let found = diagnostics("label start:\n    jump start");
        assert!(
            found
                .iter()
                .any(|item| { item.is_error() && item.message.contains("no interaction or exit") })
        );
    }

    #[test]
    fn reports_unbalanced_text_tags_without_running_story() {
        let found = diagnostics("label start:\n    \"{b}Never closed\"");
        assert!(
            found
                .iter()
                .any(|item| item.is_error() && item.message.contains("not closed"))
        );
    }
}
