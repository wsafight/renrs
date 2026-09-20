use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StaticComposite {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) layers: Vec<CompositeLayer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CompositeLayer {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) path: String,
}

#[derive(Debug)]
pub(super) struct CompositeExpression {
    pub(super) composite: StaticComposite,
    pub(super) modifiers: String,
}

#[derive(Debug)]
pub(super) struct CompositeExpressionError {
    pub(super) code: &'static str,
    pub(super) message: String,
}

pub(super) fn parse_composite_expression(
    source: &str,
) -> Result<CompositeExpression, CompositeExpressionError> {
    let (arguments, suffix) =
        split_call(source).ok_or_else(|| error("unterminated Composite constructor"))?;
    let arguments = split_arguments(arguments);
    if arguments.len() < 3 || !(arguments.len() - 1).is_multiple_of(2) {
        return Err(error(
            "Composite expressions require a size and coordinate/displayable pairs",
        ));
    }
    let (width, height) = parse_pair(arguments[0], "canvas size")?;
    let width = u32::try_from(width).expect("positive Composite width fits u32");
    let height = u32::try_from(height).expect("positive Composite height fits u32");
    let mut layers = Vec::with_capacity((arguments.len() - 1) / 2);
    for pair in arguments[1..].chunks_exact(2) {
        let (x, y) = parse_pair(pair[0], "layer position")?;
        let path = parse_quoted_path(pair[1])?;
        layers.push(CompositeLayer { x, y, path });
    }
    if layers.len() > 32 {
        return Err(error(
            "Composite expressions support at most 32 static layers",
        ));
    }
    let modifiers = suffix.trim();
    if let Some(first) = modifiers.split_whitespace().next()
        && !matches!(
            first,
            "as" | "at" | "with" | "onlayer" | "zorder" | "behind"
        )
    {
        return Err(error(
            "Composite expression suffix must be a static display modifier",
        ));
    }
    Ok(CompositeExpression {
        composite: StaticComposite {
            width,
            height,
            layers,
        },
        modifiers: modifiers.to_owned(),
    })
}

impl StaticComposite {
    pub(super) fn generated_path(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.width.to_le_bytes());
        hasher.update(self.height.to_le_bytes());
        for layer in &self.layers {
            hasher.update(layer.x.to_le_bytes());
            hasher.update(layer.y.to_le_bytes());
            hasher.update(layer.path.as_bytes());
            hasher.update([0]);
        }
        format!(
            "images/__renrs_composite_{:x}.layers.json",
            hasher.finalize()
        )
    }

    pub(super) fn json_bytes(&self, paths: &[String]) -> Vec<u8> {
        let layers = self
            .layers
            .iter()
            .zip(paths)
            .map(|(layer, path)| {
                serde_json::json!({
                    "path": path,
                    "x": layer.x,
                    "y": layer.y,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&serde_json::json!({
            "width": self.width,
            "height": self.height,
            "layers": layers,
        }))
        .expect("static Composite JSON must serialize")
    }
}

fn parse_pair(source: &str, name: &str) -> Result<(i32, i32), CompositeExpressionError> {
    let value = source.trim();
    let Some(value) = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Err(error(&format!(
            "Composite {name} must be a pair of integers"
        )));
    };
    let values = value.split(',').map(str::trim).collect::<Vec<_>>();
    if values.len() != 2 {
        return Err(error(&format!(
            "Composite {name} must be a pair of integers"
        )));
    }
    let parse = |value: &str| {
        value
            .parse::<i32>()
            .ok()
            .filter(|value| (-8192..=8192).contains(value))
    };
    let Some(x) = parse(values[0]) else {
        return Err(error(&format!(
            "Composite {name} must use bounded integers"
        )));
    };
    let Some(y) = parse(values[1]) else {
        return Err(error(&format!(
            "Composite {name} must use bounded integers"
        )));
    };
    if name == "canvas size" && (x <= 0 || y <= 0) {
        return Err(error("Composite canvas size must be positive"));
    }
    Ok((x, y))
}

fn parse_quoted_path(source: &str) -> Result<String, CompositeExpressionError> {
    let source = source.trim();
    let Some(quote) = source
        .chars()
        .next()
        .filter(|quote| matches!(quote, '\'' | '"'))
    else {
        return Err(error(
            "Composite displayables must use quoted static image paths",
        ));
    };
    let mut escaped = false;
    let end = source[quote.len_utf8()..]
        .char_indices()
        .find_map(|(offset, character)| {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote {
                return Some(quote.len_utf8() + offset);
            }
            None
        })
        .ok_or_else(|| error("Composite image path is unterminated"))?;
    if !source[end + quote.len_utf8()..].trim().is_empty() {
        return Err(error("Composite displayables cannot have nested modifiers"));
    }
    let value = &source[quote.len_utf8()..end];
    let path = if quote == '"' {
        value.replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        value.replace("\\'", "'").replace("\\\\", "\\")
    };
    if path.is_empty() {
        return Err(error("Composite image paths cannot be empty"));
    }
    Ok(path)
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
                start = index + 1;
            }
            _ => {}
        }
    }
    arguments.push(source[start..].trim());
    arguments
}

fn error(message: &str) -> CompositeExpressionError {
    CompositeExpressionError {
        code: "image_expression_composite_unsupported",
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_static_composite_layers() {
        let expression = parse_composite_expression(
            "(640, 360), (0, 0), \"images/bg.png\", (40, 20), 'images/hero.png') as hero",
        )
        .expect("static Composite should parse");
        assert_eq!(expression.composite.width, 640);
        assert_eq!(expression.composite.layers[1].x, 40);
        assert_eq!(expression.modifiers, "as hero");
        assert!(
            expression
                .composite
                .generated_path()
                .ends_with(".layers.json")
        );
    }

    #[test]
    fn rejects_dynamic_composite_children_and_sizes() {
        assert!(parse_composite_expression("(640, 360), (0, 0), image_path)").is_err());
        assert!(parse_composite_expression("(0, 360), (0, 0), \"images/bg.png\")").is_err());
    }
}
