use crate::diagnostic::Diagnostic;
use std::borrow::Cow;
use std::collections::HashSet;

use crate::syntax::{Builtin, Expr, StrPart, Value};

/// Parses and statically checks a bounded story expression at its source location.
///
/// `RenRS` keeps literal square brackets inside strings and excludes Velin's
/// random functions from its deterministic story-expression profile.
/// # Errors
/// Returns a diagnostic for invalid syntax or a provable type error.
pub fn parse_expression(
    input: &str,
    file: &str,
    line: usize,
    base_column: usize,
) -> Result<Expr, Diagnostic> {
    parse_checked(input, file, line, base_column, false)
}

/// Parses an expression that must produce a boolean when its type is known.
/// # Errors
/// Returns a diagnostic for invalid syntax, a provable type error, or a
/// statically non-boolean condition.
pub fn parse_condition(
    input: &str,
    file: &str,
    line: usize,
    base_column: usize,
) -> Result<Expr, Diagnostic> {
    parse_checked(input, file, line, base_column, true)
}

fn parse_checked(
    input: &str,
    file: &str,
    line: usize,
    base_column: usize,
    condition: bool,
) -> Result<Expr, Diagnostic> {
    let prepared = mask_string_brackets(input);
    let expression =
        velin_parse::parse_expression(prepared.source.as_ref(), file, line, base_column)?;
    let environment = velin_check::Environment::new();
    let diagnostics = if condition {
        velin_check::check_condition(&expression, &environment, file, line, base_column)
    } else {
        velin_check::check_expression(&expression, &environment, file, line, base_column)
    };
    if let Some(error) = diagnostics.into_iter().next() {
        return Err(error);
    }
    normalize(expression, None, file, line, base_column, prepared.markers)
}

fn normalize(
    expression: Expr,
    location: Option<(usize, usize)>,
    file: &str,
    line: usize,
    column: usize,
    markers: Option<(char, char)>,
) -> Result<Expr, Diagnostic> {
    match expression {
        Expr::Spanned { span, expression } => normalize(
            *expression,
            Some((span.line, span.column)),
            file,
            line,
            column,
            markers,
        ),
        Expr::Invoke {
            function: Builtin::Random | Builtin::Chance,
            ..
        } => {
            let (line, column) = location.unwrap_or((line, column));
            Err(Diagnostic::new(
                file,
                line,
                column,
                "random and chance are not available in RenRS expressions",
            ))
        }
        Expr::Invoke {
            function,
            arguments,
        } => Ok(Expr::Invoke {
            function,
            arguments: arguments
                .into_iter()
                .map(|argument| normalize(argument, None, file, line, column, markers))
                .collect::<Result<_, _>>()?,
        }),
        Expr::Unary { op, value } => Ok(Expr::Unary {
            op,
            value: Box::new(normalize(*value, None, file, line, column, markers)?),
        }),
        Expr::Binary { left, op, right } => Ok(Expr::Binary {
            left: Box::new(normalize(*left, None, file, line, column, markers)?),
            op,
            right: Box::new(normalize(*right, None, file, line, column, markers)?),
        }),
        Expr::Interpolate { parts } => Ok(Expr::Interpolate {
            parts: parts
                .into_iter()
                .map(|part| match part {
                    StrPart::Literal(text) => Ok(StrPart::Literal(restore_string(text, markers))),
                    StrPart::Hole(expression) => {
                        normalize(*expression, None, file, line, column, markers)
                            .map(|expression| StrPart::Hole(Box::new(expression)))
                    }
                })
                .collect::<Result<_, _>>()?,
        }),
        Expr::Value(Value::String(text)) => {
            if markers.is_some_and(|markers| {
                text.as_str().contains(markers.0) || text.as_str().contains(markers.1)
            }) {
                Ok(Expr::Value(Value::String(
                    restore_string(text.as_str().to_owned(), markers).into(),
                )))
            } else {
                Ok(Expr::Value(Value::String(text)))
            }
        }
        Expr::Value(value) => Ok(Expr::Value(value)),
        Expr::Variable(name) => Ok(Expr::Variable(name)),
    }
}

struct PreparedExpression<'a> {
    source: Cow<'a, str>,
    markers: Option<(char, char)>,
}

fn mask_string_brackets(input: &str) -> PreparedExpression<'_> {
    if !contains_string_brackets(input) {
        return PreparedExpression {
            source: Cow::Borrowed(input),
            markers: None,
        };
    }

    let markers = unused_markers(input);
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in input.chars() {
        if !in_string {
            output.push(character);
            if character == '"' {
                in_string = true;
            }
            continue;
        }
        if escaped {
            output.push(character);
            escaped = false;
        } else {
            match character {
                '\\' => {
                    output.push(character);
                    escaped = true;
                }
                '"' => {
                    output.push(character);
                    in_string = false;
                }
                '[' => output.push(markers.0),
                ']' => output.push(markers.1),
                _ => output.push(character),
            }
        }
    }
    PreparedExpression {
        source: Cow::Owned(output),
        markers: Some(markers),
    }
}

fn contains_string_brackets(input: &str) -> bool {
    if !input.contains('[') && !input.contains(']') {
        return false;
    }

    let mut in_string = false;
    let mut escaped = false;
    for character in input.chars() {
        if !in_string {
            in_string = character == '"';
        } else if escaped {
            escaped = false;
        } else {
            match character {
                '\\' => escaped = true,
                '"' => in_string = false,
                '[' | ']' => return true,
                _ => {}
            }
        }
    }
    false
}

fn unused_markers(input: &str) -> (char, char) {
    const COMMON: (char, char) = ('\u{1}', '\u{2}');
    if !input.contains(COMMON.0) && !input.contains(COMMON.1) {
        return COMMON;
    }

    let used: HashSet<_> = input.chars().collect();
    let mut available = (1..=char::MAX as u32)
        .filter_map(char::from_u32)
        .filter(|character| !matches!(character, '"' | '\\' | '[' | ']'))
        .filter(|character| !used.contains(character));
    (
        available.next().expect("Unicode contains an unused marker"),
        available
            .next()
            .expect("Unicode contains two unused markers"),
    )
}

fn restore_string(mut text: String, markers: Option<(char, char)>) -> String {
    let Some(markers) = markers else {
        return text;
    };
    if text.contains(markers.0) {
        text = text.replace(markers.0, "[");
    }
    if text.contains(markers.1) {
        text = text.replace(markers.1, "]");
    }
    text
}

#[cfg(test)]
#[path = "expression/tests.rs"]
mod tests;
