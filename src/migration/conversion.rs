use super::assets::{
    AssetCatalog, GeneratedAsset, convert_image_declaration, convert_image_statement, static_target,
};
use super::atl::TransformCatalog;
use super::audio::convert_audio;
use super::expressions::{
    closing_quote, convert_assignment, convert_condition, convert_default, convert_dialogue,
    convert_expression, valid_identifier,
};
use super::menus::{menu_has_explicit_exit, menu_prompt, statement_has_explicit_exit};
use super::parameters::{split_top_level, static_invocation, top_level_assignment};
use super::{MigrationIssue, MigrationIssueKind, issue};

#[derive(Debug)]
pub(super) struct ConvertedScript {
    pub(super) output: String,
    pub(super) issues: Vec<MigrationIssue>,
    pub(super) generated_assets: Vec<GeneratedAsset>,
}

#[derive(Debug)]
pub(super) struct OpenLabel {
    name: String,
    line: usize,
    indent: usize,
    has_explicit_exit: bool,
}

pub(super) fn convert_script(source: &str, file: &str, catalog: &AssetCatalog) -> ConvertedScript {
    let mut output = String::new();
    let mut issues = Vec::new();
    let mut generated_assets = Vec::new();
    let mut skipped_block_indent = None;
    let mut skipped_menu_prompt = None;
    let mut open_label = None;
    let lines = source.lines().collect::<Vec<_>>();
    let transforms = TransformCatalog::parse(source);
    for (index, raw) in lines.iter().copied().enumerate() {
        if skipped_menu_prompt == Some(index) {
            skipped_menu_prompt = None;
            continue;
        }
        let raw = if index == 0 {
            raw.strip_prefix('\u{feff}').unwrap_or(raw)
        } else {
            raw
        };
        let line = index + 1;
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        let content = raw.trim();
        if let Some(skipped_indent) = skipped_block_indent {
            if content.is_empty() || indent > skipped_indent {
                continue;
            }
            skipped_block_indent = None;
        }
        if content.is_empty() || content.starts_with('#') {
            output.push_str(raw.trim_end());
            output.push('\n');
            continue;
        }
        if open_label
            .as_ref()
            .is_some_and(|label: &OpenLabel| indent <= label.indent)
        {
            report_label_fallthrough(file, &mut issues, open_label.take());
        }
        let indentation = " ".repeat(indent);
        if transforms.recognized_declaration(line) {
            write_line(
                &mut output,
                &indentation,
                "# Ren'Py static transform is inlined at supported use sites.",
            );
            skipped_block_indent = Some(indent);
            continue;
        }
        let conversion = menu_prompt(&lines, index, indent).map_or_else(
            || convert_line(content, catalog, &transforms),
            |(prompt_line, prompt)| {
                skipped_menu_prompt = Some(prompt_line);
                LineConversion::One(format!("menu {prompt}:"))
            },
        );
        match conversion {
            LineConversion::One(value) => write_line(&mut output, &indentation, &value),
            LineConversion::Assumed { value, message } => {
                write_line(&mut output, &indentation, &value);
                issues.push(issue(file, line, MigrationIssueKind::Assumption, message));
            }
            LineConversion::Unsupported { message, block } => {
                write_line(
                    &mut output,
                    &indentation,
                    &format!("# TODO migration: {content}"),
                );
                issues.push(issue(file, line, MigrationIssueKind::Unsupported, message));
                if block {
                    skipped_block_indent = Some(indent);
                }
            }
            LineConversion::Generated { value, asset } => {
                write_line(&mut output, &indentation, &value);
                generated_assets.push(asset);
            }
        }
        if let Some(name) = static_label_name(content) {
            open_label = Some(OpenLabel {
                name: name.to_owned(),
                line,
                indent,
                has_explicit_exit: false,
            });
        } else if let Some(label) = &mut open_label
            && indent == label.indent + 4
        {
            label.has_explicit_exit = if content == "menu:" {
                menu_has_explicit_exit(&lines, index, indent)
            } else {
                statement_has_explicit_exit(content)
            };
        }
    }
    report_label_fallthrough(file, &mut issues, open_label);
    ConvertedScript {
        output,
        issues,
        generated_assets,
    }
}

