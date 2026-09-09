use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::tooling::{
    SymbolKind, SymbolOccurrence, SymbolRole, document_symbols, format_source, is_valid_identifier,
    symbol_at, symbol_occurrences,
};
use serde_json::{Map, Value, json};

#[derive(Default)]
pub(super) struct Workspace {
    documents: HashMap<String, String>,
    open_documents: HashSet<String>,
    root: Option<PathBuf>,
}

impl Workspace {
    pub(super) fn load(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
        let Ok(files) = renrs_project::resources::collect_files(root) else {
            return;
        };
        self.documents
            .retain(|uri, _| self.open_documents.contains(uri));
        for path in files {
            if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rns"))
            {
                let uri = path_uri(&path);
                if !self.open_documents.contains(&uri)
                    && let Ok(source) = fs::read_to_string(path)
                {
                    self.documents.insert(uri, source);
                }
            }
        }
    }

    pub(super) fn insert(&mut self, uri: &str, source: &str) {
        self.documents.insert(uri.to_owned(), source.to_owned());
        self.open_documents.insert(uri.to_owned());
    }

    pub(super) fn remove(&mut self, uri: &str) {
        self.open_documents.remove(uri);
        if let Some(path) = file_uri_path(uri)
            && let Ok(source) = fs::read_to_string(path)
        {
            self.documents.insert(uri.to_owned(), source);
        } else {
            self.documents.remove(uri);
        }
    }

    pub(super) fn refresh(&mut self) {
        if let Some(root) = self.root.clone() {
            self.load(&root);
        }
    }

    pub(super) fn workspace_symbols(&self) -> Vec<Value> {
        self.documents.iter().flat_map(|(uri, source)| symbol_occurrences(source).into_iter()
            .filter(|item| item.role == SymbolRole::Definition)
            .map(move |item| json!({"name": item.name, "kind": if item.kind == SymbolKind::Label { 12 } else { 13 }, "location": location(uri, &item)}))).collect()
    }

    pub(super) fn source(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(String::as_str)
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.documents.len()
    }

    pub(super) fn find_symbol(&self, message: &Value) -> Option<SymbolOccurrence> {
        let uri = message.pointer("/params/textDocument/uri")?.as_str()?;
        let line = usize::try_from(message.pointer("/params/position/line")?.as_u64()?).ok()?;
        let column =
            usize::try_from(message.pointer("/params/position/character")?.as_u64()?).ok()?;
        symbol_at(self.source(uri)?, line, column)
    }

    pub(super) fn occurrences(&self, target: &SymbolOccurrence) -> Vec<(&str, SymbolOccurrence)> {
        self.documents
            .iter()
            .flat_map(|(uri, source)| {
                symbol_occurrences(source)
                    .into_iter()
                    .filter(|item| item.name == target.name && item.kind == target.kind)
                    .map(|item| (uri.as_str(), item))
            })
            .collect()
    }

    pub(super) fn document_symbols(&self, uri: &str) -> Vec<Value> {
        self.source(uri).map_or_else(Vec::new, |source| {
            document_symbols(source)
                .into_iter()
                .map(|symbol| {
                    let kind = match symbol.kind {
                        SymbolKind::Label => 12,
                        SymbolKind::Character
                        | SymbolKind::Image
                        | SymbolKind::DisplayLayer
                        | SymbolKind::Variable => 13,
                    };
                    json!({
                        "name": symbol.name,
                        "kind": kind,
                        "range": line_range(symbol.line, symbol.column),
                        "selectionRange": line_range(symbol.line, symbol.column)
                    })
                })
                .collect()
        })
    }

