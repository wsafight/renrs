use crate::syntax::Value;
use crate::{Program, Runtime, TranslationId, WaitState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[path = "debugger/explore.rs"]
mod explore;
pub use explore::{Exploration, ExploreLimits, explore};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub name: String,
    pub choices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choice_ids: Vec<TranslationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub expect_label: Option<String>,
    #[serde(default)]
    pub expect_variables: BTreeMap<String, Value>,
    #[serde(default)]
    pub expect_dialogue: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteSuite {
    pub routes: Vec<Route>,
}

#[derive(Debug, Serialize)]
pub struct RouteResult {
    pub name: String,
    pub label: Option<String>,
    pub interactions: usize,
    pub history_length: usize,
    pub variables: BTreeMap<String, Value>,
    pub visited_instructions: usize,
}

pub(super) struct Walk {
    runtime: Runtime,
    steps: usize,
    frontier: Option<usize>,
}

/// Replays one route and verifies its expected ending, state, and optional dialogue.
///
/// # Errors
/// Rejects mismatched projects, stale choice identities, incomplete/extra choices,
/// failed assertions, execution errors, or exceeding the interaction bound.
pub fn run_route(
    program: &Program,
    route: &Route,
    max_steps: usize,
) -> Result<RouteResult, String> {
    let result = walk(program, route, max_steps)?;
    if result.frontier.is_some() {
        return Err(format!("{}: route ends before the next choice", route.name));
    }
    let runtime = result.runtime;
    if let Some(expected) = &route.expect_label
        && runtime.current_label() != Some(expected)
    {
        return Err(format!(
            "{}: expected ending {expected}, reached {:?}",
            route.name,
            runtime.current_label()
        ));
    }
    for (name, expected) in &route.expect_variables {
        if runtime.variables().get(name) != Some(expected) {
            return Err(format!(
                "{}: variable {name} expected {expected:?}, found {:?}",
                route.name,
                runtime.variables().get(name)
            ));
        }
    }
    if let Some(text) = &route.expect_dialogue
        && !runtime
            .history()
            .iter()
            .any(|dialogue| dialogue.text.contains(text))
    {
        return Err(format!(
            "{}: expected dialogue was not reached: {text}",
            route.name
        ));
    }
    Ok(RouteResult {
        name: route.name.clone(),
        label: runtime.current_label().map(ToOwned::to_owned),
        interactions: result.steps,
        history_length: runtime.history().len(),
        variables: runtime.variables().clone(),
        visited_instructions: runtime.visited_instructions().count(),
    })
}

pub(super) fn walk(program: &Program, route: &Route, max_steps: usize) -> Result<Walk, String> {
    if route
        .project_id
        .as_ref()
        .is_some_and(|id| id != &program.project_id)
    {
        return Err("route belongs to another project".to_owned());
    }
    if route
        .fingerprint
        .as_ref()
        .is_some_and(|hash| hash != &program.fingerprint)
    {
        return Err("route was recorded against different content".to_owned());
    }
    if !route.choice_ids.is_empty() && route.choice_ids.len() != route.choices.len() {
        return Err("choice_ids must match choices in length".to_owned());
    }
    let mut runtime = Runtime::new(program.clone()).map_err(|error| error.to_string())?;
    runtime.enable_tracing();
    let mut next_choice = 0;
    let mut state = runtime.advance().map_err(|error| error.to_string())?;
    for step in 1..=max_steps {
        state = match state {
            WaitState::Finished => {
                if next_choice != route.choices.len() {
                    return Err(format!("{}: unused choices after the ending", route.name));
                }
                return Ok(Walk {
                    runtime,
                    steps: step,
                    frontier: None,
                });
            }
            WaitState::Choice { options } => {
                let Some(&choice) = route.choices.get(next_choice) else {
                    return Ok(Walk {
                        runtime,
                        steps: step,
                        frontier: Some(options.len()),
                    });
                };
                let identities = runtime.choice_ids().map_err(|error| error.to_string())?;
                if let Some(expected) = route.choice_ids.get(next_choice)
                    && identities.get(choice) != Some(expected)
                {
                    return Err(format!(
                        "{}: choice {} no longer identifies {expected}",
                        route.name,
                        next_choice + 1
                    ));
                }
                next_choice += 1;
                runtime.choose(choice)
            }
            WaitState::Dialogue | WaitState::Pause { .. } | WaitState::Effect { .. } => {
                runtime.continue_story()
            }
        }
        .map_err(|error| format!("{}: {error}", route.name))?;
        runtime.drain_audio_events().for_each(drop);
    }
    Err(format!("{}: exceeded {max_steps} interactions", route.name))
}
