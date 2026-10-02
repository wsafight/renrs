#[path = "analysis/context.rs"]
mod context;
#[path = "analysis/graph.rs"]
mod graph;

use std::collections::{BTreeSet, HashSet, VecDeque};
use std::sync::Arc;

use crate::compiler::{InstructionKind, Program, StatementId};
use crate::diagnostic::Diagnostic;
use crate::syntax::{Expr, Span, StrPart};
use crate::text::{is_text_tag, validate_text_source};

use context::{AssignmentSet, ContainingLabel, VariableIndex, containing_labels};
use graph::ControlFlowGraph;

/// Performs whole-program control-flow and definite-assignment analysis.
#[must_use]
pub fn analyze(program: &Program) -> Vec<Diagnostic> {
    let labels = containing_labels(program);
    let graph = ControlFlowGraph::new(program, &labels);
    let reachable = graph.reachable(program.labels.get("start").copied());
    let mut diagnostics = unreachable_diagnostics(program, &reachable, &labels);
    diagnostics.extend(definite_assignment_diagnostics(
        program, &graph, &reachable, &labels,
    ));
    diagnostics.extend(immediate_cycle_diagnostics(program, &graph, &reachable));
    diagnostics
}

/// Returns labels whose entry instruction is reachable from `start`.
#[must_use]
pub fn reachable_labels(program: &Program) -> BTreeSet<String> {
    let labels = containing_labels(program);
    let graph = ControlFlowGraph::new(program, &labels);
    let reachable = graph.reachable(program.labels.get("start").copied());
    program
        .labels
        .iter()
        .filter(|(_, index)| reachable.get(**index).copied().unwrap_or(false))
        .map(|(name, _)| name.clone())
        .collect()
}

fn unreachable_diagnostics(
    program: &Program,
    reachable: &[bool],
    labels: &[Option<ContainingLabel<'_>>],
) -> Vec<Diagnostic> {
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
        let Some(label) = labels[index] else {
            continue;
        };
        if reachable_labels.contains(label.name)
            && reported.insert(instruction.statement_id.clone())
        {
            diagnostics.push(warning_at(
                &instruction.span,
                "statement is unreachable because control flow cannot reach it",
            ));
        }
    }
    diagnostics
}

#[allow(clippy::too_many_lines)]
fn definite_assignment_diagnostics(
    program: &Program,
    graph: &ControlFlowGraph,
    reachable: &[bool],
    labels: &[Option<ContainingLabel<'_>>],
) -> Vec<Diagnostic> {
    let Some(start) = program.labels.get("start").copied() else {
        return Vec::new();
    };
    let variables = VariableIndex::new(program);
    let mut inputs = vec![None::<Arc<AssignmentSet>>; program.instructions.len()];
    inputs[start] = Some(Arc::new(variables.defaults(program)));
    let mut pending = VecDeque::from([start]);
    while let Some(index) = pending.pop_front() {
        let input = Arc::clone(inputs[index].as_ref().expect("queued input exists"));
        let assignment = match &program.instructions[index].kind {
            InstructionKind::Set { variable, .. } | InstructionKind::Extension { variable, .. } => {
                (!is_label_parameter(program, labels[index], variable))
                    .then(|| variables.bit(variable))
                    .flatten()
            }
            InstructionKind::Return { value: Some(_) } => variables.bit("_return"),
            _ => None,
        };
        let output = assignment.map_or(input.clone(), |variable| {
            if input.contains(variable) {
                input.clone()
            } else {
                let mut output = (*input).clone();
                output.insert(variable);
                Arc::new(output)
            }
        });
        for successor in &graph.successors[index] {
            if !reachable[*successor] {
                continue;
            }
            let changed = match &mut inputs[*successor] {
                Some(existing) => existing
                    .intersection_if_changed(&output)
                    .map(|intersection| *existing = Arc::new(intersection))
                    .is_some(),
                slot @ None => {
                    *slot = Some(Arc::clone(&output));
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
        let assigned = inputs[index].as_deref();
        let parameters = labels[index]
            .and_then(|label| program.label_parameters.get(label.name))
            .map(Vec::as_slice)
            .unwrap_or_default();
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
            InstructionKind::Choice { prompt, options } => {
                if let Some(prompt) = prompt {
                    match interpolation_variables(&prompt.text) {
                        Ok(variables) => used.extend(variables),
                        Err(message) => diagnostics.push(error_at(&instruction.span, message)),
                    }
                    if let Err(message) = validate_text_source(&prompt.text) {
                        diagnostics.push(error_at(&instruction.span, message));
                    }
                }
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
        for variable in used {
            let assigned_on_path = variables
                .bit(&variable)
                .is_some_and(|variable| assigned.is_some_and(|set| set.contains(variable)));
            let assigned_as_parameter = parameters.iter().any(|parameter| parameter == &variable);
            if !assigned_on_path
                && !assigned_as_parameter
                && reported.insert((instruction.id.clone(), variable.clone()))
            {
                diagnostics.push(error_at(
                    &instruction.span,
                    format!("variable `{variable}` may be unassigned on this path"),
                ));
            }
        }
    }
    diagnostics
}

fn is_label_parameter(
    program: &Program,
    label: Option<ContainingLabel<'_>>,
    variable: &str,
) -> bool {
    label
        .and_then(|label| program.label_parameters.get(label.name))
        .is_some_and(|parameters| parameters.iter().any(|parameter| parameter == variable))
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
        Expr::Spanned { expression, .. } => expression_variables(expression, variables),
        Expr::Interpolate { parts } => {
            for part in parts {
                if let StrPart::Hole(expression) = part {
                    expression_variables(expression, variables);
                }
            }
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
    let components = graph.components(reachable);
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
    fn analyzes_only_defaults_used_by_each_call() {
        let missing = diagnostics(
            "label start:\n    call target\n    return\nlabel target(value=missing):\n    return value",
        );
        assert!(
            missing
                .iter()
                .any(|item| item.message.contains("`missing` may be unassigned"))
        );

        let overridden = diagnostics(
            "label start:\n    call target(value=1)\n    return\nlabel target(value=missing):\n    return value",
        );
        assert!(
            !overridden
                .iter()
                .any(|item| item.message.contains("missing"))
        );
    }

    #[test]
    fn parameter_assignment_does_not_escape_its_dynamic_scope() {
        let found = diagnostics(
            "label start:\n    call target(1)\n    return value\nlabel target(value):\n    set value = 2\n    return",
        );
        assert!(
            found
                .iter()
                .any(|item| item.message.contains("`value` may be unassigned"))
        );

        let assigned = diagnostics(
            "default value = 0\nlabel start:\n    call target(1)\n    return value\nlabel target(value):\n    set value = 2\n    return",
        );
        assert!(!assigned.iter().any(Diagnostic::is_error));
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

    #[test]
    fn reachable_labels_exclude_unused_entries() {
        let program = compile(
            &parse_script(
                "label start:\n    jump done\nlabel done:\n    return\nlabel unused:\n    return",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
        let labels = reachable_labels(&program);
        assert!(labels.contains("start") && labels.contains("done"));
        assert!(!labels.contains("unused"));
    }

    #[test]
    fn pause_counts_as_an_interaction_in_a_cycle() {
        let found = diagnostics("label start:\n    pause 1\n    jump start");
        assert!(!found.iter().any(Diagnostic::is_error));
    }
}
