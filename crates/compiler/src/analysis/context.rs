use std::collections::HashMap;

use crate::compiler::{InstructionKind, Program};

#[derive(Clone, Copy)]
pub(super) struct ContainingLabel<'a> {
    pub(super) name: &'a str,
    pub(super) start: usize,
}

pub(super) fn containing_labels(program: &Program) -> Vec<Option<ContainingLabel<'_>>> {
    let mut entries: Vec<_> = program
        .labels
        .iter()
        .map(|(name, start)| (name.as_str(), *start))
        .collect();
    entries.sort_unstable_by_key(|(_, start)| *start);
    let mut result = vec![None; program.instructions.len()];
    for (position, (name, start)) in entries.iter().copied().enumerate() {
        let end = entries
            .get(position + 1)
            .map_or(program.instructions.len(), |(_, start)| *start)
            .min(program.instructions.len());
        if start < end {
            result[start..end].fill(Some(ContainingLabel { name, start }));
        }
    }
    result
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct AssignmentSet {
    words: Box<[u64]>,
}

impl AssignmentSet {
    fn empty(variable_count: usize) -> Self {
        Self {
            words: vec![0; variable_count.div_ceil(u64::BITS as usize)].into_boxed_slice(),
        }
    }

    pub(super) fn insert(&mut self, variable: usize) {
        self.words[variable / u64::BITS as usize] |= 1 << (variable % u64::BITS as usize);
    }

    pub(super) fn contains(&self, variable: usize) -> bool {
        self.words[variable / u64::BITS as usize] & (1 << (variable % u64::BITS as usize)) != 0
    }

    pub(super) fn intersection_if_changed(&self, other: &Self) -> Option<Self> {
        self.words
            .iter()
            .zip(&other.words)
            .any(|(current, incoming)| current & incoming != *current)
            .then(|| Self {
                words: self
                    .words
                    .iter()
                    .zip(&other.words)
                    .map(|(current, incoming)| current & incoming)
                    .collect(),
            })
    }
}

pub(super) struct VariableIndex<'a> {
    indexes: HashMap<&'a str, usize>,
}

impl<'a> VariableIndex<'a> {
    pub(super) fn new(program: &'a Program) -> Self {
        let mut indexes = HashMap::new();
        for name in program.defaults.keys().map(String::as_str).chain(
            program
                .instructions
                .iter()
                .filter_map(|instruction| match &instruction.kind {
                    InstructionKind::Set { variable, .. }
                    | InstructionKind::Extension { variable, .. } => Some(variable.as_str()),
                    InstructionKind::Return { value: Some(_) } => Some("_return"),
                    _ => None,
                }),
        ) {
            let next = indexes.len();
            indexes.entry(name).or_insert(next);
        }
        Self { indexes }
    }

    pub(super) fn bit(&self, name: &str) -> Option<usize> {
        self.indexes.get(name).copied()
    }

    pub(super) fn defaults(&self, program: &Program) -> AssignmentSet {
        let mut assigned = AssignmentSet::empty(self.indexes.len());
        for name in program.defaults.keys() {
            assigned.insert(self.bit(name).expect("default was indexed"));
        }
        assigned
    }
}
