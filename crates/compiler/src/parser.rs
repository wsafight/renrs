use std::path::Path;

use indexmap::IndexMap;

use crate::diagnostic::Diagnostic;
use crate::expression::parse_expression;
use crate::localization::TranslationId;
use crate::syntax::{
    Block, CharacterDef, CropRect, DefaultDef, Easing, ImageDef, MenuOption, Position, Script,
    Span, Statement, StatementKind, TransformProperties, TransitionKind,
};

/// Parses a complete `RenRS` script into a source-located syntax tree.
///
/// # Errors
///
/// Returns all top-level diagnostics that can be recovered in one pass. A
/// nested malformed block may stop recovery inside that declaration.
pub fn parse_script(
    source: &str,
    source_name: impl Into<String>,
) -> Result<Script, Vec<Diagnostic>> {
    let source_name = source_name.into();
    let fragment = parse_fragment(source, source_name.clone())?;
    if !fragment.labels.contains_key("start") {
        return Err(vec![Diagnostic::new(
            &source_name,
            1,
            1,
            "script must define a `start` label",
        )]);
    }
    Ok(fragment.into_script(source_name))
}

/// Checks one editor document without requiring that it contains `start`.
///
/// # Errors
///
/// Returns recoverable syntax diagnostics for the document.
pub fn parse_document(source: &str, source_name: impl Into<String>) -> Result<(), Vec<Diagnostic>> {
    parse_fragment(source, source_name).map(|_| ())
}

#[derive(Debug, Clone)]
pub struct ScriptFragment {
    pub source_name: String,
    pub title: Option<(String, Span)>,
    pub project_id: Option<(String, Span)>,
    pub characters: IndexMap<String, CharacterDef>,
    pub defaults: IndexMap<String, DefaultDef>,
    pub images: IndexMap<String, ImageDef>,
    pub label_parameters: IndexMap<String, Vec<String>>,
    pub labels: IndexMap<String, Block>,
}

impl ScriptFragment {
    fn into_script(self, source_name: String) -> Script {
        let title = self
            .title
            .map_or_else(|| "RenRS Game".to_owned(), |(title, _)| title);
        let project_id = self
            .project_id
            .map_or_else(|| derive_project_id(&title), |(id, _)| id);
        Script {
            source_name,
            title,
            project_id,
            characters: self.characters,
            defaults: self.defaults,
            images: self.images,
            label_parameters: self.label_parameters,
            labels: self.labels,
        }
    }
}

/// Parses one file without requiring a start label, for multi-file project assembly.
/// # Errors
/// Returns syntax diagnostics with source locations.
pub fn parse_fragment(
    source: &str,
    source_name: impl Into<String>,
) -> Result<ScriptFragment, Vec<Diagnostic>> {
    let source_name = source_name.into();
    let lines = preprocess(source, &source_name)?;
    let mut parser = Parser {
        lines,
        current: 0,
        source_name,
    };
    parser.parse()
}

#[derive(Debug, Clone)]
struct Line {
    indent: usize,
    number: usize,
    text: String,
}

fn preprocess(source: &str, source_name: &str) -> Result<Vec<Line>, Vec<Diagnostic>> {
    let mut lines = Vec::new();
    let mut errors = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let number = index + 1;
        let leading = raw
            .chars()
            .take_while(|ch| matches!(ch, ' ' | '\t'))
            .collect::<String>();
        if leading.contains('\t') {
            errors.push(
                Diagnostic::new(
                    source_name,
                    number,
                    1,
                    "tabs are not allowed for indentation",
                )
                .with_hint("use four spaces for each indentation level"),
            );
            continue;
        }
        let indent = leading.len();
        if indent % 4 != 0 {
            errors.push(
                Diagnostic::new(
                    source_name,
                    number,
                    1,
                    "indentation must be a multiple of four spaces",
                )
                .with_hint("align this line with its surrounding block"),
            );
            continue;
        }
        let content = strip_comment(&raw[indent..]).trim_end();
        if !content.trim().is_empty() {
            lines.push(Line {
                indent,
                number,
                text: content.to_owned(),
            });
        }
    }
    if errors.is_empty() {
        Ok(lines)
    } else {
        Err(errors)
    }
}

fn strip_comment(input: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (offset, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '#' if !quoted => return &input[..offset],
            _ => {}
        }
    }
    input
}

struct Parser {
    lines: Vec<Line>,
    current: usize,
    source_name: String,
}

enum ConfigDeclaration {
    Title(String),
    Id(String),
}

mod block;
#[path = "parser/blocks.rs"]
mod blocks;
#[path = "parser/cursor.rs"]
mod cursor;
mod parallel;
#[path = "parser/statement.rs"]
mod statement;
#[path = "parser/top_level.rs"]
mod top_level;

#[cfg(test)]
#[path = "parser/tests.rs"]
mod tests;

pub use cursor::derive_project_id;
pub(crate) use cursor::valid_project_id;
use cursor::{Cursor, default_alias, starts_keyword};
