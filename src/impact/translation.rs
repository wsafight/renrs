use std::collections::BTreeMap;
use std::path::Path;

use crate::ProjectSource;
use crate::localization::TranslationCatalog;

use super::{CatalogImpact, ImpactFailure, State, TranslationImpact, failure, union_keys};

pub(super) fn translation_impact(baseline: &State, candidate: &State) -> TranslationImpact {
    let (source_ids_added, source_ids_removed, source_ids_changed) =
        map_changes(&baseline.source_messages, &candidate.source_messages);
    let catalogs = union_keys(&baseline.catalogs, &candidate.catalogs)
        .into_iter()
        .filter_map(|language| {
            let before = baseline.catalogs.get(&language);
            let after = candidate.catalogs.get(&language);
            let empty = BTreeMap::new();
            let (ids_added, ids_removed, ids_changed) =
                map_changes(before.unwrap_or(&empty), after.unwrap_or(&empty));
            let missing_before = missing(&baseline.source_messages, before);
            let missing_after = missing(&candidate.source_messages, after);
            let entries_changed =
                !ids_added.is_empty() || !ids_removed.is_empty() || !ids_changed.is_empty();
            let change = match (before, after, entries_changed) {
                (None, Some(_), _) => "catalog_added",
                (Some(_), None, _) => "catalog_removed",
                (Some(_), Some(_), true) => "catalog_modified",
                (Some(_), Some(_), false) if missing_before != missing_after => "source_changed",
                (Some(_), Some(_), false) | (None, None, _) => return None,
            };
            Some(CatalogImpact {
                language,
                change,
                ids_added,
                ids_removed,
                ids_changed,
                missing_after,
            })
        })
        .collect();
    TranslationImpact {
        source_ids_added,
        source_ids_removed,
        source_ids_changed,
        catalogs,
    }
}

fn missing(
    sources: &BTreeMap<String, serde_json::Value>,
    entries: Option<&BTreeMap<String, serde_json::Value>>,
) -> Vec<String> {
    sources
        .keys()
        .filter(|id| {
            entries
                .and_then(|values| values.get(*id))
                .is_none_or(|value| value.is_null() || value.as_str().is_some_and(str::is_empty))
        })
        .cloned()
        .collect()
}

pub(super) fn load_catalogs(
    source: &ProjectSource,
) -> Result<BTreeMap<String, BTreeMap<String, serde_json::Value>>, ImpactFailure> {
    let mut catalogs = BTreeMap::new();
    for path in source
        .resource_names()
        .map_err(|error| failure(error.to_string(), Vec::new()))?
        .into_iter()
        .filter(|path| {
            path.starts_with("locales/")
                && Path::new(path)
                    .extension()
                    .is_some_and(|value| value == "json")
        })
    {
        let catalog = TranslationCatalog::from_reader(std::io::Cursor::new(
            source
                .read(&path)
                .map_err(|error| failure(error.to_string(), Vec::new()))?,
        ))
        .map_err(|error| failure(error.to_string(), Vec::new()))?;
        let mut entries = catalog
            .messages
            .into_iter()
            .map(|(id, value)| (id.to_string(), serde_json::Value::String(value)))
            .collect::<BTreeMap<_, _>>();
        entries.extend(catalog.plurals.into_iter().map(|(id, value)| {
            (
                id.to_string(),
                serde_json::to_value(value).expect("plural serializes"),
            )
        }));
        catalogs.insert(catalog.language, entries);
    }
    Ok(catalogs)
}

fn map_changes<V: PartialEq>(
    baseline: &BTreeMap<String, V>,
    candidate: &BTreeMap<String, V>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let added = candidate
        .keys()
        .filter(|key| !baseline.contains_key(*key))
        .cloned()
        .collect();
    let removed = baseline
        .keys()
        .filter(|key| !candidate.contains_key(*key))
        .cloned()
        .collect();
    let changed = baseline
        .iter()
        .filter_map(|(key, value)| {
            candidate
                .get(key)
                .filter(|candidate_value| *candidate_value != value)
                .map(|_| key.clone())
        })
        .collect();
    (added, removed, changed)
}
