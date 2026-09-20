use super::expressions::valid_identifier;
use super::parameters::{split_top_level, static_invocation};

#[derive(Debug, Clone)]
pub(super) struct TransformHeader {
    pub(super) name: String,
    pub(super) parameters: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct ParameterizedTransform {
    parameters: Vec<String>,
    body: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TransformCall {
    pub(super) name: String,
    pub(super) arguments: Vec<f32>,
}

pub(super) fn parse_transform_header(source: &str) -> Option<Result<TransformHeader, String>> {
    let rest = source.strip_prefix("transform ")?.strip_suffix(':')?.trim();
    let Some(open) = rest.find('(') else {
        return Some(if valid_identifier(rest) {
            Ok(TransformHeader {
                name: rest.to_owned(),
                parameters: None,
            })
        } else {
            Err("named transform requires a static identifier".to_owned())
        });
    };
    if !rest.ends_with(')') || rest[..open].contains(|character: char| character.is_whitespace()) {
        return Some(Err(
            "parameterized ATL transform declaration is malformed".to_owned()
        ));
    }
    let name = &rest[..open];
    if !valid_identifier(name) {
        return Some(Err(
            "parameterized ATL transform requires a static identifier".to_owned(),
        ));
    }
    let argument_source = &rest[open + 1..rest.len() - 1];
    let parameters = match split_top_level(argument_source) {
        Ok(parts) => parts
            .into_iter()
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        Err(message) => return Some(Err(message.to_owned())),
    };
    if parameters.is_empty() {
        return Some(Ok(TransformHeader {
            name: name.to_owned(),
            parameters: None,
        }));
    }
    if parameters
        .iter()
        .any(|parameter| !valid_identifier(parameter) || is_reserved_parameter(parameter))
    {
        return Some(Err(
            "parameterized ATL transforms require unique positional identifier parameters"
                .to_owned(),
        ));
    }
    let mut unique = parameters.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != parameters.len() {
        return Some(Err(
            "parameterized ATL transforms cannot repeat a parameter name".to_owned(),
        ));
    }
    Some(Ok(TransformHeader {
        name: name.to_owned(),
        parameters: Some(parameters),
    }))
}

impl ParameterizedTransform {
    pub(super) fn new(parameters: Vec<String>, body: Vec<String>) -> Result<Self, String> {
        if body.is_empty() {
            return Err("parameterized ATL transform must contain a supported body".to_owned());
        }
        let neutral = template_body(&body, &parameters)?;
        if neutral.is_empty() {
            return Err("parameterized ATL transform must contain a supported body".to_owned());
        }
        Ok(Self { parameters, body })
    }

    pub(super) fn specialized_body(&self, arguments: &[f32]) -> Result<Vec<String>, String> {
        if arguments.len() != self.parameters.len() {
            return Err(format!(
                "ATL transform expects {} positional argument(s), got {}",
                self.parameters.len(),
                arguments.len()
            ));
        }
        if arguments.iter().any(|value| !value.is_finite()) {
            return Err("ATL transform arguments must be finite numbers".to_owned());
        }
        let substitutions = self
            .parameters
            .iter()
            .zip(arguments.iter())
            .map(|(name, value)| (name.as_str(), value.to_string()))
            .collect::<Vec<_>>();
        Ok(self
            .body
            .iter()
            .map(|line| {
                line.split_whitespace()
                    .map(|token| {
                        substitutions
                            .iter()
                            .find_map(|(name, value)| (*name == token).then_some(value.as_str()))
                            .unwrap_or(token)
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect())
    }
}

pub(super) fn parse_transform_call(source: &str) -> Result<TransformCall, String> {
    let source = source.trim();
    let (name, arguments) = static_invocation(source)
        .map_err(|_| "ATL transform call must use a static name and positional arguments")?;
    let Some(arguments) = arguments else {
        return Err("ATL transform call requires parentheses with positional arguments".to_owned());
    };
    let arguments = split_top_level(arguments)
        .map_err(str::to_owned)?
        .into_iter()
        .map(|value| {
            value
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| "ATL transform arguments must be finite numeric literals".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TransformCall {
        name: name.to_owned(),
        arguments,
    })
}

pub(super) fn normalize_at_transform_call(
    source: &str,
) -> Result<Option<(String, TransformCall)>, String> {
    let Some((start, end)) = find_at_call(source) else {
        return Ok(None);
    };
    let call_source = &source[start..end];
    let call = parse_transform_call(call_source)?;
    let mut normalized = source.to_owned();
    normalized.replace_range(start..end, &call.name);
    Ok(Some((normalized, call)))
}

pub(super) fn normalize_and_specialize<T>(
    source: &str,
    specialize: impl FnOnce(&TransformCall) -> Result<Option<T>, String>,
) -> Result<(String, Option<String>, Option<T>), String> {
    let Some((normalized, call)) = normalize_at_transform_call(source)? else {
        return Ok((source.to_owned(), None, None));
    };
    let name = call.name.clone();
    let transform = specialize(&call)?;
    Ok((normalized, Some(name), transform))
}

fn find_at_call(source: &str) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index + 1 < bytes.len() {
        let byte = bytes[index];
        if let Some(current) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == current {
                quote = None;
            }
            index += 1;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if &bytes[index..index + 2] == b"at"
            && (index == 0 || !is_identifier_byte(bytes[index - 1]))
            && (index + 2 == bytes.len() || !is_identifier_byte(bytes[index + 2]))
        {
            let mut name_start = index + 2;
            while bytes.get(name_start).is_some_and(u8::is_ascii_whitespace) {
                name_start += 1;
            }
            let mut name_end = name_start;
            while bytes
                .get(name_end)
                .is_some_and(|byte| is_identifier_byte(*byte))
            {
                name_end += 1;
            }
            if name_end > name_start && bytes.get(name_end) == Some(&b'(') {
                let mut depth = 0_usize;
                let mut call_quote = None;
                let mut call_escaped = false;
                for (offset, value) in source[name_end..].char_indices() {
                    if let Some(current) = call_quote {
                        if call_escaped {
                            call_escaped = false;
                        } else if value == '\\' {
                            call_escaped = true;
                        } else if value == current {
                            call_quote = None;
                        }
                        continue;
                    }
                    match value {
                        '\'' | '"' => call_quote = Some(value),
                        '(' => depth += 1,
                        ')' => {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                return Some((name_start, name_end + offset + 1));
                            }
                        }
                        _ => {}
                    }
                }
                return None;
            }
        }
        index += 1;
    }
    None
}

fn template_body(body: &[String], parameters: &[String]) -> Result<Vec<String>, String> {
    let mut neutral = Vec::with_capacity(body.len());
    for (index, line) in body.iter().enumerate() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.first() == Some(&"repeat") {
            return Err("parameterized ATL transforms cannot use repeat".to_owned());
        }
        if tokens.first() == Some(&"pause") {
            if tokens.len() != 2 || parse_number(tokens[1]).is_none() {
                return Err("parameterized ATL pause duration must be a finite number".to_owned());
            }
            neutral.push(tokens.join(" "));
            continue;
        }
        let offset = match tokens.first().copied() {
            Some("linear" | "easein" | "easeout" | "ease") => {
                if tokens.len() < 2 || parse_number(tokens[1]).is_none() {
                    return Err(
                        "parameterized ATL timing duration must be a finite number".to_owned()
                    );
                }
                2
            }
            _ => 0,
        };
        let mut replaced = tokens.clone();
        let mut cursor = offset;
        while cursor < tokens.len() {
            let property = tokens[cursor];
            let Some(value) = tokens.get(cursor + 1).copied() else {
                return Err("parameterized ATL transform property is missing a value".to_owned());
            };
            if !matches!(
                property,
                "xalign" | "yalign" | "alpha" | "zoom" | "rotate" | "xoffset" | "yoffset"
            ) {
                return Err("parameterized ATL transform uses an unsupported property".to_owned());
            }
            if parameters.iter().any(|parameter| parameter == value) {
                if matches!(property, "xalign" | "yalign") {
                    return Err("parameterized ATL alignment values must remain static".to_owned());
                }
                replaced[cursor + 1] = neutral_value(property);
            } else if parse_number(value).is_none() {
                return Err(format!(
                    "parameterized ATL property `{property}` must use a numeric literal or a direct parameter"
                ));
            }
            cursor += 2;
        }
        if cursor != tokens.len() {
            return Err("parameterized ATL transform property list is malformed".to_owned());
        }
        neutral.push(replaced.join(" "));
        if index + 1 == body.len() && tokens.first() == Some(&"repeat") {
            return Err("parameterized ATL transforms cannot use repeat".to_owned());
        }
    }
    Ok(neutral)
}

fn neutral_value(property: &str) -> &'static str {
    if property == "zoom" { "1" } else { "0" }
}

fn parse_number(value: &str) -> Option<f32> {
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn is_reserved_parameter(value: &str) -> bool {
    matches!(
        value,
        "linear"
            | "ease"
            | "easein"
            | "easeout"
            | "pause"
            | "repeat"
            | "xalign"
            | "yalign"
            | "alpha"
            | "zoom"
            | "rotate"
            | "xoffset"
            | "yoffset"
    )
}

fn is_identifier_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numeric_positional_calls_and_normalizes_spacing() {
        let (source, call) = normalize_at_transform_call("hero at move_by(20, -0.5) as h")
            .expect("call should parse")
            .expect("call should be found");
        assert_eq!(source, "hero at move_by as h");
        assert_eq!(call.name, "move_by");
        assert_eq!(call.arguments, [20.0, -0.5]);
    }

    #[test]
    fn rejects_dynamic_or_default_arguments() {
        assert!(parse_transform_call("move_by(distance)").is_err());
        assert!(parse_transform_call("move_by(x=20)").is_err());
        assert!(
            parse_transform_header("transform move_by(distance=1):")
                .expect("header should be recognized")
                .is_err()
        );
    }

    #[test]
    fn validates_direct_parameter_property_values() {
        let transform = ParameterizedTransform::new(
            vec!["distance".to_owned()],
            vec!["xoffset distance".to_owned()],
        )
        .expect("template should parse");
        assert_eq!(transform.specialized_body(&[20.0]).unwrap(), ["xoffset 20"]);
        assert!(
            ParameterizedTransform::new(
                vec!["distance".to_owned()],
                vec!["xoffset distance + 1".to_owned()],
            )
            .is_err()
        );
    }
}
