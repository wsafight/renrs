use super::assets::{
    AssetCatalog, TransitionConversion, convert_image_declaration, convert_image_statement,
    convert_transition, static_target,
};
use super::atl::TransformCatalog;
use super::atl_parameters::normalize_and_specialize;
use super::audio::convert_audio;
use super::control_flow::{convert_call, convert_label};
use super::expressions::{
    closing_quote, convert_assignment, convert_condition, convert_default, convert_dialogue,
    convert_expression, valid_identifier,
};
use super::generated_assets::GeneratedAsset;
use super::menus::{menu_has_explicit_exit, menu_prompt, statement_has_explicit_exit};
use super::parameters::static_invocation;
use super::{MigrationIssue, MigrationIssueKind, coded_issue, issue};

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

#[allow(clippy::too_many_lines)]
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
            if let Some(message) = transforms.declaration_issue(line) {
                write_line(
                    &mut output,
                    &indentation,
                    &format!("# TODO migration: {content}"),
                );
                issues.push(issue(
                    file,
                    line,
                    MigrationIssueKind::Unsupported,
                    message.to_owned(),
                ));
            } else {
                write_line(
                    &mut output,
                    &indentation,
                    "# Ren'Py static transform is inlined at supported use sites.",
                );
            }
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
            LineConversion::Unsupported {
                message,
                block,
                code,
            } => {
                write_line(
                    &mut output,
                    &indentation,
                    &format!("# TODO migration: {content}"),
                );
                issues.push(code.map_or_else(
                    || issue(file, line, MigrationIssueKind::Unsupported, message.clone()),
                    |code| {
                        coded_issue(
                            file,
                            line,
                            MigrationIssueKind::Unsupported,
                            code,
                            message.clone(),
                        )
                    },
                ));
                if block {
                    skipped_block_indent = Some(indent);
                }
            }
            LineConversion::Generated {
                value,
                asset,
                assumption,
            } => {
                write_line(&mut output, &indentation, &value);
                if let Some(message) = assumption {
                    issues.push(issue(file, line, MigrationIssueKind::Assumption, message));
                }
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
        code: Option<&'static str>,
    },
    Generated {
        value: String,
        asset: GeneratedAsset,
        assumption: Option<String>,
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
        return match convert_transition(rest.trim()) {
            Ok(TransitionConversion { value, assumption }) => match assumption {
                Some(message) => LineConversion::Assumed { value, message },
                None => LineConversion::One(value),
            },
            Err(message) => unsupported(&message, false),
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
    unsupported_statement(content)
}

fn unsupported_statement(content: &str) -> LineConversion {
    let Some(name) = custom_statement_name(content) else {
        return unsupported(
            "statement is outside the supported Ren'Py migration subset",
            content.ends_with(':'),
        );
    };
    unsupported_with_code(
        format!(
            "custom Ren'Py statement `{name}` is outside the supported migration subset; review source `{content}` manually"
        ),
        content.ends_with(':'),
        "custom_statement_unsupported",
    )
}

fn custom_statement_name(content: &str) -> Option<&str> {
    let name = content.split_whitespace().next()?.trim_end_matches(':');
    if !valid_identifier(name) || matches!(name, "testsuite" | "testcase") {
        return None;
    }
    Some(name)
}

fn convert_camera(source: &str, transforms: &TransformCatalog) -> LineConversion {
    let (source, parameterized_name, specialized_transform) =
        match normalize_and_specialize(source, |call| transforms.specialize(call)) {
            Ok(value) => value,
            Err(message) => {
                return unsupported_with_code(message, false, "atl_parameters_unsupported");
            }
        };
    if parameterized_name.is_some() && specialized_transform.is_none() {
        return unsupported_with_code(
            "ATL transform call is not a known parameterized transform",
            false,
            "atl_parameters_unsupported",
        );
    }
    let tokens = source.split_whitespace().collect::<Vec<_>>();
    let (alias, transform_name) = match tokens.as_slice() {
        ["at", name] | ["master", "at", name] => ("camera".to_owned(), *name),
        [layer, "at", name] if matches!(*layer, "transient" | "screens" | "overlay") => {
            (format!("camera onlayer {layer}"), *name)
        }
        [layer, "at", _] => {
            return unsupported(
                &format!("layer camera `{layer}` is not a standard static display layer"),
                false,
            );
        }
        _ => return unsupported("camera requires a static `at <transform>` clause", false),
    };
    let transform = transforms.get(transform_name).or_else(|| {
        specialized_transform
            .as_ref()
            .filter(|_| parameterized_name.as_deref() == Some(transform_name))
    });
    let Some(transform) = transform else {
        if let Some(reason) = transforms.unsupported_reason(transform_name) {
            return unsupported(reason, false);
        }
        return unsupported(
            "camera references an unsupported static ATL transform",
            false,
        );
    };
    let Some(statements) = transform.camera_statements(&alias) else {
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
        code: None,
    }
}

pub(super) fn unsupported_with_code(
    message: impl Into<String>,
    block: bool,
    code: &'static str,
) -> LineConversion {
    LineConversion::Unsupported {
        message: message.into(),
        block,
        code: Some(code),
    }
}

#[cfg(test)]
#[path = "conversion_tests.rs"]
mod tests;
