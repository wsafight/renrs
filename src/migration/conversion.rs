use super::assets::{AssetCatalog, convert_image_statement, static_target};
use super::expressions::{
    closing_quote, convert_assignment, convert_condition, convert_default, convert_dialogue,
    escape_string, named_quoted_argument, quoted_argument, valid_identifier,
};
use super::{MigrationIssue, MigrationIssueKind, issue};

#[derive(Debug)]
pub(super) struct ConvertedScript {
    pub(super) output: String,
    pub(super) issues: Vec<MigrationIssue>,
}

#[derive(Debug)]
pub(super) struct OpenLabel {
    name: String,
    line: usize,
    indent: usize,
    last_direct_statement: Option<String>,
}

pub(super) fn convert_script(source: &str, file: &str, catalog: &AssetCatalog) -> ConvertedScript {
    let mut output = String::new();
    let mut issues = Vec::new();
    let mut skipped_block_indent = None;
    let mut open_label = None;
    for (index, raw) in source.lines().enumerate() {
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
        match convert_line(content, catalog) {
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
        }
        if let Some(name) = static_label_name(content) {
            open_label = Some(OpenLabel {
                name: name.to_owned(),
                line,
                indent,
                last_direct_statement: None,
            });
        } else if let Some(label) = &mut open_label
            && indent == label.indent + 4
        {
            label.last_direct_statement = Some(content.to_owned());
        }
    }
    report_label_fallthrough(file, &mut issues, open_label);
    ConvertedScript { output, issues }
}

fn write_line(output: &mut String, indentation: &str, value: &str) {
    output.push_str(indentation);
    output.push_str(value);
    output.push('\n');
}

#[derive(Debug)]
pub(super) enum LineConversion {
    One(String),
    Assumed { value: String, message: String },
    Unsupported { message: String, block: bool },
}

#[allow(clippy::too_many_lines)]
fn convert_line(content: &str, catalog: &AssetCatalog) -> LineConversion {
    if is_unsupported_block(content) {
        return unsupported(
            "Python, screen language, ATL, and transform blocks require manual migration",
            content.ends_with(':'),
        );
    }
    if let Some(converted) = convert_definition(content) {
        return converted;
    }
    if content.starts_with("label ") {
        return if static_label_name(content).is_some() {
            LineConversion::One(content.to_owned())
        } else {
            unsupported("label parameters and expressions are not supported", false)
        };
    }
    if matches!(content, "menu:" | "else:" | "return" | "stop music") {
        return LineConversion::One(content.to_owned());
    }
    if content.starts_with("return ") {
        return unsupported("return values are not supported", false);
    }
    if content.starts_with('"') && content.ends_with(':') {
        return convert_menu_option(content);
    }
    if let Some(dialogue) = convert_dialogue(content) {
        return dialogue;
    }
    if let Some(rest) = content.strip_prefix("scene ") {
        return convert_image_statement("scene", rest, catalog);
    }
    if let Some(rest) = content.strip_prefix("show ") {
        return convert_image_statement("show", rest, catalog);
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
            _ => unsupported("custom transitions require manual migration", false),
        };
    }
    if let Some(rest) = content.strip_prefix("jump ") {
        return static_target("jump", rest);
    }
    if let Some(rest) = content.strip_prefix("call ") {
        return static_target("call", rest);
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
    if content.starts_with("play music ")
        || content.starts_with("play sound ")
        || content.starts_with("pause ")
    {
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

fn convert_definition(content: &str) -> Option<LineConversion> {
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
    Some(LineConversion::One(format!(
        "define {name} = character \"{}\" color \"{}\"",
        escape_string(&display_name),
        escape_string(&color)
    )))
}

fn is_unsupported_block(content: &str) -> bool {
    content == "python:"
        || content.starts_with("init python")
        || content.starts_with("screen ")
        || content.starts_with("transform ")
        || content.starts_with("init ")
}

fn static_label_name(content: &str) -> Option<&str> {
    let name = content.strip_prefix("label ")?.strip_suffix(':')?.trim();
    valid_identifier(name).then_some(name)
}

pub(super) fn report_label_fallthrough(
    file: &str,
    issues: &mut Vec<MigrationIssue>,
    label: Option<OpenLabel>,
) {
    let Some(label) = label else {
        return;
    };
    let has_explicit_exit = label
        .last_direct_statement
        .as_deref()
        .is_some_and(|statement| {
            statement == "return"
                || statement.starts_with("return ")
                || statement.starts_with("jump ")
        });
    if !has_explicit_exit {
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
mod tests {
    use super::*;

    #[test]
    fn converts_interpolation_and_reports_fallthrough() {
        let converted = convert_script(
            "label start:\n    $ ready = True\n    \"Hello [ready]\"\nlabel second:\n    return\n",
            "script.rpy",
            &AssetCatalog::empty(),
        );
        assert!(converted.output.contains("set ready = true"));
        assert!(converted.output.contains("\"Hello {ready}\""));
        assert!(
            converted
                .issues
                .iter()
                .any(|item| item.message.contains("fallthrough"))
        );
    }
}
