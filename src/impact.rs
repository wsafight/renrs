use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;

use crate::compiler::InstructionId;
use crate::debugger::{Route, RouteResult, RouteSuite, run_route};
use crate::localization::extract_catalog;
use crate::{Diagnostic, Program, ProjectSource, Runtime};

#[path = "impact/save.rs"]
mod save;
#[path = "impact/translation.rs"]
mod translation;
use save::save_compatibility;
use translation::{load_catalogs, translation_impact};

#[derive(Debug, Serialize)]
pub struct ImpactReport {
    pub baseline: ProjectVersion,
    pub candidate: ProjectVersion,
    pub changed: bool,
    pub labels: LabelChanges,
    pub routes: Vec<RouteImpact>,
    pub endings: Vec<EndingImpact>,
    pub new_unreachable_labels: Vec<String>,
    pub translations: TranslationImpact,
    pub save_compatibility: SaveCompatibility,
}

#[derive(Debug, Serialize)]
pub struct ProjectVersion {
    pub project_id: String,
    pub fingerprint: String,
}

#[derive(Debug, Serialize)]
pub struct LabelChanges {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub renamed: Vec<LabelRename>,
}

#[derive(Debug, Serialize)]
pub struct LabelRename {
    pub from: String,
    pub to: String,
    pub stable_instruction: String,
}

#[derive(Debug, Serialize)]
pub struct RouteImpact {
    pub name: String,
    pub reasons: Vec<&'static str>,
    pub baseline: Option<RouteOutcome>,
    pub candidate: Option<RouteOutcome>,
}

