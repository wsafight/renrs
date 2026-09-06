use crate::expression::parse_expression;

use super::conversion::{LineConversion, unsupported};

pub(super) fn convert_assignment(content: &str) -> LineConversion {
    let Some((variable, expression)) = content.split_once('=') else {
        return unsupported("only simple assignments can be converted", false);
    };
    let variable = variable.trim();
    if !valid_identifier(variable) {
        return unsupported("assignment target must be one variable", false);
    }
    match convert_expression(expression.trim()) {
        Ok(expression) => LineConversion::One(format!("set {variable} = {expression}")),
        Err(message) => unsupported(&message, false),
    }
}

pub(super) fn convert_default(content: &str) -> LineConversion {
    let Some((variable, expression)) = content.split_once('=') else {
        return unsupported("only simple default declarations can be converted", false);
    };
    let variable = variable.trim();
    if !valid_identifier(variable) {
        return unsupported("default target must be one variable", false);
    }
    match convert_expression(expression.trim()) {
        Ok(expression) => LineConversion::One(format!("default {variable} = {expression}")),
        Err(message) => unsupported(&message, false),
    }
}

pub(super) fn convert_condition(keyword: &str, source: &str) -> LineConversion {
    let Some(expression) = source.strip_suffix(':').map(str::trim) else {
        return unsupported("condition must end with `:`", false);
    };
    match convert_expression(expression) {
        Ok(expression) => LineConversion::One(format!("{keyword} {expression}:")),
        Err(message) => unsupported(&message, true),
    }
}

fn convert_expression(source: &str) -> Result<String, String> {
    if source.is_empty() {
        return Err("expression cannot be empty".to_owned());
    }
    let mut output = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if matches!(character, '\'' | '"') {
            let quote = character;
            output.push(character);
            let mut escaped = false;
            let mut closed = false;
            for current in characters.by_ref() {
                output.push(current);
                if escaped {
                    escaped = false;
                } else if current == '\\' {
                    escaped = true;
                } else if current == quote {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err("expression contains an unterminated string".to_owned());
            }
            continue;
        }
        if is_identifier_start(character) {
            let mut identifier = String::from(character);
            while characters
                .peek()
                .is_some_and(|next| is_identifier_continue(*next))
            {
                identifier.push(
                    characters
                        .next()
                        .expect("peeked expression identifier character exists"),
                );
            }
            match identifier.as_str() {
                "True" => output.push_str("true"),
                "False" => output.push_str("false"),
                "None" => {
                    return Err(
                        "Ren'Py `None` has no equivalent in the RenRS value model".to_owned()
                    );
                }
                _ => output.push_str(&identifier),
            }
        } else {
            output.push(character);
        }
    }
    parse_expression(&output, "<migrated expression>", 1, 1).map_err(|error| {
        format!(
            "expression is outside the supported RenRS subset: {}",
            error.message
        )
    })?;
    Ok(output)
}

pub(super) fn convert_dialogue(content: &str) -> Option<LineConversion> {
    let quote = content.find('"')?;
    let prefix = content[..quote].trim();
    if !prefix.is_empty() && !valid_identifier(prefix) {
        return None;
    }
    let Some(end) = closing_quote(content, quote) else {
        return Some(unsupported("dialogue has an unterminated string", false));
    };
    if !content[end + 1..].trim().is_empty() {
        return Some(unsupported(
            "dialogue attributes, ids, arguments, and `with` clauses require manual migration",
            false,
        ));
    }
    let text = &content[quote + 1..end];
    Some(match convert_dialogue_interpolation(text) {
        Ok(text) => LineConversion::One(format!("{}\"{text}\"", &content[..quote])),
        Err(message) => unsupported(message, false),
    })
}

fn convert_dialogue_interpolation(source: &str) -> Result<String, &'static str> {
    let mut output = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '[' {
            output.push(character);
            continue;
        }
        if characters.peek() == Some(&'[') {
            characters.next();
            output.push('[');
            continue;
        }
        let mut field = String::new();
        let mut closed = false;
        for current in characters.by_ref() {
            if current == ']' {
                closed = true;
                break;
            }
            field.push(current);
        }
        if !closed {
            return Err("dialogue interpolation has an unclosed `[` field");
        }
        let name = field.trim();
        if !valid_identifier(name) {
            return Err(
                "only simple `[variable]` dialogue interpolation can be migrated automatically",
            );
        }
        output.push('{');
        output.push_str(name);
        output.push('}');
    }
    Ok(output)
}

pub(super) fn quoted_argument(source: &str) -> Option<String> {
    let start = source.find('"')?;
    let end = closing_quote(source, start)?;
    Some(unescape_basic(&source[start + 1..end]))
}

pub(super) fn named_quoted_argument(source: &str, name: &str) -> Option<String> {
    let marker = format!("{name}=");
    let start = source.find(&marker)? + marker.len();
    quoted_argument(&source[start..])
}

pub(super) fn closing_quote(source: &str, start: usize) -> Option<usize> {
    let mut escaped = false;
    for (offset, character) in source[start + 1..].char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(start + 1 + offset);
        }
    }
    None
}

fn unescape_basic(source: &str) -> String {
    source.replace("\\\"", "\"").replace("\\\\", "\\")
}

pub(super) fn escape_string(source: &str) -> String {
    source.replace('\\', "\\\\").replace('"', "\\\"")
}

const fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

const fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_ascii_digit()
}

pub(super) fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters.next().is_some_and(is_identifier_start) && characters.all(is_identifier_continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_only_literal_boolean_tokens() {
        assert!(matches!(
            convert_assignment("ready = True"),
            LineConversion::One(ref value) if value == "set ready = true"
        ));
        assert!(matches!(
            convert_assignment("name = \"True and None\""),
            LineConversion::One(ref value) if value.contains("True and None")
        ));
    }

    #[test]
    fn rejects_none_and_calls() {
        assert!(matches!(
            convert_assignment("value = None"),
            LineConversion::Unsupported { ref message, .. } if message.contains("`None`")
        ));
        assert!(matches!(
            convert_assignment("value = make_value()"),
            LineConversion::Unsupported { ref message, .. }
                if message.contains("supported RenRS subset")
        ));
    }
}
