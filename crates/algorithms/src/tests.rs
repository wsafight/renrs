use super::graph::{Graph, breadth_first};
use std::ops::ControlFlow;

#[test]
fn reachable_marks_the_start_component_only() {
    let successors = vec![vec![1, 2], vec![2], vec![3], vec![]];
    let graph = Graph::new(&successors);
    assert_eq!(graph.reachable(Some(0)), [true, true, true, true]);
    assert_eq!(graph.reachable(Some(3)), [false, false, false, true]);
    assert_eq!(graph.reachable(None), [false, false, false, false]);
    assert_eq!(graph.reachable(Some(99)), [false, false, false, false]);
}

#[test]
fn reachable_ignores_out_of_range_successors() {
    let successors = vec![vec![1, 99], vec![]];
    let graph = Graph::new(&successors);
    assert_eq!(graph.reachable(Some(0)), [true, true]);
}

#[test]
fn strongly_connected_components_finds_cycles_and_skips_excluded_nodes() {
    // 0 <-> 1 cycle, 2 -> 3 cycle, 4 isolated.
    let successors = vec![vec![1], vec![0], vec![3], vec![2], vec![]];
    let graph = Graph::new(&successors);
    let included = [true, true, true, true, false];
    let mut components = graph
        .strongly_connected_components(&included)
        .into_iter()
        .map(|mut component| {
            component.sort_unstable();
            component
        })
        .collect::<Vec<_>>();
    components.sort();
    assert_eq!(components, [vec![0, 1], vec![2, 3]]);
}

#[test]
fn strongly_connected_components_handles_deep_chains_without_recursion() {
    // A 100_000-node chain would overflow the stack under a recursive Tarjan.
    let length = 100_000;
    let successors = (0..length)
        .map(|node| {
            if node + 1 < length {
                vec![node + 1]
            } else {
                vec![]
            }
        })
        .collect::<Vec<_>>();
    let graph = Graph::new(&successors);
    let components = graph.strongly_connected_components(&vec![true; length]);
    assert_eq!(components.len(), length);
}

#[test]
fn breadth_first_visits_each_state_once_and_bounds_cycles() {
    let order = breadth_first((0_usize, 0_u8), 8, |node, _| {
        ControlFlow::Continue(if node < 3 { vec![node + 1] } else { vec![0] })
    });
    assert_eq!(order.len(), 4);
    assert_eq!(order[0], (0, 0));
}

#[test]
fn breadth_first_carries_state_and_stops_at_the_visit_cap() {
    let order = breadth_first((0_usize, 0_u8), 3, |_, depth| {
        *depth += 1;
        ControlFlow::Continue(vec![])
    });
    assert_eq!(order.len(), 1);
    assert_eq!(order[0].1, 0);
}

#[test]
fn breadth_first_stops_when_expansion_breaks() {
    let order = breadth_first((0_usize, ()), 8, |node, ()| {
        if node == 1 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(vec![node + 1])
        }
    });
    assert_eq!(order, [(0, ()), (1, ())]);
}

#[test]
fn empty_graph_has_no_nodes() {
    let graph = Graph::default();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
    assert_eq!(graph.successors(0), []);
}