#[derive(Debug, Serialize)]
pub struct RouteOutcome {
    pub passed: bool,
    pub label: Option<String>,
    pub variables: BTreeMap<String, crate::syntax::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EndingImpact {
    pub id: String,
    pub reasons: Vec<&'static str>,
    pub baseline: Option<EndingState>,
    pub candidate: Option<EndingState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EndingState {
    pub title: String,
    pub label: String,
    pub statically_reachable: bool,
    pub route_covered: bool,
}

#[derive(Debug, Serialize)]
pub struct TranslationImpact {
    pub source_ids_added: Vec<String>,
    pub source_ids_removed: Vec<String>,
    pub source_ids_changed: Vec<String>,
    pub catalogs: Vec<CatalogImpact>,
}

#[derive(Debug, Serialize)]
pub struct CatalogImpact {
    pub language: String,
    pub change: &'static str,
    pub ids_added: Vec<String>,
    pub ids_removed: Vec<String>,
    pub ids_changed: Vec<String>,
    pub missing_after: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SaveCompatibility {
    pub risk: &'static str,
    pub requires_save_fixture_validation: bool,
    pub reasons: Vec<String>,
    pub defaults_added: Vec<String>,
    pub defaults_removed: Vec<String>,
    pub default_types_changed: Vec<TypeChange>,
    pub stable_positions_removed: Vec<String>,
    pub stable_positions_aliased: Vec<StableAlias>,
    pub unstable_interaction_positions: usize,
    pub unstable_call_sites: usize,
}

#[derive(Debug, Serialize)]
pub struct TypeChange {
    pub name: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct StableAlias {
    pub from: String,
    pub to: String,
}

pub struct ImpactFailure {
    pub code: &'static str,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

struct State {
    program: Program,
    routes: BTreeMap<String, Route>,
    reachable: BTreeSet<String>,
    covered: BTreeSet<String>,
    route_results: BTreeMap<String, Result<RouteResult, String>>,
    source_messages: BTreeMap<String, serde_json::Value>,
    catalogs: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    defaults: BTreeMap<String, String>,
}

/// Compares two complete, compiled project sources and reports semantic impact.
///
/// # Errors
/// Returns diagnostics when either project or a required sidecar is invalid.
pub fn analyze_impact(baseline: &Path, candidate: &Path) -> Result<ImpactReport, ImpactFailure> {
    let baseline = State::load(baseline)?;
    let candidate = State::load(candidate)?;
    let labels = label_changes(&baseline.program, &candidate.program);
    let routes = route_impacts(&baseline, &candidate);
    let endings = ending_impacts(&baseline, &candidate);
    let translations = translation_impact(&baseline, &candidate);
    let save_compatibility = save_compatibility(&baseline, &candidate);
    let new_unreachable_labels = candidate
        .program
        .labels
        .keys()
        .filter(|name| {
            !candidate.reachable.contains(*name)
                && (baseline.reachable.contains(*name)
                    || !baseline.program.labels.contains_key(*name))
        })
        .cloned()
        .collect::<Vec<_>>();
    let changed = baseline.program.fingerprint != candidate.program.fingerprint
        || baseline.program.project_id != candidate.program.project_id
        || !routes.is_empty()
        || !endings.is_empty()
        || !new_unreachable_labels.is_empty()
        || !translations.source_ids_added.is_empty()
        || !translations.source_ids_removed.is_empty()
        || !translations.source_ids_changed.is_empty()
        || !translations.catalogs.is_empty();
    Ok(ImpactReport {
        baseline: version(&baseline.program),
        candidate: version(&candidate.program),
        changed,
        labels,
        routes,
        endings,
        new_unreachable_labels,
        translations,
        save_compatibility,
    })
}

impl State {
    fn load(path: &Path) -> Result<Self, ImpactFailure> {
        let source = ProjectSource::open(path.to_path_buf())
            .map_err(|error| failure(error.to_string(), Vec::new()))?;
        let program = source.compile().map_err(|diagnostics| ImpactFailure {
            code: crate::protocol::code::PROJECT_INVALID,
            message: format!("project compilation failed: {}", path.display()),
            diagnostics,
        })?;
        let routes = load_routes(&source)?;
        let route_results = routes
            .iter()
            .map(|(name, route)| (name.clone(), run_route(&program, route, 10_000)))
            .collect::<BTreeMap<_, _>>();
        let covered = route_results
            .values()
            .filter_map(|result| result.as_ref().ok())
            .flat_map(|result| result.visited_instruction_ids.iter())
            .map(ToString::to_string)
            .collect();
        let source_messages = extract_catalog(&program)
            .into_iter()
            .map(|message| {
                let id = message.id.to_string();
                (
                    id,
                    serde_json::to_value(message).expect("translation source serializes"),
                )
            })
            .collect();
        let catalogs = load_catalogs(&source)?;
        let runtime = Runtime::new(program.clone())
            .map_err(|error| failure(error.to_string(), Vec::new()))?;
        let defaults = runtime
            .variables()
            .iter()
            .map(|(name, value)| (name.clone(), value.type_name().to_owned()))
            .collect();
        let reachable = crate::analysis::reachable_labels(&program);
        Ok(Self {
            program,
            routes,
            reachable,
            covered,
            route_results,
            source_messages,
            catalogs,
            defaults,
        })
    }
}

fn version(program: &Program) -> ProjectVersion {
    ProjectVersion {
        project_id: program.project_id.clone(),
        fingerprint: program.fingerprint.clone(),
    }
}

fn label_changes(baseline: &Program, candidate: &Program) -> LabelChanges {
    let baseline_names = baseline.labels.keys().cloned().collect::<BTreeSet<_>>();
    let candidate_names = candidate.labels.keys().cloned().collect::<BTreeSet<_>>();
    let mut added = candidate_names
        .difference(&baseline_names)
        .cloned()
        .collect::<Vec<_>>();
    let mut removed = baseline_names
        .difference(&candidate_names)
        .cloned()
        .collect::<Vec<_>>();
    let mut renamed = Vec::new();
    removed.retain(|from| {
        let Some(id) = label_id(baseline, from) else {
            return true;
        };
        let Some(to) = added
            .iter()
            .find(|to| label_id(candidate, to).is_some_and(|candidate_id| candidate_id == id))
            .cloned()
        else {
            return true;
        };
        added.retain(|name| name != &to);
        renamed.push(LabelRename {
            from: from.clone(),
            to,
            stable_instruction: id.to_string(),
        });
        false
    });
    LabelChanges {
        added,
        removed,
        renamed,
    }
}

fn label_id<'a>(program: &'a Program, name: &str) -> Option<&'a InstructionId> {
    program
        .labels
        .get(name)
        .and_then(|index| program.instruction_id(*index))
}

fn route_impacts(baseline: &State, candidate: &State) -> Vec<RouteImpact> {
    union_keys(&baseline.routes, &candidate.routes)
        .into_iter()
        .filter_map(|name| {
            let before = baseline.routes.get(&name);
            let after = candidate.routes.get(&name);
            let before_result = baseline.route_results.get(&name);
            let after_result = candidate.route_results.get(&name);
            let mut reasons = Vec::new();
            match (before, after) {
                (None, Some(_)) => reasons.push("route_added"),
                (Some(_), None) => reasons.push("route_removed"),
                (Some(before), Some(after)) if json(before) != json(after) => {
                    reasons.push("definition_changed");
                }
                _ => {}
            }
            if outcome_signature(before_result) != outcome_signature(after_result) {
                reasons.push("result_changed");
            }
            if executed_content_changed(baseline, candidate, before_result, after_result) {
                reasons.push("executed_content_changed");
            }
            (!reasons.is_empty()).then(|| RouteImpact {
                name,
                reasons,
                baseline: before_result.map(route_outcome),
                candidate: after_result.map(route_outcome),
            })
        })
        .collect()
}

fn ending_impacts(baseline: &State, candidate: &State) -> Vec<EndingImpact> {
    let before = ending_states(baseline);
    let after = ending_states(candidate);
    union_keys(&before, &after)
        .into_iter()
        .filter_map(|id| {
            let old = before.get(&id);
            let new = after.get(&id);
            let mut reasons = Vec::new();
            match (old, new) {
                (None, Some(_)) => reasons.push("ending_added"),
                (Some(_), None) => reasons.push("ending_removed"),
                (Some(old), Some(new)) => {
                    if old.title != new.title || old.label != new.label {
                        reasons.push("definition_changed");
                    }
                    if old.statically_reachable != new.statically_reachable {
                        reasons.push("reachability_changed");
                    }
                    if old.route_covered != new.route_covered {
                        reasons.push("route_coverage_changed");
                    }
                }
                (None, None) => {}
            }
            (!reasons.is_empty()).then(|| EndingImpact {
                id,
                reasons,
                baseline: old.cloned(),
                candidate: new.cloned(),
            })
        })
        .collect()
}

fn ending_states(state: &State) -> BTreeMap<String, EndingState> {
    state
        .program
        .progress
        .endings
        .iter()
        .map(|ending| {
            let covered = label_id(&state.program, &ending.label)
                .is_some_and(|id| state.covered.contains(id.as_str()));
            (
                ending.id.clone(),
                EndingState {
                    title: ending.title.clone(),
                    label: ending.label.clone(),
                    statically_reachable: state.reachable.contains(&ending.label),
                    route_covered: covered,
                },
            )
        })
        .collect()
}

fn load_routes(source: &ProjectSource) -> Result<BTreeMap<String, Route>, ImpactFailure> {
    if !source.contains("routes.json") {
        return Ok(BTreeMap::new());
    }
    let bytes = source
        .read("routes.json")
        .map_err(|error| failure(error.to_string(), Vec::new()))?;
    let suite: RouteSuite =
        serde_json::from_slice(&bytes).map_err(|error| failure(error.to_string(), Vec::new()))?;
    let mut routes = BTreeMap::new();
    for route in suite.routes {
        if routes.insert(route.name.clone(), route).is_some() {
            return Err(failure(
                "routes.json contains duplicate route names".to_owned(),
                Vec::new(),
            ));
        }
    }
    Ok(routes)
}

fn route_outcome(result: &Result<RouteResult, String>) -> RouteOutcome {
    match result {
        Ok(result) => RouteOutcome {
            passed: true,
            label: result.label.clone(),
            variables: result.variables.clone(),
            error: None,
        },
        Err(error) => RouteOutcome {
            passed: false,
            label: None,
            variables: BTreeMap::new(),
            error: Some(error.clone()),
        },
    }
}

fn outcome_signature(result: Option<&Result<RouteResult, String>>) -> Option<serde_json::Value> {
    result.map(|result| json(&route_outcome(result)))
}

fn executed_content_changed(
    baseline: &State,
    candidate: &State,
    before: Option<&Result<RouteResult, String>>,
    after: Option<&Result<RouteResult, String>>,
) -> bool {
    let (Some(Ok(before)), Some(Ok(after))) = (before, after) else {
        return false;
    };
    let before = instruction_signatures(&baseline.program, &before.visited_instruction_ids);
    let after = instruction_signatures(&candidate.program, &after.visited_instruction_ids);
    before != after
}

fn instruction_signatures(
    program: &Program,
    ids: &[InstructionId],
) -> BTreeMap<String, serde_json::Value> {
    ids.iter()
        .map(|id| {
            let value = program
                .instruction_index(id)
                .and_then(|index| program.instructions.get(index))
                .map_or(serde_json::Value::Null, |instruction| {
                    json(&instruction.kind)
                });
            (id.to_string(), value)
        })
        .collect()
}

fn union_keys<V, W>(
    baseline: &BTreeMap<String, V>,
    candidate: &BTreeMap<String, W>,
) -> BTreeSet<String> {
    baseline.keys().chain(candidate.keys()).cloned().collect()
}

fn json(value: &impl Serialize) -> serde_json::Value {
    serde_json::to_value(value).expect("report value serializes")
}

fn failure(error: String, diagnostics: Vec<Diagnostic>) -> ImpactFailure {
    ImpactFailure {
        code: crate::protocol::code::PROJECT_INVALID,
        message: error,
        diagnostics,
    }
}
