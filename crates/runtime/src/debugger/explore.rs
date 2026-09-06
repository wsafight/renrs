use super::{Route, walk};
use crate::Program;
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy)]
pub struct ExploreLimits {
    pub max_runs: usize,
    pub max_steps: usize,
    pub max_depth: usize,
}

impl Default for ExploreLimits {
    fn default() -> Self {
        Self {
            max_runs: 128,
            max_steps: 10_000,
            max_depth: 32,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Exploration {
    pub complete: bool,
    pub runs: usize,
    pub finished_paths: Vec<Route>,
    pub failures: Vec<String>,
    pub truncated_branches: usize,
    pub visited_instructions: usize,
    pub total_instructions: usize,
    pub uncovered_labels: Vec<String>,
}

/// Explores menu branches by replaying bounded choice prefixes from the start.
///
/// # Errors
/// Returns an error for zero or excessively large bounds. Script failures and
/// unfinished exploration are reported explicitly in the result.
pub fn explore(program: &Program, limits: ExploreLimits) -> Result<Exploration, String> {
    if !(1..=10_000).contains(&limits.max_runs)
        || !(1..=1_000_000).contains(&limits.max_steps)
        || !(1..=256).contains(&limits.max_depth)
    {
        return Err("bounds must be runs=1..10000, steps=1..1000000, depth=1..256".to_owned());
    }
    let mut result = Exploration {
        complete: true,
        runs: 0,
        finished_paths: Vec::new(),
        failures: Vec::new(),
        truncated_branches: 0,
        visited_instructions: 0,
        total_instructions: program.instructions.len(),
        uncovered_labels: Vec::new(),
    };
    let mut visited = BTreeSet::new();
    let mut pending = vec![Vec::new()];
    while let Some(choices) = pending.pop() {
        if result.runs >= limits.max_runs {
            result.truncated_branches += 1 + pending.len();
            break;
        }
        result.runs += 1;
        let route = Route {
            name: format!("path-{}", result.runs),
            choices,
            ..Route::default()
        };
        match walk(program, &route, limits.max_steps) {
            Ok(walk) => {
                visited.extend(walk.runtime.visited_instructions());
                if let Some(count) = walk.frontier {
                    if route.choices.len() >= limits.max_depth {
                        result.truncated_branches += count;
                        continue;
                    }
                    let capacity = limits.max_runs.saturating_sub(result.runs + pending.len());
                    result.truncated_branches += count.saturating_sub(capacity);
                    for choice in (0..count.min(capacity)).rev() {
                        let mut branch = route.choices.clone();
                        branch.push(choice);
                        pending.push(branch);
                    }
                } else {
                    result.finished_paths.push(Route {
                        expect_label: walk.runtime.current_label().map(ToOwned::to_owned),
                        expect_variables: walk.runtime.variables().clone(),
                        ..route
                    });
                }
            }
            Err(error) => result.failures.push(error),
        }
    }
    result.complete = result.truncated_branches == 0 && result.failures.is_empty();
    result.visited_instructions = visited.len();
    result.uncovered_labels = program
        .labels
        .iter()
        .filter(|(_, index)| !visited.contains(index))
        .map(|(name, _)| name.clone())
        .collect();
    Ok(result)
}
