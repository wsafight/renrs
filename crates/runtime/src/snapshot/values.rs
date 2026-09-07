use crate::syntax::Value;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) enum Node {
    Integer(i64),
    Boolean(bool),
    String(String),
    List(Vec<usize>),
    Record(BTreeMap<String, usize>),
}

#[derive(Default)]
pub(super) struct Encoder {
    nodes: Vec<Node>,
    interned: HashMap<Node, usize>,
    lists: HashMap<usize, usize>,
    records: HashMap<usize, usize>,
}

impl Encoder {
    pub(super) fn record(&mut self, values: &Arc<BTreeMap<String, Value>>) -> usize {
        let address = Arc::as_ptr(values) as usize;
        if let Some(index) = self.records.get(&address) {
            return *index;
        }
        let children = values
            .iter()
            .map(|(name, value)| (name.clone(), self.value(value)))
            .collect();
        let index = self.intern(Node::Record(children));
        self.records.insert(address, index);
        index
    }

    pub(super) fn value(&mut self, value: &Value) -> usize {
        let node = match value {
            Value::Integer(value) => Node::Integer(*value),
            Value::Boolean(value) => Node::Boolean(*value),
            Value::String(value) => Node::String(value.clone()),
            Value::Record(values) => return self.record(values),
            Value::List(values) => {
                let address = Arc::as_ptr(values) as usize;
                if let Some(index) = self.lists.get(&address) {
                    return *index;
                }
                let children = values.iter().map(|value| self.value(value)).collect();
                let index = self.intern(Node::List(children));
                self.lists.insert(address, index);
                return index;
            }
        };
        self.intern(node)
    }

    fn intern(&mut self, node: Node) -> usize {
        if let Some(index) = self.interned.get(&node) {
            return *index;
        }
        let index = self.nodes.len();
        self.interned.insert(node.clone(), index);
        self.nodes.push(node);
        index
    }

    pub(super) fn finish(self) -> Vec<Node> {
        self.nodes
    }
}

pub(super) fn decode(nodes: Vec<Node>) -> Result<Vec<Value>, String> {
    if nodes.len() > 1_000_000 {
        return Err("snapshot value table exceeds its budget".to_owned());
    }
    let mut values = Vec::with_capacity(nodes.len());
    let mut budgets: Vec<(usize, usize, usize)> = Vec::with_capacity(nodes.len());
    // Only backward edges are allowed: cycles and recursive expansion are impossible.
    for node in nodes {
        let mut budget = (1_usize, 0_usize, 0_usize);
        let mut include = |index: usize| -> Result<(), String> {
            let &(items, depth, bytes) = budgets
                .get(index)
                .ok_or("invalid snapshot value reference")?;
            budget.0 = budget.0.saturating_add(items);
            budget.1 = budget.1.max(depth.saturating_add(1));
            budget.2 = budget.2.saturating_add(bytes);
            Ok(())
        };
        match &node {
            Node::List(children) => {
                for index in children {
                    include(*index)?;
                }
            }
            Node::Record(children) => {
                for index in children.values() {
                    include(*index)?;
                }
            }
            Node::String(text) => budget.2 = text.len(),
            Node::Integer(_) | Node::Boolean(_) => {}
        }
        if budget.0 > 1_000_000 || budget.1 > 18 || budget.2 > 64 * 1024 * 1024 {
            return Err("snapshot collection exceeds its decoded budget".to_owned());
        }
        let child = |index: usize| {
            values
                .get(index)
                .cloned()
                .ok_or("invalid snapshot value reference")
        };
        let value = match node {
            Node::Integer(value) => Value::Integer(value),
            Node::Boolean(value) => Value::Boolean(value),
            Node::String(value) => Value::String(value),
            Node::List(children) => Value::List(Arc::new(
                children.into_iter().map(child).collect::<Result<_, _>>()?,
            )),
            Node::Record(children) => Value::Record(Arc::new(
                children
                    .into_iter()
                    .map(|(name, index)| Ok((name, child(index)?)))
                    .collect::<Result<_, String>>()?,
            )),
        };
        values.push(value);
        budgets.push(budget);
    }
    Ok(values)
}
