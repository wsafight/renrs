use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Character,
    Label,
    Image,
    DisplayLayer,
    Variable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolRole {
    Definition,
    Reference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolOccurrence {
    pub name: String,
    pub kind: SymbolKind,
    pub role: SymbolRole,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

#[must_use]
pub fn document_symbols(source: &str) -> Vec<DocumentSymbol> {
    source
        .lines()
        .enumerate()
        .filter_map(|(line, raw)| {
            let trimmed = raw.trim_start();
            let column = raw.len() - trimmed.len();
            if column != 0 {
                return None;
            }
            if let Some(rest) = trimmed.strip_prefix("label ") {
                let name = rest.split(['(', ':']).next()?.trim();
                valid_identifier(name).then(|| DocumentSymbol {
                    name: name.to_owned(),
                    kind: SymbolKind::Label,
                    line,
                    column,
                })
            } else if let Some(rest) = trimmed.strip_prefix("define ") {
                let name = rest.split('=').next()?.trim();
                valid_identifier(name).then(|| DocumentSymbol {
                    name: name.to_owned(),
                    kind: SymbolKind::Character,
                    line,
                    column,
                })
            } else if let Some(rest) = trimmed.strip_prefix("image ") {
                let name = rest.split('=').next()?.trim();
                valid_identifier(name).then(|| DocumentSymbol {
                    name: name.to_owned(),
                    kind: SymbolKind::Image,
                    line,
                    column,
                })
            } else if let Some(rest) = trimmed.strip_prefix("default ") {
                let name = rest.split('=').next()?.trim();
                valid_identifier(name).then(|| DocumentSymbol {
                    name: name.to_owned(),
                    kind: SymbolKind::Variable,
                    line,
                    column,
                })
            } else if let Some(rest) = trimmed.strip_prefix("layer ") {
                let name = rest.split_whitespace().next()?;
                valid_identifier(name).then(|| DocumentSymbol {
                    name: name.to_owned(),
                    kind: SymbolKind::DisplayLayer,
                    line,
                    column,
                })
            } else {
                None
            }
        })
        .collect()
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn symbol_occurrences(source: &str) -> Vec<SymbolOccurrence> {
    let mut occurrences = Vec::new();
    for (line, raw) in source.lines().enumerate() {
        let mut tokens = identifier_tokens(raw);
        if raw.trim_start().starts_with("@id ") {
            while tokens
                .first()
                .is_some_and(|token| matches!(token.0, "id" | "alias"))
            {
                tokens.remove(0);
            }
        }
        let Some(first) = tokens.first() else {
            continue;
        };
        let definition_kind = match first.0 {
            "label" => Some(SymbolKind::Label),
            "define" => Some(SymbolKind::Character),
            "image" => Some(SymbolKind::Image),
            "default" => Some(SymbolKind::Variable),
            "layer" => Some(SymbolKind::DisplayLayer),
            _ => None,
        };
        if let Some(kind) = definition_kind
            && let Some((name, column)) = tokens.get(1)
        {
            occurrences.push(occurrence(
                name,
                kind,
                SymbolRole::Definition,
                line,
                *column,
            ));
        }
        if first.0 == "label" {
            for (name, column) in tokens.iter().skip(2) {
                let role = if label_parameter_starts_at(raw, *column) {
                    SymbolRole::Definition
                } else {
                    SymbolRole::Reference
                };
                if role == SymbolRole::Definition || !language_keyword(name) {
                    occurrences.push(occurrence(name, SymbolKind::Variable, role, line, *column));
                }
            }
        }
        for (name, column) in interpolation_variables(raw) {
            occurrences.push(occurrence(
                name,
                SymbolKind::Variable,
                SymbolRole::Reference,
                line,
                column,
            ));
        }

        match first.0 {
            "jump" | "call" => {
                push_reference(&mut occurrences, &tokens, 1, SymbolKind::Label, line);
                if first.0 == "call" {
                    push_expression_variables(&mut occurrences, &tokens, 2, line);
                }
            }
            "scene" | "show" => {
                push_reference(&mut occurrences, &tokens, 1, SymbolKind::Image, line);
                if let Some(index) = tokens.iter().position(|token| token.0 == "onlayer") {
                    push_reference(
                        &mut occurrences,
                        &tokens,
                        index + 1,
                        SymbolKind::DisplayLayer,
                        line,
                    );
                }
            }
            "clear" => {
                push_reference(&mut occurrences, &tokens, 1, SymbolKind::DisplayLayer, line);
            }
            "set" | "extend" => {
                if let Some((name, column)) = tokens.get(1) {
                    occurrences.push(occurrence(
                        name,
                        SymbolKind::Variable,
                        SymbolRole::Reference,
                        line,
                        *column,
                    ));
                }
                push_expression_variables(&mut occurrences, &tokens, 2, line);
            }
            "if" | "elif" | "return" => {
                push_expression_variables(&mut occurrences, &tokens, 1, line);
            }
            "default" => {
                push_expression_variables(&mut occurrences, &tokens, 2, line);
            }
            keyword
                if !language_keyword(keyword)
                    && raw[first.1 + keyword.len()..].trim_start().starts_with('"') =>
            {
                occurrences.push(occurrence(
                    keyword,
                    SymbolKind::Character,
                    SymbolRole::Reference,
                    line,
                    first.1,
                ));
            }
            _ => {}
        }
        if raw.trim_start().starts_with('"')
            && raw.trim_end().ends_with(':')
            && let Some(condition) = tokens.iter().position(|token| token.0 == "if")
        {
            push_expression_variables(&mut occurrences, &tokens, condition + 1, line);
        }
    }
    let lines: Vec<_> = source.lines().collect();
    for item in &mut occurrences {
        item.column = lines[item.line][..item.column].encode_utf16().count();
    }
    occurrences.sort_by_key(|item| (item.line, item.column));
    occurrences.dedup_by(|left, right| {
        left.line == right.line && left.column == right.column && left.kind == right.kind
    });
    occurrences
}

fn interpolation_variables(raw: &str) -> Vec<(&str, usize)> {
    let mut values = Vec::new();
    let bytes = raw.as_bytes();
    let (mut quoted, mut index) = (false, 0);
    while index < bytes.len() {
        match bytes[index] {
            b'#' if !quoted => break,
            b'\\' if quoted => {
                index += 2;
                continue;
            }
            b'"' => quoted = !quoted,
            b'{' if quoted => {
                if bytes.get(index + 1) == Some(&b'{') {
                    index += 2;
                    continue;
                }
                if let Some(end) = raw[index + 1..].find('}') {
                    let value = &raw[index + 1..index + 1 + end];
                    if valid_identifier(value) && !crate::text::is_text_tag(value) {
                        values.push((value, index + 1));
                    }
                    index += end + 2;
                    continue;
                }
            }
            _ => {}
        }
        index += 1;
    }
    values
}

#[must_use]
pub fn symbol_at(source: &str, line: usize, column: usize) -> Option<SymbolOccurrence> {
    symbol_occurrences(source).into_iter().find(|occurrence| {
        occurrence.line == line
            && (occurrence.column..occurrence.column + occurrence.length).contains(&column)
    })
}

#[must_use]
pub fn is_valid_identifier(value: &str) -> bool {
    valid_identifier(value)
}

fn identifier_tokens(line: &str) -> Vec<(&str, usize)> {
    let bytes = line.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut quoted = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            index += 1;
            continue;
        }
        if byte == b'#' {
            break;
        }
        if byte == b'"' {
            quoted = true;
            index += 1;
            continue;
        }
        if byte == b'_' || byte.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index] == b'_' || bytes[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            tokens.push((&line[start..index], start));
        } else {
            index += 1;
        }
    }
    tokens
}

