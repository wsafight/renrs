use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::archive::ResourceArchive;
use crate::diagnostic::Diagnostic;
use crate::parser::{ScriptFragment, derive_project_id, parse_fragment};
use crate::syntax::{Block, CharacterDef, DefaultDef, DisplayLayerDef, ImageDef, Script, Span};

/// Loads every non-hidden `.rns` file below a game directory in sorted order.
///
/// Declarations share one project namespace. Exactly one `start` label is
/// required across the project, while `config title` may be omitted or
/// declared once.
///
/// # Errors
///
/// Returns filesystem, parse, duplicate-declaration, and missing-entry-point
/// diagnostics. Files that can be read are still parsed so authors receive as
/// many useful diagnostics as possible in one run.
pub fn load_project(game_root: &Path) -> Result<Script, Vec<Diagnostic>> {
    let files = collect_scripts(game_root)?;
    let mut sources = Vec::new();
    let mut diagnostics = Vec::new();
    let mut total_bytes = 0;
    for path in files {
        let source_name = renrs_shared::path::relative_name(game_root, &path);
        let result = fs::File::open(&path)
            .and_then(|file| {
                crate::resource_budget::read_limited(file, crate::resource_budget::SCRIPT_BYTES)
            })
            .and_then(|bytes| crate::resource_budget::decode_script(bytes, &mut total_bytes));
        match result {
            Ok(source) => sources.push((source_name, source)),
            Err(error) => diagnostics.push(Diagnostic::new(
                &source_name,
                1,
                1,
                format!("could not read script: {error}"),
            )),
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    load_sources(sources, &game_root.display().to_string())
}

/// Loads all visible `.rns` entries directly from a resource archive.
///
/// # Errors
///
/// Returns archive read, UTF-8, parse, duplicate-declaration, and missing-entry
/// diagnostics without extracting files to disk.
pub fn load_project_archive(archive: &ResourceArchive) -> Result<Script, Vec<Diagnostic>> {
    let mut names = archive
        .entries()
        .iter()
        .map(|entry| entry.path.as_str())
        .filter(|path| visible_script_path(path))
        .collect::<Vec<_>>();
    names.sort_unstable();
    let mut sources = Vec::with_capacity(names.len());
    let mut diagnostics = Vec::new();
    let mut total_bytes = 0;
    for name in names {
        let result = archive
            .read_limited(name, crate::resource_budget::SCRIPT_BYTES)
            .and_then(|bytes| {
                crate::resource_budget::decode_script(bytes, &mut total_bytes)
                    .map_err(crate::archive::ArchiveError::Io)
            });
        match result {
            Ok(source) => sources.push((name.to_owned(), source)),
            Err(error) => diagnostics.push(Diagnostic::new(
                name,
                1,
                1,
                format!("could not read script: {error}"),
            )),
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    load_sources(sources, &archive.path().display().to_string())
}

fn load_sources(
    sources: Vec<(String, String)>,
    project_name: &str,
) -> Result<Script, Vec<Diagnostic>> {
    if sources.is_empty() {
        return Err(vec![Diagnostic::new(
            project_name,
            1,
            1,
            "project does not contain any `.rns` files",
        )]);
    }
    let mut fragments = Vec::with_capacity(sources.len());
    let mut diagnostics = Vec::new();
    for (source_name, source) in sources {
        match parse_fragment(&source, &source_name) {
            Ok(fragment) => fragments.push(fragment),
            Err(errors) => diagnostics.extend(errors),
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    merge_fragments(fragments)
}

fn visible_script_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rns"))
        && path
            .split('/')
            .all(|component| !component.starts_with('.') && !component.is_empty())
}

fn collect_scripts(game_root: &Path) -> Result<Vec<PathBuf>, Vec<Diagnostic>> {
    crate::resources::collect_files(game_root)
        .map(|files| {
            files
                .into_iter()
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("rns"))
                })
                .collect()
        })
        .map_err(|error| {
            vec![Diagnostic::new(
                game_root.display().to_string(),
                1,
                1,
                error.to_string(),
            )]
        })
}

#[allow(clippy::too_many_lines)]
pub(crate) fn merge_fragments(fragments: Vec<ScriptFragment>) -> Result<Script, Vec<Diagnostic>> {
    let mut title: Option<(String, Span)> = None;
    let mut project_id: Option<(String, Span)> = None;
    let mut characters: IndexMap<String, CharacterDef> = IndexMap::new();
    let mut defaults: IndexMap<String, DefaultDef> = IndexMap::new();
    let mut images: IndexMap<String, ImageDef> = IndexMap::new();
    let mut display_layers: IndexMap<String, DisplayLayerDef> = IndexMap::new();
    let mut transforms: IndexMap<String, crate::syntax::NamedTransform> = IndexMap::new();
    let mut label_parameters: IndexMap<String, Vec<crate::syntax::LabelParameter>> =
        IndexMap::new();
    let mut labels: IndexMap<String, Block> = IndexMap::new();
    let mut diagnostics = Vec::new();

    for fragment in fragments {
        let ScriptFragment {
            source_name,
            title: fragment_title,
            project_id: fragment_project_id,
            characters: fragment_characters,
            defaults: fragment_defaults,
            images: fragment_images,
            display_layers: fragment_display_layers,
            transforms: fragment_transforms,
            label_parameters: fragment_label_parameters,
            labels: fragment_labels,
        } = fragment;
        if let Some((value, span)) = fragment_title {
            if let Some((_, first)) = &title {
                diagnostics.push(
                    at(&span, "`config title` is declared more than once").with_hint(format!(
                        "the first declaration is at {}:{}:{}",
                        first.source, first.line, first.column
                    )),
                );
            } else {
                title = Some((value, span));
            }
        }
        if let Some((value, span)) = fragment_project_id {
            if let Some((_, first)) = &project_id {
                diagnostics.push(
                    at(&span, "`config id` is declared more than once").with_hint(format!(
                        "the first declaration is at {}:{}:{}",
                        first.source, first.line, first.column
                    )),
                );
            } else {
                project_id = Some((value, span));
            }
        }
        for (id, character) in fragment_characters {
            if let Some(first) = characters.get(&id) {
                diagnostics.push(
                    at(
                        &character.span,
                        format!("character `{id}` is defined more than once"),
                    )
                    .with_hint(format!(
                        "the first definition is at {}:{}:{}",
                        first.span.source, first.span.line, first.span.column
                    )),
                );
            } else {
                characters.insert(id, character);
            }
        }
        for (name, definition) in fragment_defaults {
            if let Some(first) = defaults.get(&name) {
                diagnostics.push(
                    at(
                        &definition.span,
                        format!("default `{name}` is declared more than once"),
                    )
                    .with_hint(format!(
                        "the first declaration is at {}:{}:{}",
                        first.span.source, first.span.line, first.span.column
                    )),
                );
            } else {
                defaults.insert(name, definition);
            }
        }
        for (name, definition) in fragment_images {
            if let Some(first) = images.get(&name) {
                diagnostics.push(
                    at(
                        &definition.span,
                        format!("image `{name}` is declared more than once"),
                    )
                    .with_hint(format!(
                        "the first declaration is at {}:{}:{}",
                        first.span.source, first.span.line, first.span.column
                    )),
                );
            } else {
                images.insert(name, definition);
            }
        }
        for (name, definition) in fragment_display_layers {
            if let Some(first) = display_layers.get(&name) {
                diagnostics.push(
                    at(
                        &definition.span,
                        format!("display layer `{name}` is declared more than once"),
                    )
                    .with_hint(format!(
                        "the first declaration is at {}:{}:{}",
                        first.span.source, first.span.line, first.span.column
                    )),
                );
            } else {
                display_layers.insert(name, definition);
            }
        }
        for (name, definition) in fragment_transforms {
            if let Some(first) = transforms.get(&name) {
                diagnostics.push(
                    at(
                        &definition.span,
                        format!("transform `{name}` is declared more than once"),
                    )
                    .with_hint(format!(
                        "the first declaration is at {}:{}:{}",
                        first.span.source, first.span.line, first.span.column
                    )),
                );
            } else {
                transforms.insert(name, definition);
            }
        }
        for (name, block) in fragment_labels {
            if let Some(first) = labels.get(&name) {
                let span = block.first().map_or_else(
                    || Span::in_source(&source_name, 1, 1),
                    |statement| statement.span.clone(),
                );
                let first_span = first
                    .first()
                    .map_or_else(|| Span::new(1, 1), |statement| statement.span.clone());
                diagnostics.push(
                    at(&span, format!("label `{name}` is defined more than once")).with_hint(
                        format!(
                            "the first definition is in {} near line {}",
                            first_span.source, first_span.line
                        ),
                    ),
                );
            } else {
                label_parameters.insert(
                    name.clone(),
                    fragment_label_parameters
                        .get(&name)
                        .cloned()
                        .unwrap_or_default(),
                );
                labels.insert(name, block);
            }
        }
    }

    if !labels.contains_key("start") {
        diagnostics.push(Diagnostic::new(
            ".",
            1,
            1,
            "project must define a `start` label",
        ));
    }
    if diagnostics.is_empty() {
        let title = title.map_or_else(|| "RenRS Game".to_owned(), |(value, _)| value);
        Ok(Script {
            source_name: ".".to_owned(),
            project_id: project_id.map_or_else(|| derive_project_id(&title), |(value, _)| value),
            title,
            characters,
            defaults,
            images,
            display_layers,
            transforms,
            label_parameters,
            labels,
        })
    } else {
        Err(diagnostics)
    }
}

fn at(span: &Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(&span.source, span.line, span.column, message)
}

#[cfg(test)]
mod tests;
