use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextStyle {
    pub bold: bool,
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruby: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledText {
    pub plain: String,
    pub runs: Vec<TextRun>,
}

#[must_use]
pub fn is_text_tag(tag: &str) -> bool {
    matches!(tag, "b" | "/b" | "u" | "/u" | "/ruby" | "br" | "/color")
        || tag.starts_with("color=")
        || tag.starts_with("ruby=")
}

/// Validates markup while treating non-tag braces as interpolation fields.
///
/// # Errors
///
/// Returns the same structural tag errors as [`parse_text_markup`]. Variable
/// names and escaped braces are replaced with inert text before parsing.
pub fn validate_text_source(input: &str) -> Result<(), String> {
    let mut normalized = String::new();
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '{' {
            if characters.peek() == Some(&'{') {
                characters.next();
                normalized.push_str("literal");
                continue;
            }
            let mut field = String::new();
            loop {
                match characters.next() {
                    Some('}') => break,
                    Some(current) => field.push(current),
                    None => return Err("unclosed `{` in dialogue interpolation".to_owned()),
                }
            }
            if is_text_tag(field.trim()) {
                normalized.push('{');
                normalized.push_str(field.trim());
                normalized.push('}');
            } else {
                normalized.push_str("value");
            }
        } else if character == '}' && characters.peek() == Some(&'}') {
            characters.next();
            normalized.push('}');
        } else {
            normalized.push(character);
        }
    }
    parse_text_markup(&normalized).map(|_| ())
}

/// Parses `RenRS` dialogue tags after variable interpolation.
///
/// # Errors
///
/// Returns an error for invalid colors, unmatched closing tags, or unclosed
/// bold/color spans. Unknown brace content remains literal text.
#[allow(clippy::too_many_lines)]
pub fn parse_text_markup(input: &str) -> Result<StyledText, String> {
    let mut plain = String::new();
    let mut runs = Vec::new();
    let mut buffer = String::new();
    let mut bold_depth = 0_usize;
    let mut colors = Vec::<String>::new();
    let mut underline_depth = 0_usize;
    let mut ruby = None;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        if character != '{' {
            buffer.push(character);
            continue;
        }
        let mut tag = String::new();
        let mut closed = false;
        for current in characters.by_ref() {
            if current == '}' {
                closed = true;
                break;
            }
            tag.push(current);
        }
        if !closed {
            buffer.push('{');
            buffer.push_str(&tag);
            break;
        }
        if !is_text_tag(&tag) {
            buffer.push('{');
            buffer.push_str(&tag);
            buffer.push('}');
            continue;
        }

        flush_run(
            &mut buffer,
            &mut plain,
            &mut runs,
            TextStyle {
                bold: bold_depth > 0,
                color: colors.last().cloned(),
                underline: underline_depth > 0,
                ruby: ruby.clone(),
            },
        );
        match tag.as_str() {
            "b" => bold_depth += 1,
            "u" => underline_depth += 1,
            "/u" if underline_depth > 0 => underline_depth -= 1,
            "/u" => return Err("unmatched underline closing tag".to_owned()),
            "/ruby" if ruby.is_some() => {
                ruby = None;
            }
            "/ruby" => return Err("unmatched ruby closing tag".to_owned()),
            value if value.starts_with("ruby=") => {
                let annotation = &value[5..];
                if ruby.is_some()
                    || annotation.is_empty()
                    || annotation.chars().count() > 64
                    || annotation.chars().any(char::is_control)
                {
                    return Err("ruby requires 1..64 characters and cannot be nested".to_owned());
                }
                ruby = Some(annotation.to_owned());
            }
            "/b" if bold_depth > 0 => bold_depth -= 1,
            "/b" => return Err("text tag `{/b}` has no matching `{b}`".to_owned()),
            "br" => append_run(
                "\n".to_owned(),
                &mut plain,
                &mut runs,
                TextStyle {
                    bold: bold_depth > 0,
                    color: colors.last().cloned(),
                    underline: underline_depth > 0,
                    ruby: ruby.clone(),
                },
            ),
            "/color" if !colors.is_empty() => {
                colors.pop();
            }
            "/color" => {
                return Err("text tag `{/color}` has no matching `{color=...}`".to_owned());
            }
            value if value.starts_with("color=") => {
                let color = &value["color=".len()..];
                if !valid_color(color) {
                    return Err(format!(
                        "text color `{color}` must use #RRGGBB or #RRGGBBAA"
                    ));
                }
                colors.push(color.to_owned());
            }
            _ => unreachable!("recognized tags are handled above"),
        }
    }
    flush_run(
        &mut buffer,
        &mut plain,
        &mut runs,
        TextStyle {
            bold: bold_depth > 0,
            color: colors.last().cloned(),
            underline: underline_depth > 0,
            ruby: ruby.clone(),
        },
    );
    if bold_depth > 0 {
        return Err("text tag `{b}` is not closed".to_owned());
    }
    if !colors.is_empty() {
        return Err("text tag `{color=...}` is not closed".to_owned());
    }
    if underline_depth > 0 || ruby.is_some() {
        return Err("unclosed underline or ruby tag".to_owned());
    }
    Ok(StyledText { plain, runs })
}

fn flush_run(buffer: &mut String, plain: &mut String, runs: &mut Vec<TextRun>, style: TextStyle) {
    if buffer.is_empty() {
        return;
    }
    append_run(std::mem::take(buffer), plain, runs, style);
}

fn append_run(text: String, plain: &mut String, runs: &mut Vec<TextRun>, style: TextStyle) {
    plain.push_str(&text);
    if let Some(previous) = runs.last_mut()
        && previous.style == style
    {
        previous.text.push_str(&text);
    } else {
        runs.push(TextRun { text, style });
    }
}

fn valid_color(color: &str) -> bool {
    let Some(hex) = color.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ruby_and_underline_preserve_plain_text() {
        let text = parse_text_markup("{ruby=signal}{u}word{/u}{/ruby}").unwrap();
        assert_eq!(text.plain, "word");
        assert_eq!(text.runs[0].style.ruby.as_deref(), Some("signal"));
        assert!(text.runs[0].style.underline);
        assert!(parse_text_markup("{ruby=a}{ruby=b}x{/ruby}{/ruby}").is_err());
    }

    #[test]
    fn parses_bold_color_and_line_breaks() {
        let styled =
            parse_text_markup("Plain {b}bold {color=#ff0080}pink{/color}{/b}{br}next").unwrap();
        assert_eq!(styled.plain, "Plain bold pink\nnext");
        assert!(styled.runs.iter().any(|run| run.style.bold));
        assert!(
            styled
                .runs
                .iter()
                .any(|run| run.style.color.as_deref() == Some("#ff0080"))
        );
    }

    #[test]
    fn rejects_unbalanced_tags() {
        assert!(parse_text_markup("{b}never closed").is_err());
        assert!(parse_text_markup("{/color}").is_err());
        assert!(validate_text_source("Hello {name}, {b}open").is_err());
        assert!(validate_text_source("Literal {{brace}}").is_ok());
    }
}
