use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum StaticValue {
    String(String),
    Number(f32),
    Boolean(bool),
    None,
    Reference(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct StaticAssignment {
    pub(super) target: String,
    pub(super) value: StaticValue,
}

pub(super) fn parse_assignment(line: &str) -> Option<StaticAssignment> {
    let line = line.trim_start_matches('\u{feff}').trim();
    let rest = line
        .strip_prefix("define ")
        .or_else(|| line.strip_prefix("default "))?;
    let (target, value) = rest.split_once('=')?;
    let target = target.trim();
    if !valid_path(target) {
        return None;
    }
    Some(StaticAssignment {
        target: target.to_owned(),
        value: parse_value(value.trim())?,
    })
}

pub(super) fn resolve<'a>(
    values: &'a BTreeMap<String, StaticValue>,
    target: &str,
) -> Option<&'a StaticValue> {
    let mut current = values.get(target)?;
    for _ in 0..16 {
        match current {
            StaticValue::Reference(reference) => current = values.get(reference)?,
            _ => return Some(current),
        }
    }
    None
}

fn parse_value(source: &str) -> Option<StaticValue> {
    if let Some(inner) = source
        .strip_prefix("_(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_value(inner.trim());
    }
    if source == "True" {
        return Some(StaticValue::Boolean(true));
    }
    if source == "False" {
        return Some(StaticValue::Boolean(false));
    }
    if source == "None" {
        return Some(StaticValue::None);
    }
    if let Some(value) = quoted(source) {
        return Some(StaticValue::String(value));
    }
    if let Ok(value) = source.parse::<f32>()
        && value.is_finite()
    {
        return Some(StaticValue::Number(value));
    }
    valid_path(source).then(|| StaticValue::Reference(source.to_owned()))
}

fn quoted(source: &str) -> Option<String> {
    let quote = source.chars().next()?;
    if !matches!(quote, '\'' | '"') || !source.ends_with(quote) || source.len() < 2 {
        return None;
    }
    let body = &source[quote.len_utf8()..source.len() - quote.len_utf8()];
    let mut result = String::with_capacity(body.len());
    let mut escaped = false;
    for character in body.chars() {
        if escaped {
            result.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    if escaped {
        result.push('\\');
    }
    Some(result)
}

fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
                && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_literals_translation_wrappers_and_aliases() {
        assert_eq!(
            parse_assignment("define config.name = _(\"Story\")")
                .unwrap()
                .value,
            StaticValue::String("Story".to_owned())
        );
        assert_eq!(
            parse_assignment("define gui.button_size = gui.text_size")
                .unwrap()
                .value,
            StaticValue::Reference("gui.text_size".to_owned())
        );
        assert_eq!(
            parse_assignment("default preferences.afm_time = 15")
                .unwrap()
                .value,
            StaticValue::Number(15.0)
        );
    }
}