fn write_line(output: &mut String, indentation: &str, value: &str) {
    for line in value.lines() {
        output.push_str(indentation);
        output.push_str(line);
        output.push('\n');
    }
}

#[derive(Debug)]
pub(super) enum LineConversion {
    One(String),
    Assumed {
        value: String,
        message: String,
    },
    Unsupported {
        message: String,
        block: bool,
    },
    Generated {
        value: String,
        asset: GeneratedAsset,
    },
}

#[allow(clippy::too_many_lines)]
fn convert_line(
    content: &str,
    catalog: &AssetCatalog,
    transforms: &TransformCatalog,
) -> LineConversion {
    if is_unsupported_block(content) {
        return unsupported(
            "Python, screen language, ATL, and transform blocks require manual migration",
            content.ends_with(':'),
        );
    }
    if let Some(converted) = super::story::convert_definition(content) {
        return converted;
    }
    if content.starts_with("image ") {
        return convert_image_declaration(content, catalog);
    }
    if let Some(rest) = content.strip_prefix("label ") {
        return convert_label(rest);
    }
    if matches!(
        content,
        "menu:"
            | "else:"
            | "return"
            | "stop music"
            | "stop sound"
            | "stop voice"
            | "window show"
            | "window hide"
    ) {
        return LineConversion::One(content.to_owned());
    }
    if let Some(expression) = content.strip_prefix("return ") {
        return match convert_expression(expression.trim()) {
            Ok(expression) => LineConversion::One(format!("return {expression}")),
            Err(message) => unsupported(&message, false),
        };
    }
    if content.starts_with('"') && content.ends_with(':') {
        return convert_menu_option(content);
    }
    if let Some(dialogue) = convert_dialogue(content) {
        return dialogue;
    }
    if let Some(converted) = super::story::convert_story(content) {
        return converted;
    }
    if let Some(rest) = content.strip_prefix("scene ") {
        return convert_image_statement("scene", rest, catalog, transforms);
    }
    if let Some(rest) = content.strip_prefix("show ") {
        return convert_image_statement("show", rest, catalog, transforms);
    }
    if let Some(rest) = content.strip_prefix("camera ") {
        return convert_camera(rest, transforms);
    }
    if let Some(rest) = content.strip_prefix("hide ") {
        return if valid_identifier(rest.trim()) {
            LineConversion::One(format!("hide {}", rest.trim()))
        } else {
            unsupported("complex hide statements require manual migration", false)
        };
    }
    if let Some(rest) = content.strip_prefix("with ") {
        return match rest.trim() {
            "fade" | "dissolve" => LineConversion::Assumed {
                value: "transition fade 0.5".to_owned(),
                message: format!("mapped Ren'Py `{}` to a 0.5 second fade", rest.trim()),
            },
            "pushleft" => LineConversion::One("transition push left 0.5".to_owned()),
            "pushright" => LineConversion::One("transition push right 0.5".to_owned()),
            "wipeleft" => LineConversion::One("transition wipe left 0.5".to_owned()),
            "wiperight" => LineConversion::One("transition wipe right 0.5".to_owned()),
            "hpunch" => LineConversion::One("transition punch h 0.25".to_owned()),
            "vpunch" => LineConversion::One("transition punch v 0.25".to_owned()),
            _ => unsupported("custom transitions require manual migration", false),
        };
    }
    if let Some(rest) = content.strip_prefix("jump ") {
        return static_target("jump", rest);
    }
    if let Some(rest) = content.strip_prefix("call ") {
        return convert_call(rest);
    }
    if let Some(rest) = content.strip_prefix("default ") {
        return convert_default(rest);
    }
    if let Some(rest) = content.strip_prefix("$ ") {
        return convert_assignment(rest);
    }
    if let Some(rest) = content.strip_prefix("if ") {
        return convert_condition("if", rest);
    }
    if let Some(rest) = content.strip_prefix("elif ") {
        return convert_condition("elif", rest);
    }
    if let Some(converted) = convert_audio(content) {
        return converted;
    }
    if content.starts_with("pause ") {
        return LineConversion::One(content.to_owned());
    }
    if content == "pass" {
        return LineConversion::Assumed {
            value: "# Ren'Py pass".to_owned(),
            message: "removed a no-op `pass` statement".to_owned(),
        };
    }
    unsupported(
        "statement is outside the supported Ren'Py migration subset",
        content.ends_with(':'),
    )
}

