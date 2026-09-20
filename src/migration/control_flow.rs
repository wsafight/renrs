use super::conversion::{LineConversion, unsupported, unsupported_with_code};
use super::expressions::{convert_expression, valid_identifier};
use super::parameters::{split_top_level, static_invocation, top_level_assignment};

pub(super) fn convert_label(source: &str) -> LineConversion {
    let Some(header) = source.strip_suffix(':').map(str::trim) else {
        return unsupported("label declaration must end with `:`", false);
    };
    let (name, arguments) = match static_invocation(header) {
        Ok(value) => value,
        Err(message) => return unsupported(message, false),
    };
    let Some(arguments) = arguments else {
        return LineConversion::One(format!("label {name}:"));
    };
    let parts = match split_top_level(arguments) {
        Ok(parts) => parts,
        Err(message) => return unsupported(message, false),
    };
    let mut converted = Vec::with_capacity(parts.len());
    let mut names = Vec::with_capacity(parts.len());
    let mut saw_default = false;
    for part in parts {
        let (parameter, default) = top_level_assignment(part)
            .map_or((part.trim(), None), |value| {
                (value.0.trim(), Some(value.1.trim()))
            });
        if !valid_identifier(parameter) {
            return unsupported(
                "label parameters must be named, fixed RenRS parameters",
                false,
            );
        }
        if names.contains(&parameter) {
            return unsupported("label contains a duplicate parameter", false);
        }
        names.push(parameter);
        if let Some(default) = default {
            saw_default = true;
            let default = match convert_expression(default) {
                Ok(value) => value,
                Err(message) => return unsupported(&message, false),
            };
            converted.push(format!("{parameter}={default}"));
        } else {
            if saw_default {
                return unsupported(
                    "required label parameters must precede parameters with defaults",
                    false,
                );
            }
            converted.push(parameter.to_owned());
        }
    }
    if name == "start" && !converted.is_empty() {
        return unsupported("the RenRS `start` label cannot declare parameters", false);
    }
    LineConversion::One(format!("label {name}({}):", converted.join(", ")))
}

pub(super) fn convert_call(source: &str) -> LineConversion {
    if source.trim().starts_with("expression ") {
        return unsupported_with_code(
            format!(
                "call target `{}` is dynamic or malformed; Ren'Py expression targets require manual migration",
                source.trim()
            ),
            false,
            "call_target_dynamic",
        );
    }
    let (name, arguments) = match static_invocation(source.trim()) {
        Ok(value) => value,
        Err(message) => return unsupported_call(source.trim(), message),
    };
    let Some(arguments) = arguments else {
        return LineConversion::One(format!("call {name}"));
    };
    let parts = match split_top_level(arguments) {
        Ok(parts) => parts,
        Err(message) => return unsupported(message, false),
    };
    let mut converted = Vec::with_capacity(parts.len());
    let mut names = Vec::new();
    let mut saw_named = false;
    for part in parts {
        if let Some((argument, value)) = top_level_assignment(part) {
            let argument = argument.trim();
            if !valid_identifier(argument) {
                return unsupported("call keyword argument must be a static name", false);
            }
            if names.contains(&argument) {
                return unsupported("call contains a duplicate named argument", false);
            }
            names.push(argument);
            saw_named = true;
            let value = match convert_expression(value.trim()) {
                Ok(value) => value,
                Err(message) => return unsupported(&message, false),
            };
            converted.push(format!("{argument}={value}"));
        } else {
            if saw_named {
                return unsupported(
                    "positional call arguments must precede named arguments",
                    false,
                );
            }
            let value = match convert_expression(part.trim()) {
                Ok(value) => value,
                Err(message) => return unsupported(&message, false),
            };
            converted.push(value);
        }
    }
    LineConversion::One(format!("call {name}({})", converted.join(", ")))
}

fn unsupported_call(source: &str, reason: &str) -> LineConversion {
    let name_end = source
        .as_bytes()
        .iter()
        .position(|byte| !(*byte == b'_' || byte.is_ascii_alphanumeric()))
        .unwrap_or(source.len());
    let name = &source[..name_end];
    if valid_identifier(name) {
        unsupported_with_code(
            format!("call `{source}` has a dynamic or malformed invocation clause: {reason}"),
            false,
            "call_clause_unsupported",
        )
    } else {
        unsupported_with_code(
            format!("call target `{source}` is dynamic or malformed: {reason}"),
            false,
            "call_target_dynamic",
        )
    }
}
