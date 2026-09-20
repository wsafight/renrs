use super::composite_expression::{StaticComposite, parse_composite_expression};
use super::expressions::valid_identifier;
use super::parameters::top_level_assignment;

#[derive(Debug)]
pub(super) struct StaticImageExpression {
    pub(super) path: String,
    pub(super) modifiers: String,
    pub(super) transform: Option<StaticExpressionTransform>,
    pub(super) composite: Option<StaticComposite>,
}

impl StaticImageExpression {
    pub(super) fn position(&self) -> &'static str {
        self.transform
            .as_ref()
            .and_then(|transform| transform.position)
            .unwrap_or("center")
    }

    pub(super) fn has_transform(&self) -> bool {
        self.transform.is_some()
    }

    pub(super) fn transform_statements(&self, alias: &str) -> Vec<String> {
        self.transform.as_ref().map_or_else(Vec::new, |transform| {
            transform
                .properties
                .iter()
                .map(|(property, value)| format!("transform {alias} {property} {value}"))
                .collect()
        })
    }
}

#[derive(Debug)]
pub(super) struct StaticExpressionTransform {
    pub(super) position: Option<&'static str>,
    pub(super) properties: Vec<(&'static str, f32)>,
}

#[derive(Debug)]
pub(super) struct ImageExpressionError {
    pub(super) code: &'static str,
    pub(super) message: String,
}

pub(super) fn parse_static_image_expression(
    source: &str,
) -> Result<StaticImageExpression, ImageExpressionError> {
    let source = source.trim_start();
    if let Some(source) = source
        .strip_prefix("At(")
        .or_else(|| source.strip_prefix("im.At("))
    {
        return parse_at_expression(source);
    }
    if let Some(source) = source
        .strip_prefix("Transform(")
        .or_else(|| source.strip_prefix("im.Transform("))
    {
        return parse_transform_expression(source);
    }
    if let Some(source) = source
        .strip_prefix("Composite(")
        .or_else(|| source.strip_prefix("im.Composite("))
    {
        let expression =
            parse_composite_expression(source).map_err(|error| ImageExpressionError {
                code: error.code,
                message: error.message,
            })?;
        return Ok(StaticImageExpression {
            path: String::new(),
            modifiers: expression.modifiers,
            transform: None,
            composite: Some(expression.composite),
        });
    }
    let (source, constructor) = if let Some(source) = source.strip_prefix("Image(") {
        (source, true)
    } else if let Some(source) = source.strip_prefix("im.Image(") {
        (source, true)
    } else {
        (source, false)
    };
    if !constructor && !source.starts_with('"') && !source.starts_with('\'') {
        return Err(error(
            "image_expression_dynamic",
            "only single- or double-quoted static image expressions or Image(path) constructors can be migrated automatically",
        ));
    }
    let (path, remainder) = parse_quoted_path(source, constructor)?;
    let remainder = if constructor {
        let remainder = remainder.trim_start();
        let Some(remainder) = remainder.strip_prefix(')') else {
            return Err(error(
                "image_expression_constructor_unsupported",
                "Image constructors with additional arguments or dynamic properties require manual migration",
            ));
        };
        remainder
    } else {
        remainder
    };
    let modifiers = remainder.trim();
    if let Some(first) = modifiers.split_whitespace().next()
        && !matches!(
            first,
            "as" | "at" | "with" | "onlayer" | "zorder" | "behind"
        )
    {
        return Err(error(
            "image_expression_dynamic",
            "image expression suffix must be a static display modifier",
        ));
    }
    Ok(StaticImageExpression {
        path,
        modifiers: modifiers.to_owned(),
        transform: None,
        composite: None,
    })
}

fn parse_at_expression(source: &str) -> Result<StaticImageExpression, ImageExpressionError> {
    let (arguments, suffix) = split_call(source).ok_or_else(|| {
        error(
            "image_expression_malformed",
            "At image expression contains an unterminated constructor",
        )
    })?;
    let arguments = split_arguments(arguments);
    if arguments.len() != 2 {
        return Err(error(
            "image_expression_constructor_unsupported",
            "At image expressions require exactly one static image and one static transform",
        ));
    }
    let (path, remainder) = parse_quoted_path(arguments[0].trim(), false)?;
    if !remainder.trim().is_empty() {
        return Err(error(
            "image_expression_constructor_unsupported",
            "At image expressions do not support nested image modifiers",
        ));
    }
    let transform = arguments[1].trim();
    if !valid_identifier(transform) {
        return Err(error(
            "image_expression_dynamic",
            "At image expressions require a static transform identifier",
        ));
    }
    let suffix = suffix.trim();
    let modifiers = if suffix.is_empty() {
        format!("at {transform}")
    } else {
        format!("at {transform} {suffix}")
    };
    Ok(StaticImageExpression {
        path,
        modifiers,
        transform: None,
        composite: None,
    })
}

fn parse_transform_expression(source: &str) -> Result<StaticImageExpression, ImageExpressionError> {
    let (arguments, suffix) = split_call(source).ok_or_else(|| {
        error(
            "image_expression_malformed",
            "Transform image expression contains an unterminated constructor",
        )
    })?;
    let arguments = split_arguments(arguments);
    let Some(path_argument) = arguments.first().copied().filter(|value| !value.is_empty()) else {
        return Err(error(
            "image_expression_transform_unsupported",
            "Transform image expressions require one static image path",
        ));
    };
    let (path, remainder) = parse_quoted_path(path_argument, false)?;
    if !remainder.trim().is_empty() {
        return Err(error(
            "image_expression_transform_unsupported",
            "Transform image expressions require a single static image path",
        ));
    }
    let transform = parse_transform_options(&arguments[1..])?;
    let suffix = suffix.trim();
    if let Some(first) = suffix.split_whitespace().next()
        && !matches!(
            first,
            "as" | "at" | "with" | "onlayer" | "zorder" | "behind"
        )
    {
        return Err(error(
            "image_expression_dynamic",
            "image expression suffix must be a static display modifier",
        ));
    }
    Ok(StaticImageExpression {
        path,
        modifiers: suffix.to_owned(),
        transform: (!transform.properties.is_empty() || transform.position.is_some())
            .then_some(transform),
        composite: None,
    })
}

fn parse_transform_options(
    arguments: &[&str],
) -> Result<StaticExpressionTransform, ImageExpressionError> {
    let mut transform = StaticExpressionTransform {
        position: None,
        properties: Vec::new(),
    };
    let mut alignment_mask = 0_u8;
    for argument in arguments {
        let Some((name, value)) = top_level_assignment(argument.trim()) else {
            return Err(error(
                "image_expression_transform_unsupported",
                "Transform image expression options must use static named numeric values",
            ));
        };
        if transform
            .properties
            .iter()
            .any(|(property, _)| *property == name.trim())
            || name.trim() == "xalign" && alignment_mask & 1 != 0
            || name.trim() == "yalign" && alignment_mask & 2 != 0
        {
            return Err(error(
                "image_expression_transform_unsupported",
                "Transform image expression options cannot repeat a property",
            ));
        }
        let name = name.trim();
        let value = value
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite());
        let Some(value) = value else {
            return Err(error(
                "image_expression_transform_unsupported",
                "Transform image expression options must be finite numeric literals",
            ));
        };
        match name {
            "xalign" => {
                alignment_mask |= 1;
                transform.position = Some(match value {
                    value if value.abs() < f32::EPSILON => "left",
                    value if (value - 0.5).abs() < f32::EPSILON => "center",
                    value if (value - 1.0).abs() < f32::EPSILON => "right",
                    _ => {
                        return Err(error(
                            "image_expression_transform_unsupported",
                            "Transform xalign must be 0, 0.5, or 1",
                        ));
                    }
                });
            }
            "yalign" if (value - 1.0).abs() < f32::EPSILON => alignment_mask |= 2,
            "alpha" if (0.0..=1.0).contains(&value) => {
                transform.properties.push(("alpha", value));
            }
            "zoom" if (0.01..=20.0).contains(&value) => {
                transform.properties.push(("scale", value));
            }
            "rotate" | "xoffset" | "yoffset" => transform.properties.push((
                match name {
                    "rotate" => "rotate",
                    "xoffset" => "x",
                    _ => "y",
                },
                value,
            )),
            _ => {
                return Err(error(
                    "image_expression_transform_unsupported",
                    "Transform image expression uses an unsupported option",
                ));
            }
        }
    }
    Ok(transform)
}

