use std::fmt::Write;

use super::conversion::{LineConversion, unsupported};
use super::expressions::{escape_string, named_quoted_argument, quoted_argument, valid_identifier};

pub(super) fn convert_definition(content: &str) -> Option<LineConversion> {
    let rest = content.strip_prefix("define ")?;
    let (name, value) = rest.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    if name == "config.name" {
        let title = quoted_argument(value)?;
        return Some(LineConversion::One(format!(
            "config title \"{}\"",
            escape_string(&title)
        )));
    }
    if !valid_identifier(name) || !value.starts_with("Character(") || !value.ends_with(')') {
        return Some(unsupported(
            "only static Character declarations can be converted",
            false,
        ));
    }
    let arguments = &value["Character(".len()..value.len() - 1];
    let Some(display_name) = quoted_argument(arguments) else {
        return Some(unsupported(
            "translated or computed character names require manual migration",
            false,
        ));
    };
    let color = named_quoted_argument(arguments, "color").unwrap_or_else(|| "#f4f4f5".to_owned());
    let image = named_quoted_argument(arguments, "image");
    let mut result = format!(
        "define {name} = character \"{}\" color \"{}\"",
        escape_string(&display_name),
        escape_string(&color)
    );
    if let Some(image) = image {
        if !valid_identifier(&image) {
            return Some(unsupported(
                "character image must be a static identifier",
                false,
            ));
        }
        write!(result, " image {image}").expect("writing to a String cannot fail");
    }
    Some(LineConversion::One(result))
}

pub(super) fn convert_story(content: &str) -> Option<LineConversion> {
    if let Some(rest) = content.strip_prefix("call screen ") {
        return Some(static_screen("call screen", rest));
    }
    if let Some(rest) = content.strip_prefix("show screen ") {
        return Some(static_screen("show screen", rest));
    }
    if let Some(rest) = content.strip_prefix("hide screen ") {
        return Some(static_screen("hide screen", rest));
    }
    if let Some(rest) = content.strip_prefix("transform ") {
        return Some(convert_named_transform(rest));
    }
    None
}

fn static_screen(command: &str, rest: &str) -> LineConversion {
    if valid_identifier(rest.trim()) {
        LineConversion::One(format!("{command} {}", rest.trim()))
    } else {
        unsupported(&format!("{command} requires a static name"), false)
    }
}

fn convert_named_transform(rest: &str) -> LineConversion {
    let rest = rest.trim();
    let (name, body) = rest
        .strip_suffix(':')
        .map_or((rest, false), |name| (name.trim(), true));
    if !valid_identifier(name) || matches!(name, "left" | "center" | "right" | "camera") {
        return unsupported("named transform requires a static identifier", body);
    }
    LineConversion::One(if body {
        format!("transform {name}:")
    } else {
        format!("transform {name}")
    })
}
