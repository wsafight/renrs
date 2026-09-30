//! Graph traversal over adjacency lists.
//!
//! Callers build the adjacency list themselves; this module owns only the
//! algorithms that were previously duplicated across the compiler, runtime and
//! tooling crates.

use std::collections::{HashSet, VecDeque};
use std::ops::ControlFlow;

/// Directed graph borrowing one successor list per node.
///
/// Successor indices are expected to be valid node indices. Out-of-range
/// entries are ignored by the traversals rather than panicking.
#[derive(Debug, Clone, Copy, Default)]
pub struct Graph<'a> {
    successors: &'a [Vec<usize>],
}

impl<'a> Graph<'a> {
    /// Creates a graph over existing successor lists, one entry per node.
    #[must_use]
    pub const fn new(successors: &'a [Vec<usize>]) -> Self {
        Self { successors }
    }

    /// Number of nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.successors.len()
    }

    /// Whether the graph has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.successors.is_empty()
    }

    /// Successors of `node`, or an empty slice for an out-of-range node.
    #[must_use]
    pub fn successors(&self, node: usize) -> &[usize] {
        self.successors.get(node).map_or(&[], Vec::as_slice)
    }

    /// Marks every node reachable from `start`.
    ///
    /// Returns one flag per node. An absent or out-of-range `start` reaches
    /// nothing.
    #[must_use]
    pub fn reachable(&self, start: Option<usize>) -> Vec<bool> {
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
                    .filter(|next| *next < reachable.len() && !reachable[*next]),
            );
        }
        reachable
    }

    /// Strongly connected components of the subgraph induced by `included`.
    ///
    /// Uses an explicit-stack Tarjan so deep control-flow graphs cannot exhaust
    /// the call stack. Components come back in Tarjan's completion order, each
    /// listing its members.
    #[must_use]
    pub fn strongly_connected_components(&self, included: &[bool]) -> Vec<Vec<usize>> {
        let length = self.successors.len();
        let mut state = Tarjan {
            graph: *self,
            included,
            next_index: 0,
            indices: vec![None; length],
            low_links: vec![0; length],
            stack: Vec::new(),
            on_stack: vec![false; length],
            components: Vec::new(),
        };
        for node in 0..length {
            if included.get(node).copied().unwrap_or(false) && state.indices[node].is_none() {
                state.visit(node);
            }
        }
        state.components
    }
}

/// Iterative Tarjan state. `pending` records each node's resume point, which
/// keeps the depth-first walk free of recursion.
struct Tarjan<'a> {
    graph: Graph<'a>,
    included: &'a [bool],
    next_index: usize,
    indices: Vec<Option<usize>>,
    low_links: Vec<usize>,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    components: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    fn visit(&mut self, root: usize) {
        let mut pending = vec![(root, 0_usize)];
        while let Some(&(node, next_successor)) = pending.last() {
            if next_successor == 0 {
                let index = self.next_index;
                self.next_index += 1;
                self.indices[node] = Some(index);
                self.low_links[node] = index;
                self.stack.push(node);
                self.on_stack[node] = true;
            }
            let successor = self.graph.successors(node).get(next_successor).copied();
            if let Some(successor) = successor {
                let last = pending.len() - 1;
                pending[last].1 += 1;
                if !self.included.get(successor).copied().unwrap_or(false) {
                    continue;
                }
                match self.indices[successor] {
                    None => pending.push((successor, 0)),
                    Some(index) if self.on_stack[successor] => {
                        self.low_links[node] = self.low_links[node].min(index);
                    }
                    Some(_) => {}
                }
                continue;
            }
            pending.pop();
            if self.indices[node] == Some(self.low_links[node]) {
                let mut component = Vec::new();
                while let Some(member) = self.stack.pop() {
                    self.on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                self.components.push(component);
            }
            if let Some(&(parent, _)) = pending.last() {
                self.low_links[parent] = self.low_links[parent].min(self.low_links[node]);
            }
        }
    }
}

/// Breadth-first traversal over `(node, state)` pairs.
///
/// Visited pairs are returned in dequeue order. `expand` yields the successors
/// of a pair and may mutate the carried state, so callers can thread a call
/// stack or a depth without this module knowing about it. Returning
/// [`ControlFlow::Break`] stops the traversal. Both the visit count and the
/// pending queue are capped at `max_visits`, so cyclic graphs terminate.
pub fn breadth_first<N, S, F>(start: (N, S), max_visits: usize, mut expand: F) -> Vec<(N, S)>
where
    N: Copy + Eq + std::hash::Hash,
    S: Clone + Eq + std::hash::Hash,
    F: FnMut(N, &mut S) -> ControlFlow<(), Vec<N>>,
{
    let mut queue = VecDeque::from([start]);
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    while let Some((node, mut state)) = queue.pop_front() {
        if order.len() >= max_visits {
            break;
        }
        if !visited.insert((node, state.clone())) {
            continue;
        }
        order.push((node, state.clone()));
        let ControlFlow::Continue(successors) = expand(node, &mut state) else {
            break;
        };
        for successor in successors {
            if queue.len() >= max_visits {
                break;
            }
            queue.push_back((successor, state.clone()));
        }
    }
    order
}