fn label_parameter_starts_at(line: &str, column: usize) -> bool {
    let Some(opening) = line.find('(').filter(|opening| *opening < column) else {
        return false;
    };
    let bytes = line.as_bytes();
    let mut item_start = opening + 1;
    let (mut parentheses, mut brackets, mut braces) = (0_usize, 0_usize, 0_usize);
    let (mut quoted, mut escaped) = (false, false);
    for (index, byte) in bytes
        .iter()
        .copied()
        .enumerate()
        .take(column)
        .skip(opening + 1)
    {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'(' => parentheses += 1,
            b')' => parentheses = parentheses.saturating_sub(1),
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b'{' => braces += 1,
            b'}' => braces = braces.saturating_sub(1),
            b',' if parentheses == 0 && brackets == 0 && braces == 0 => item_start = index + 1,
            _ => {}
        }
    }
    bytes[item_start..column]
        .iter()
        .all(u8::is_ascii_whitespace)
}

fn push_reference(
    occurrences: &mut Vec<SymbolOccurrence>,
    tokens: &[(&str, usize)],
    index: usize,
    kind: SymbolKind,
    line: usize,
) {
    if let Some((name, column)) = tokens.get(index) {
        occurrences.push(occurrence(name, kind, SymbolRole::Reference, line, *column));
    }
}

fn push_expression_variables(
    occurrences: &mut Vec<SymbolOccurrence>,
    tokens: &[(&str, usize)],
    start: usize,
    line: usize,
) {
    occurrences.extend(
        tokens
            .iter()
            .skip(start)
            .filter(|(name, _)| !language_keyword(name))
            .map(|(name, column)| {
                occurrence(
                    name,
                    SymbolKind::Variable,
                    SymbolRole::Reference,
                    line,
                    *column,
                )
            }),
    );
}

fn occurrence(
    name: &str,
    kind: SymbolKind,
    role: SymbolRole,
    line: usize,
    column: usize,
) -> SymbolOccurrence {
    SymbolOccurrence {
        name: name.to_owned(),
        kind,
        role,
        line,
        column,
        length: name.len(),
    }
}

fn language_keyword(value: &str) -> bool {
    matches!(
        value,
        "and"
            | "as"
            | "at"
            | "call"
            | "center"
            | "clear"
            | "color"
            | "default"
            | "define"
            | "elif"
            | "else"
            | "false"
            | "fadein"
            | "fadeout"
            | "hide"
            | "id"
            | "if"
            | "image"
            | "jump"
            | "label"
            | "layer"
            | "left"
            | "loop"
            | "menu"
            | "move"
            | "music"
            | "not"
            | "or"
            | "onlayer"
            | "order"
            | "over"
            | "pause"
            | "play"
            | "return"
            | "right"
            | "scene"
            | "set"
            | "show"
            | "sound"
            | "stop"
            | "to"
            | "timeline"
            | "parallel"
            | "nvl"
            | "transform"
            | "transition"
            | "true"
            | "video"
            | "voice"
            | "volume"
            | "zorder"
    )
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
