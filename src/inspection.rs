use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;

use crate::compiler::InstructionKind;
use crate::debugger::{RouteResult, RouteSuite, run_route};
use crate::localization::{TranslationCatalog, extract_catalog};
use crate::runtime::RuntimeSnapshot;
use crate::{Diagnostic, Program, ProjectSource, Runtime, analyze};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInspection {
    pub engine_version: &'static str,
    pub source_kind: &'static str,
    pub title: String,
    pub project_id: String,
    pub fingerprint: String,
    pub snapshot_format: u32,
    pub save_container_format: u32,
    pub instruction_count: usize,
    pub script_files: Vec<String>,
    pub characters: Vec<CharacterInspection>,
    pub variables: Vec<VariableInspection>,
    pub labels: Vec<LabelInspection>,
    pub endings: Vec<EndingInspection>,
    pub resources: Vec<ResourceInspection>,
    pub localization: Vec<LocalizationInspection>,
    pub routes: RouteInspection,
}

#[derive(Debug, Clone, Serialize)]
pub struct CharacterInspection {
    pub id: String,
    pub name: String,
    pub color: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariableInspection {
    pub name: String,
    pub value_type: String,
    pub initial_value: crate::syntax::Value,
    pub persistent: bool,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabelInspection {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub statically_reachable: bool,
    pub route_covered: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndingInspection {
    pub id: String,
    pub title: String,
    pub label: String,
    pub route_covered: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceInspection {
    pub path: String,
    pub kind: &'static str,
    pub referenced_by_story: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalizationInspection {
    pub path: String,
    pub language: String,
    pub fallback: Option<String>,
    pub source_messages: usize,
    pub translated_messages: usize,
    pub missing: Vec<String>,
    pub obsolete: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteInspection {
    pub configured: bool,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<RouteCheck>,
    pub coverage: CoverageInspection,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteCheck {
    pub name: String,
    pub expected_label: Option<String>,
    pub passed: bool,
    pub result: Option<RouteResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageInspection {
    pub visited_instructions: usize,
    pub total_instructions: usize,
    pub percent: f64,
    pub covered_labels: Vec<String>,
    pub uncovered_labels: Vec<String>,
}

pub struct InspectionFailure {
    pub code: &'static str,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Compiles and describes the project's current, read-only state.
///
/// # Errors
/// Returns source-aware project diagnostics or malformed sidecar errors.
pub fn inspect_project(
    path: &Path,
) -> Result<(ProjectInspection, Vec<Diagnostic>), InspectionFailure> {
    let source = ProjectSource::open(path.to_path_buf()).map_err(|error| InspectionFailure {
        code: crate::protocol::code::PROJECT_INVALID,
        message: error.to_string(),
        diagnostics: Vec::new(),
    })?;
    let program = source.compile().map_err(|diagnostics| InspectionFailure {
        code: crate::protocol::code::PROJECT_INVALID,
        message: "project compilation failed".to_owned(),
        diagnostics,
    })?;
    let diagnostics = analyze(&program);
    let resources = source.resource_names().map_err(|error| InspectionFailure {
        code: crate::protocol::code::PROJECT_INVALID,
        message: error.to_string(),
        diagnostics: Vec::new(),
    })?;
    let routes = inspect_routes(&source, &program)?;
    let covered = covered_instruction_ids(&routes);
    let reachable = crate::analysis::reachable_labels(&program);
    let runtime = Runtime::new(program.clone()).map_err(|error| InspectionFailure {
        code: crate::protocol::code::RUNTIME_FAILED,
        message: error.to_string(),
        diagnostics: Vec::new(),
    })?;
    let labels = labels(&program, &reachable, &covered);
    let inspection = ProjectInspection {
        engine_version: env!("CARGO_PKG_VERSION"),
        source_kind: match &source {
            ProjectSource::Directory(_) => "directory",
            ProjectSource::Archive(_) => "archive",
        },
        title: program.title.clone(),
        project_id: program.project_id.clone(),
        fingerprint: program.fingerprint.clone(),
        snapshot_format: RuntimeSnapshot::FORMAT_VERSION,
        save_container_format: crate::save_format::SaveFile::CONTAINER_VERSION,
        instruction_count: program.instructions.len(),
        script_files: resources
            .iter()
            .filter(|path| extension(path) == "rns")
            .cloned()
            .collect(),
        characters: characters(&program),
        variables: variables(&program, &runtime),
        endings: endings(&program, &covered),
        labels,
        resources: inspect_resources(resources, &program),
        localization: inspect_localization(&source, &program)?,
        routes,
    };
    Ok((inspection, diagnostics))
}

fn characters(program: &Program) -> Vec<CharacterInspection> {
    program
        .characters
        .iter()
        .map(|(id, value)| CharacterInspection {
            id: id.clone(),
            name: value.name.clone(),
            color: value.color.clone(),
            file: value.span.source.clone(),
            line: value.span.line,
        })
        .collect()
}

fn variables(program: &Program, runtime: &Runtime) -> Vec<VariableInspection> {
    program
        .defaults
        .iter()
        .filter_map(|(name, definition)| {
            runtime
                .variables()
                .get(name)
                .map(|value| VariableInspection {
                    name: name.clone(),
                    value_type: value.type_name().to_owned(),
                    initial_value: value.clone(),
                    persistent: name.starts_with("persistent_"),
                    file: definition.span.source.clone(),
                    line: definition.span.line,
                })
        })
        .collect()
}

fn labels(
    program: &Program,
    reachable: &BTreeSet<String>,
    covered: &BTreeSet<String>,
) -> Vec<LabelInspection> {
    program
        .labels
        .iter()
        .map(|(name, index)| {
            let instruction = &program.instructions[*index];
            LabelInspection {
                name: name.clone(),
                file: instruction.span.source.clone(),
                line: instruction.span.line,
                statically_reachable: reachable.contains(name),
                route_covered: covered.contains(instruction.id.as_str()),
            }
        })
        .collect()
}

fn endings(program: &Program, covered: &BTreeSet<String>) -> Vec<EndingInspection> {
    program
        .progress
        .endings
        .iter()
        .map(|ending| EndingInspection {
            id: ending.id.clone(),
            title: ending.title.clone(),
            label: ending.label.clone(),
            route_covered: program
                .labels
                .get(&ending.label)
                .and_then(|index| program.instruction_id(*index))
                .is_some_and(|id| covered.contains(id.as_str())),
        })
        .collect()
}

fn inspect_routes(
    source: &ProjectSource,
    program: &Program,
) -> Result<RouteInspection, InspectionFailure> {
    let configured = source.contains("routes.json");
    let suite = if configured {
        serde_json::from_slice::<RouteSuite>(
            &source
                .read("routes.json")
                .map_err(|error| sidecar_error(error.to_string()))?,
        )
        .map_err(|error| sidecar_error(error.to_string()))?
    } else {
        RouteSuite { routes: Vec::new() }
    };
    let results = suite
        .routes
        .iter()
        .map(|route| match run_route(program, route, 10_000) {
            Ok(result) => RouteCheck {
                name: route.name.clone(),
                expected_label: route.expect_label.clone(),
                passed: true,
                result: Some(result),
                error: None,
            },
            Err(error) => RouteCheck {
                name: route.name.clone(),
                expected_label: route.expect_label.clone(),
                passed: false,
                result: None,
                error: Some(error),
            },
        })
        .collect::<Vec<_>>();
    let covered = covered_instruction_ids_from_checks(&results);
    let (covered_labels, uncovered_labels) = coverage_labels(program, &covered);
    let visited = covered.len();
    let total = program.instructions.len();
    Ok(RouteInspection {
        configured,
        passed: results.iter().filter(|result| result.passed).count(),
        failed: results.iter().filter(|result| !result.passed).count(),
        results,
        coverage: CoverageInspection {
            visited_instructions: visited,
            total_instructions: total,
            percent: coverage_percent(visited, total),
            covered_labels,
            uncovered_labels,
        },
    })
}

fn covered_instruction_ids(routes: &RouteInspection) -> BTreeSet<String> {
    covered_instruction_ids_from_checks(&routes.results)
}

fn covered_instruction_ids_from_checks(routes: &[RouteCheck]) -> BTreeSet<String> {
    routes
        .iter()
        .filter_map(|check| check.result.as_ref())
        .flat_map(|result| result.visited_instruction_ids.iter())
        .map(ToString::to_string)
        .collect()
}

fn coverage_labels(program: &Program, covered: &BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    let mut covered_labels = Vec::new();
    let mut uncovered_labels = Vec::new();
    for (name, index) in &program.labels {
        let is_covered = program
            .instruction_id(*index)
            .is_some_and(|id| covered.contains(id.as_str()));
        if is_covered {
            covered_labels.push(name.clone());
        } else {
            uncovered_labels.push(name.clone());
        }
    }
    (covered_labels, uncovered_labels)
}

fn inspect_resources(names: Vec<String>, program: &Program) -> Vec<ResourceInspection> {
    let referenced = referenced_resources(program);
    names
        .into_iter()
        .map(|path| ResourceInspection {
            kind: resource_kind(&path),
            referenced_by_story: referenced.contains(&path),
            path,
        })
        .collect()
}

fn referenced_resources(program: &Program) -> BTreeSet<String> {
    let mut resources = BTreeSet::new();
    for instruction in &program.instructions {
        match &instruction.kind {
            InstructionKind::Video { path, .. }
            | InstructionKind::Scene { path }
            | InstructionKind::Show { path, .. }
            | InstructionKind::PlayMusic { path, .. }
            | InstructionKind::QueueMusic { path, .. }
            | InstructionKind::PlaySound { path, .. }
            | InstructionKind::PlayVoice { path } => {
                resources.insert(path.clone());
            }
            _ => {}
        }
    }
    resources.extend(
        program
            .layered_images
            .values()
            .flat_map(|image| image.layers.iter())
            .flat_map(crate::syntax::ImageLayer::paths)
            .map(str::to_owned),
    );
    resources.extend(
        program
            .progress
            .gallery
            .iter()
            .filter_map(|item| item.image.clone()),
    );
    resources
}

fn inspect_localization(
    source: &ProjectSource,
    program: &Program,
) -> Result<Vec<LocalizationInspection>, InspectionFailure> {
    let source_ids = extract_catalog(program)
        .into_iter()
        .map(|message| message.id.to_string())
        .collect::<BTreeSet<_>>();
    let names = source.resource_names().map_err(|error| InspectionFailure {
        code: crate::protocol::code::PROJECT_INVALID,
        message: error.to_string(),
        diagnostics: Vec::new(),
    })?;
    names
        .into_iter()
        .filter(|path| path.starts_with("locales/") && extension(path) == "json")
        .map(|path| {
            let catalog = TranslationCatalog::from_reader(std::io::Cursor::new(
                source
                    .read(&path)
                    .map_err(|error| sidecar_error(error.to_string()))?,
            ))
            .map_err(|error| sidecar_error(error.to_string()))?;
            let entries = catalog
                .messages
                .keys()
                .chain(catalog.plurals.keys())
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>();
            let translated = catalog
                .messages
                .iter()
                .filter(|(_, value)| !value.is_empty())
                .map(|(id, _)| id.to_string())
                .chain(catalog.plurals.keys().map(ToString::to_string))
                .collect::<BTreeSet<_>>();
            Ok(LocalizationInspection {
                path,
                language: catalog.language,
                fallback: catalog.fallback,
                source_messages: source_ids.len(),
                translated_messages: source_ids.intersection(&translated).count(),
                missing: source_ids.difference(&translated).cloned().collect(),
                obsolete: entries.difference(&source_ids).cloned().collect(),
            })
        })
        .collect()
}

fn coverage_percent(visited: usize, total: usize) -> f64 {
    if total == 0 {
        return 100.0;
    }
    let basis_points = visited.saturating_mul(10_000) / total;
    f64::from(u32::try_from(basis_points).unwrap_or(10_000)) / 100.0
}

fn sidecar_error(error: String) -> InspectionFailure {
    InspectionFailure {
        code: crate::protocol::code::PROJECT_INVALID,
        message: error,
        diagnostics: Vec::new(),
    }
}

fn resource_kind(path: &str) -> &'static str {
    match extension(path) {
        "rns" => "script",
        "png" | "jpg" | "jpeg" | "webp" | "tga" => "image",
        "wav" | "ogg" | "mp3" | "flac" => "audio",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "json" if path.starts_with("locales/") => "localization",
        "json" => "configuration",
        _ => "other",
    }
}

fn extension(path: &str) -> &str {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
}
