use std::collections::HashMap;

use crate::compiler::{InstructionKind, Program};
use crate::syntax::{Expr, Value};

pub(super) struct ControlFlowGraph {
    pub(super) successors: Vec<Vec<usize>>,
}

impl ControlFlowGraph {
    pub(super) fn new(program: &Program) -> Self {
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
                    InstructionKind::Return { .. } => containing_label_start(program, index)
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
        let mut reachable = vec![false; self.successors.len()];
        let Some(start) = start.filter(|index| *index < reachable.len()) else {
            return reachable;
        };
        let mut pending = vec![start];
        while let Some(index) = pending.pop() {
            if reachable[index] {
                continue;
            }
            reachable[index] = true;
            pending.extend(
                self.successors[index]
                    .iter()
                    .copied()
                    .filter(|next| !reachable[*next]),
            );
        }
        reachable
    }
}

fn containing_label_start(program: &Program, instruction: usize) -> Option<usize> {
    program
        .labels
        .values()
        .copied()
        .take_while(|start| *start <= instruction)
        .last()
}

fn valid_targets<const N: usize>(targets: [Option<usize>; N], length: usize) -> Vec<usize> {
    targets
        .into_iter()
        .flatten()
        .filter(|target| *target < length)
        .collect()
}

fn constant_boolean(expression: &Expr) -> Option<bool> {
    match expression {
        Expr::Value(Value::Boolean(value)) => Some(*value),
        _ => None,
    }
}

pub(super) fn strongly_connected_components(
    graph: &ControlFlowGraph,
    reachable: &[bool],
) -> Vec<Vec<usize>> {
    struct Tarjan<'a> {
        graph: &'a ControlFlowGraph,
        reachable: &'a [bool],
        next_index: usize,
        indices: Vec<Option<usize>>,
        low_links: Vec<usize>,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        components: Vec<Vec<usize>>,
    }

    impl Tarjan<'_> {
        fn visit(&mut self, node: usize) {
            let index = self.next_index;
            self.next_index += 1;
            self.indices[node] = Some(index);
            self.low_links[node] = index;
            self.stack.push(node);
            self.on_stack[node] = true;

            for successor in &self.graph.successors[node] {
                if !self.reachable[*successor] {
                    continue;
                }
                if self.indices[*successor].is_none() {
                    self.visit(*successor);
                    self.low_links[node] = self.low_links[node].min(self.low_links[*successor]);
                } else if self.on_stack[*successor] {
                    self.low_links[node] = self.low_links[node]
                        .min(self.indices[*successor].expect("visited node has an index"));
                }
            }

            if self.low_links[node] == index {
                let mut component = Vec::new();
                loop {
                    let member = self.stack.pop().expect("component root is on stack");
                    self.on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                self.components.push(component);
            }
        }
    }

    let length = graph.successors.len();
    let mut tarjan = Tarjan {
        graph,
        reachable,
        next_index: 0,
        indices: vec![None; length],
        low_links: vec![0; length],
        stack: Vec::new(),
        on_stack: vec![false; length],
        components: Vec::new(),
    };
    for (node, is_reachable) in reachable.iter().copied().enumerate().take(length) {
        if is_reachable && tarjan.indices[node].is_none() {
            tarjan.visit(node);
        }
    }
    tarjan.components
}