fn split_call(source: &str) -> Option<(&str, &str)> {
    let mut depth = 1;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if let Some(expected) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == expected {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&source[..index], &source[index + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_arguments(source: &str) -> Vec<&str> {
    let mut arguments = Vec::new();
    let mut start = 0;
    let mut depth: usize = 0;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if let Some(expected) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == expected {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                arguments.push(source[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    arguments.push(source[start..].trim());
    arguments
}
fn parse_quoted_path(
    source: &str,
    constructor: bool,
) -> Result<(String, &str), ImageExpressionError> {
    let source = source.trim_start();
    let Some(quote) = source.chars().next() else {
        return Err(error(
            if constructor {
                "image_expression_dynamic"
            } else {
                "image_expression_malformed"
            },
            "image expression must contain a quoted resource path",
        ));
    };
    if !matches!(quote, '\'' | '"') {
        return Err(error(
            "image_expression_dynamic",
            "image expression path must be a static quoted resource",
        ));
    }
    let Some(end) = quoted_end(source, 0, quote) else {
        return Err(error(
            "image_expression_malformed",
            "image expression contains an unterminated string",
        ));
    };
    let value = &source[quote.len_utf8()..end];
    let path = if quote == '"' {
        value.replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        value.replace("\\'", "'").replace("\\\\", "\\")
    };
    if path.is_empty() {
        return Err(error(
            "image_expression_malformed",
            "image expression resource path cannot be empty",
        ));
    }
    Ok((path, &source[end + 1..]))
}

fn error(code: &'static str, message: &str) -> ImageExpressionError {
    ImageExpressionError {
        code,
        message: message.to_owned(),
    }
}

fn quoted_end(source: &str, start: usize, quote: char) -> Option<usize> {
    let mut escaped = false;
    source[start + quote.len_utf8()..]
        .char_indices()
        .find_map(|(offset, character)| {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote {
                return Some(start + quote.len_utf8() + offset);
            }
            None
        })
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_quoted_resource_and_preserves_modifiers() {
        let expression = parse_static_image_expression("\"images/hero.png\" as hero at left")
            .expect("static expression should parse");
        assert_eq!(expression.path, "images/hero.png");
        assert_eq!(expression.modifiers, "as hero at left");
    }

    #[test]
    fn parses_single_quoted_resource_paths() {
        let expression = parse_static_image_expression("'images/hero.png' as hero")
            .expect("single-quoted expression should parse");
        assert_eq!(expression.path, "images/hero.png");
        assert_eq!(expression.modifiers, "as hero");
    }

    #[test]
    fn parses_static_image_constructors() {
        for source in [
            "Image(\"images/hero.png\") as hero at left",
            "im.Image('images/hero.png') as hero at right",
        ] {
            let expression = parse_static_image_expression(source)
                .expect("static image constructor should parse");
            assert_eq!(expression.path, "images/hero.png");
            assert!(expression.modifiers.starts_with("as hero"));
        }
    }

    #[test]
    fn parses_bounded_static_transform_expressions() {
        let expression = parse_static_image_expression(
            "Transform(\"images/hero.png\", zoom=1.2, rotate=15, xalign=0.5, yalign=1) as hero",
        )
        .expect("static Transform should parse");
        assert_eq!(expression.path, "images/hero.png");
        assert_eq!(expression.modifiers, "as hero");
        assert_eq!(expression.position(), "center");
        assert_eq!(
            expression.transform_statements("hero"),
            ["transform hero scale 1.2", "transform hero rotate 15"]
        );
    }

    #[test]
    fn parses_static_at_wrappers_and_preserves_show_modifiers() {
        for source in [
            "At(\"images/hero.png\", focus) as hero",
            "im.At('images/hero.png', focus) as hero at left",
        ] {
            let expression =
                parse_static_image_expression(source).expect("static At wrapper should parse");
            assert_eq!(expression.path, "images/hero.png");
            assert!(expression.modifiers.starts_with("at focus"));
            assert!(expression.modifiers.contains("as hero"));
        }
    }

    #[test]
    fn rejects_dynamic_and_unterminated_expressions() {
        assert!(parse_static_image_expression("image_path as hero").is_err());
        assert!(parse_static_image_expression("\"images/hero.png as hero").is_err());
        assert!(parse_static_image_expression("Image(image_path) as hero").is_err());
        assert!(
            parse_static_image_expression("At(\"images/hero.png\", transform_name) as hero")
                .is_ok()
        );
        assert!(parse_static_image_expression("At(image_path, focus) as hero").is_err());
        assert!(
            parse_static_image_expression("At(\"images/hero.png\", focus, other) as hero").is_err()
        );
        assert!(
            parse_static_image_expression("Image(\"images/hero.png\", xalign=0.5) as hero")
                .is_err()
        );
        let error =
            parse_static_image_expression("Transform(\"images/hero.png\", zoom=scale) as hero")
                .expect_err("dynamic Transform options should be rejected");
        assert_eq!(error.code, "image_expression_transform_unsupported");
    }
}