    pub(super) fn formatting_edits(&self, uri: &str) -> Vec<Value> {
        self.source(uri).map_or_else(Vec::new, |source| {
            vec![json!({
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": source.lines().count() + 1, "character": 0}
                },
                "newText": format_source(source)
            })]
        })
    }

    pub(super) fn definitions(&self, message: &Value) -> Vec<Value> {
        self.find_symbol(message).map_or_else(Vec::new, |target| {
            self.occurrences(&target)
                .into_iter()
                .filter(|(_, occurrence)| occurrence.role == SymbolRole::Definition)
                .map(|(uri, occurrence)| location(uri, &occurrence))
                .collect()
        })
    }

    pub(super) fn references(&self, message: &Value) -> Vec<Value> {
        let include_declaration = message
            .pointer("/params/context/includeDeclaration")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        self.find_symbol(message).map_or_else(Vec::new, |target| {
            self.occurrences(&target)
                .into_iter()
                .filter(|(_, occurrence)| {
                    include_declaration || occurrence.role != SymbolRole::Definition
                })
                .map(|(uri, occurrence)| location(uri, &occurrence))
                .collect()
        })
    }

    pub(super) fn rename(&self, message: &Value, new_name: &str) -> Option<Value> {
        if !is_valid_identifier(new_name) {
            return None;
        }
        let mut changes = Map::new();
        if let Some(target) = self.find_symbol(message) {
            if new_name != target.name
                && self.documents.values().any(|source| {
                    symbol_occurrences(source).iter().any(|item| {
                        item.name == new_name
                            && item.kind == target.kind
                            && item.role == SymbolRole::Definition
                    })
                })
            {
                return None;
            }
            for (uri, occurrence) in self.occurrences(&target) {
                changes
                    .entry(uri.to_owned())
                    .or_insert_with(|| Value::Array(Vec::new()))
                    .as_array_mut()
                    .expect("workspace edit entry is always an array")
                    .push(json!({
                        "range": occurrence_range(&occurrence),
                        "newText": new_name
                    }));
            }
        }
        Some(json!({"changes": changes}))
    }

    pub(super) fn completions(&self) -> Vec<Value> {
        let mut seen = HashSet::new();
        let mut items = Vec::new();
        for source in self.documents.values() {
            for occurrence in symbol_occurrences(source)
                .into_iter()
                .filter(|item| item.role == SymbolRole::Definition)
            {
                if seen.insert((occurrence.name.clone(), occurrence.kind)) {
                    items.push(json!({
                        "label": occurrence.name,
                        "kind": completion_kind(occurrence.kind),
                        "detail": symbol_kind_name(occurrence.kind)
                    }));
                }
            }
        }
        for keyword in KEYWORDS {
            items.push(json!({"label": keyword, "kind": 14, "detail": "keyword"}));
        }
        for name in [
            "list", "record", "get", "put", "push", "remove", "len", "contains",
        ] {
            items.push(
                json!({"label": name, "kind": 3, "detail": "deterministic built-in function"}),
            );
        }
        items
    }
}

const KEYWORDS: [&str; 33] = [
    "fadein",
    "fadeout",
    "nvl",
    "call",
    "clear",
    "hide",
    "if",
    "jump",
    "loop",
    "menu",
    "move",
    "onlayer",
    "order",
    "pause",
    "play",
    "return",
    "scene",
    "set",
    "extend",
    "show",
    "stop",
    "timeline",
    "parallel",
    "transform",
    "transition",
    "video",
    "voice",
    "volume",
    "window",
    "repeat",
    "screen",
    "if_changed",
    "zorder",
];

pub(super) fn workspace_root(message: &Value) -> Option<PathBuf> {
    message
        .pointer("/params/rootUri")
        .and_then(Value::as_str)
        .and_then(file_uri_path)
        .or_else(|| {
            message
                .pointer("/params/rootPath")
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
}

fn line_range(line: usize, column: usize) -> Value {
    json!({
        "start": {"line": line, "character": column},
        "end": {"line": line, "character": column + 1}
    })
}

fn occurrence_range(occurrence: &SymbolOccurrence) -> Value {
    json!({
        "start": {"line": occurrence.line, "character": occurrence.column},
        "end": {"line": occurrence.line, "character": occurrence.column + occurrence.length}
    })
}

fn location(uri: &str, occurrence: &SymbolOccurrence) -> Value {
    json!({"uri": uri, "range": occurrence_range(occurrence)})
}

const fn completion_kind(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Label => 3,
        SymbolKind::Character | SymbolKind::Variable => 6,
        SymbolKind::Image => 17,
        SymbolKind::DisplayLayer => 13,
    }
}

const fn symbol_kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Label => "label",
        SymbolKind::Character => "character",
        SymbolKind::Image => "image",
        SymbolKind::DisplayLayer => "display layer",
        SymbolKind::Variable => "variable",
    }
}

pub(super) fn file_uri_path(uri: &str) -> Option<PathBuf> {
    url::Url::parse(uri).ok()?.to_file_path().ok()
}

pub(super) fn path_uri(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    url::Url::from_file_path(absolute).map_or_else(|()| String::new(), |uri| uri.to_string())
}
