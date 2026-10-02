use std::collections::HashMap;

use crate::compiler::{InstructionKind, Program};
use crate::syntax::{Expr, Value};
use renrs_algorithms::graph::Graph;

use super::context::ContainingLabel;

pub(super) struct ControlFlowGraph {
    pub(super) successors: Vec<Vec<usize>>,
}

impl ControlFlowGraph {
    pub(super) fn new(program: &Program, labels: &[Option<ContainingLabel<'_>>]) -> Self {
        let length = program.instructions.len();
        let mut return_targets = HashMap::<usize, Vec<usize>>::new();
        for (index, instruction) in program.instructions.iter().enumerate() {
            if let InstructionKind::Call { target, .. } = instruction.kind
                && index + 1 < length
            {
                return_targets.entry(target).or_default().push(index + 1);
            }
        }
        let successors = program
            .instructions
            .iter()
            .enumerate()
            .map(|(index, instruction)| {
                let next = (index + 1 < length).then_some(index + 1);
                match &instruction.kind {
                    InstructionKind::Jump { target } | InstructionKind::Call { target, .. } => {
                        valid_targets([Some(*target)], length)
                    }
                    InstructionKind::JumpIfFalse { condition, target } => {
                        match constant_boolean(condition) {
                            Some(true) => valid_targets([next], length),
                            Some(false) => valid_targets([Some(*target)], length),
                            None => valid_targets([next, Some(*target)], length),
                        }
                    }
                    InstructionKind::Choice { options, .. } => options
                        .iter()
                        .map(|option| option.target)
                        .filter(|target| *target < length)
                        .collect(),
                    InstructionKind::Return { .. } => labels[index]
                        .map(|label| label.start)
                        .and_then(|label| return_targets.get(&label))
                        .cloned()
                        .unwrap_or_default(),
                    _ => valid_targets([next], length),
                }
            })
            .collect();
        Self { successors }
    }

    pub(super) fn reachable(&self, start: Option<usize>) -> Vec<bool> {
        self.algorithm().reachable(start)
    }

    pub(super) fn components(&self, reachable: &[bool]) -> Vec<Vec<usize>> {
        self.algorithm().strongly_connected_components(reachable)
    }

    pub(super) fn algorithm(&self) -> Graph<'_> {
        Graph::new(&self.successors)
    }
}

fn valid_targets<const N: usize>(targets: [Option<usize>; N], length: usize) -> Vec<usize> {
    targets
        .into_iter()
        .flatten()
        .filter(|target| *target < length)
        .collect()
}

fn constant_boolean(expression: &Expr) -> Option<bool> {
    match expression.unspanned() {
        Expr::Value(Value::Boolean(value)) => Some(*value),
        _ => None,
    }
}