fn convert_camera(source: &str, transforms: &TransformCatalog) -> LineConversion {
    let tokens = source.split_whitespace().collect::<Vec<_>>();
    let transform_name = match tokens.as_slice() {
        ["at", name] | ["master", "at", name] => *name,
        [layer, "at", _] => {
            return unsupported(
                &format!(
                    "layer camera `{layer}` requires manual migration; only master is supported"
                ),
                false,
            );
        }
        _ => return unsupported("camera requires a static `at <transform>` clause", false),
    };
    let Some(transform) = transforms.get(transform_name) else {
        return unsupported(
            "camera references an unsupported static ATL transform",
            false,
        );
    };
    let Some(statements) = transform.camera_statements() else {
        return unsupported(
            "camera transforms using xalign/yalign require manual migration",
            false,
        );
    };
    if statements.is_empty() {
        return unsupported(
            "camera transform does not contain supported properties",
            false,
        );
    }
    let value = statements.join("\n");
    transform
        .assumption()
        .map_or(LineConversion::One(value.clone()), |message| {
            LineConversion::Assumed { value, message }
        })
}

fn convert_label(source: &str) -> LineConversion {
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

fn convert_call(source: &str) -> LineConversion {
    let (name, arguments) = match static_invocation(source.trim()) {
        Ok(value) => value,
        Err(message) => return unsupported(message, false),
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

fn convert_menu_option(content: &str) -> LineConversion {
    let Some(quote) = closing_quote(content, 0) else {
        return unsupported("menu choice has an unterminated string", true);
    };
    if content[quote + 1..].trim() != ":" {
        return unsupported(
            "conditional, argument-bearing, or dynamic menu choices require manual migration",
            true,
        );
    }
    if content[1..quote].contains('[') {
        return unsupported("menu choice interpolation is not supported by RenRS", true);
    }
    LineConversion::One(content.to_owned())
}

fn is_unsupported_block(content: &str) -> bool {
    content == "python:"
        || content.starts_with("init python")
        || content.starts_with("screen ")
        || content.starts_with("init ")
}

fn static_label_name(content: &str) -> Option<&str> {
    let header = content.strip_prefix("label ")?.strip_suffix(':')?.trim();
    static_invocation(header).ok().map(|(name, _)| name)
}

pub(super) fn report_label_fallthrough(
    file: &str,
    issues: &mut Vec<MigrationIssue>,
    label: Option<OpenLabel>,
) {
    let Some(label) = label else {
        return;
    };
    if !label.has_explicit_exit {
        issues.push(issue(
            file,
            label.line,
            MigrationIssueKind::Unsupported,
            format!(
                "label `{}` may rely on Ren'Py fallthrough, while RenRS implicitly returns at the end of every label; add an explicit `jump` or `return` after reviewing the intended flow",
                label.name
            ),
        ));
    }
}

pub(super) fn unsupported(message: &str, block: bool) -> LineConversion {
    LineConversion::Unsupported {
        message: message.to_owned(),
        block,
    }
}

#[cfg(test)]
#[path = "conversion_tests.rs"]
mod tests;
