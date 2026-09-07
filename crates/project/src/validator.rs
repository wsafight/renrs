use std::collections::HashSet;
use std::path::{Component, Path};

use crate::archive::ResourceArchive;
use crate::diagnostic::Diagnostic;
use crate::syntax::{Block, Expr, Script, StatementKind};

#[must_use]
pub fn validate(script: &Script, game_root: &Path) -> Vec<Diagnostic> {
    validate_with_resources(script, &|resource| game_root.join(resource).is_file())
}

#[must_use]
pub fn validate_archive(script: &Script, archive: &ResourceArchive) -> Vec<Diagnostic> {
    validate_with_resources(script, &|resource| archive.contains(resource))
}

fn validate_with_resources(
    script: &Script,
    resource_exists: &dyn Fn(&str) -> bool,
) -> Vec<Diagnostic> {
    let mut validator = Validator {
        script,
        resource_exists,
        diagnostics: Vec::new(),
        assigned_variables: HashSet::new(),
    };

    for character in script.characters.values() {
        if parse_hex_color(&character.color).is_none() {
            validator.diagnostics.push(Diagnostic::new(
                &character.span.source,
                character.span.line,
                character.span.column,
                format!(
                    "character color `{}` must use #RRGGBB or #RRGGBBAA",
                    character.color
                ),
            ));
        }
    }

    for definition in script.images.values() {
        validator.resource(&definition.path, &definition.span);
    }
    for (name, definition) in &script.display_layers {
        if crate::syntax::builtin_display_layer_order(name).is_some() {
            validator.push(
                &definition.span,
                format!("display layer `{name}` is built in and cannot be redeclared"),
            );
        }
    }
    validator
        .assigned_variables
        .extend(script.defaults.keys().cloned());
    for definition in script.defaults.values() {
        validator.expression(&definition.value, &definition.span);
    }
    for block in script.labels.values() {
        collect_assignments(block, &mut validator.assigned_variables);
    }
    for parameters in script.label_parameters.values() {
        for parameter in parameters {
            if let Some(default) = &parameter.default {
                validator.expression(default, &parameter.span);
            }
        }
    }
    let global_assignments = validator.assigned_variables.clone();
    for (label, block) in &script.labels {
        validator.assigned_variables.clone_from(&global_assignments);
        if let Some(parameters) = script.label_parameters.get(label) {
            validator
                .assigned_variables
                .extend(parameters.iter().map(|parameter| parameter.name.clone()));
        }
        validator.block(block);
    }
    validator.diagnostics
}

struct Validator<'a> {
    script: &'a Script,
    resource_exists: &'a dyn Fn(&str) -> bool,
    diagnostics: Vec<Diagnostic>,
    assigned_variables: HashSet<String>,
}

impl Validator<'_> {
    fn block(&mut self, block: &Block) {
        for statement in block {
            match &statement.kind {
                StatementKind::Timeline { block } => self.block(block),
                StatementKind::Parallel { tracks } => {
                    if let Err(message) = crate::syntax::validate_tracks(tracks) {
                        self.push(&statement.span, message);
                    }
                }
                StatementKind::Video { path, .. } => self.resource(path, &statement.span),
                StatementKind::Dialogue { speaker, .. } => {
                    if let Some(speaker) = speaker
                        && !self.script.characters.contains_key(speaker)
                    {
                        self.push(&statement.span, format!("unknown character `{speaker}`"));
                    }
                }
                StatementKind::Scene { path } | StatementKind::Show { path, .. } => {
                    if let Some(name) = path.strip_prefix("@image:") {
                        if !self.script.images.contains_key(name) {
                            self.push(&statement.span, format!("unknown image `{name}`"));
                        }
                    } else {
                        self.resource(path, &statement.span);
                    }
                    if let StatementKind::Show { display_layer, .. } = &statement.kind {
                        self.display_layer(display_layer, &statement.span);
                    }
                }
                StatementKind::PlayMusic { path, .. }
                | StatementKind::QueueMusic { path, .. }
                | StatementKind::PlaySound { path, .. }
                | StatementKind::PlayVoice { path } => {
                    self.resource(path, &statement.span);
                }
                StatementKind::Jump { label } => {
                    if !self.script.labels.contains_key(label) {
                        self.push(&statement.span, format!("unknown label `{label}`"));
                    }
                }
                StatementKind::Call { label, arguments } => {
                    if !self.script.labels.contains_key(label) {
                        self.push(&statement.span, format!("unknown label `{label}`"));
                    }
                    for argument in arguments {
                        self.expression(&argument.value, &statement.span);
                    }
                }
                StatementKind::Return { value } => {
                    if let Some(value) = value {
                        self.expression(value, &statement.span);
                    }
                }
                StatementKind::Set { value, .. }
                | StatementKind::Extension { input: value, .. } => {
                    self.expression(value, &statement.span);
                }
                StatementKind::If {
                    branches,
                    else_block,
                } => {
                    for (condition, block) in branches {
                        self.expression(condition, &statement.span);
                        self.block(block);
                    }
                    self.block(else_block);
                }
                StatementKind::Menu { prompt, options } => {
                    if let Some(speaker) =
                        prompt.as_ref().and_then(|prompt| prompt.speaker.as_ref())
                        && !self.script.characters.contains_key(speaker)
                    {
                        self.push(&statement.span, format!("unknown character `{speaker}`"));
                    }
                    for option in options {
                        if let Some(condition) = &option.condition {
                            self.expression(condition, &option.span);
                        }
                        self.block(&option.block);
                    }
                }
                StatementKind::ClearLayer { display_layer } => {
                    self.display_layer(display_layer, &statement.span);
                }
                StatementKind::Hide { .. }
                | StatementKind::Nvl { .. }
                | StatementKind::StopMusic { .. }
                | StatementKind::Pause { .. }
                | StatementKind::Move { .. }
                | StatementKind::Transform { .. }
                | StatementKind::Transition { .. } => {}
            }
        }
    }

    fn expression(&mut self, expression: &Expr, span: &crate::syntax::Span) {
        match expression {
            Expr::Variable(name) if !self.assigned_variables.contains(name) => {
                self.push(span, format!("variable `{name}` is never assigned"));
            }
            Expr::Unary { value, .. } => self.expression(value, span),
            Expr::Invoke { arguments, .. } => {
                for argument in arguments {
                    self.expression(argument, span);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.expression(left, span);
                self.expression(right, span);
            }
            Expr::Value(_) | Expr::Variable(_) => {}
        }
    }

    fn resource(&mut self, resource: &str, span: &crate::syntax::Span) {
        let path = Path::new(resource);
        let is_safe = !resource.is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !is_safe {
            self.push(
                span,
                format!("resource path `{resource}` must be a safe relative path"),
            );
            return;
        }
        if !(self.resource_exists)(resource) {
            self.push(span, format!("resource `{resource}` does not exist"));
        }
    }

    fn display_layer(&mut self, name: &str, span: &crate::syntax::Span) {
        if crate::syntax::builtin_display_layer_order(name).is_none()
            && !self.script.display_layers.contains_key(name)
        {
            self.push(span, format!("unknown display layer `{name}`"));
        }
    }

    fn push(&mut self, span: &crate::syntax::Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(
            &span.source,
            span.line,
            span.column,
            message,
        ));
    }
}

fn collect_assignments(block: &Block, assigned: &mut HashSet<String>) {
    for statement in block {
        match &statement.kind {
            StatementKind::Set { variable, .. } | StatementKind::Extension { variable, .. } => {
                assigned.insert(variable.clone());
            }
            StatementKind::If {
                branches,
                else_block,
            } => {
                for (_, block) in branches {
                    collect_assignments(block, assigned);
                }
                collect_assignments(else_block, assigned);
            }
            StatementKind::Menu { options, .. } => {
                for option in options {
                    collect_assignments(&option.block, assigned);
                }
            }
            _ => {}
        }
    }
}

#[must_use]
pub fn parse_hex_color(input: &str) -> Option<[u8; 4]> {
    let hex = input.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let alpha = if hex.len() == 8 {
        u8::from_str_radix(&hex[6..8], 16).ok()?
    } else {
        255
    };
    Some([red, green, blue, alpha])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_script;

    #[test]
    fn reports_unknown_labels_and_unsafe_resources() {
        let script = parse_script(
            "label start:\n    scene \"../secret.png\"\n    jump nowhere",
            "script.rns",
        )
        .unwrap();
        let diagnostics = validate(&script, Path::new("."));
        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("safe relative"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("unknown label"))
        );
    }

    #[test]
    fn reports_unknown_display_layers() {
        let script = parse_script(
            "label start:\n    show \"missing.png\" onlayer effects\n    clear overlay",
            "script.rns",
        )
        .unwrap();
        let diagnostics = validate(&script, Path::new("."));
        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("unknown display layer `effects`"))
        );
        assert!(
            !diagnostics
                .iter()
                .any(|item| item.message.contains("display layer `overlay`"))
        );
    }

    #[test]
    fn parses_supported_colors() {
        assert_eq!(parse_hex_color("#ff0080"), Some([255, 0, 128, 255]));
        assert_eq!(parse_hex_color("#ff008080"), Some([255, 0, 128, 128]));
        assert_eq!(parse_hex_color("red"), None);
    }

    #[test]
    fn validates_label_default_and_call_argument_expressions() {
        let script = parse_script(
            "default known = 1\nlabel start:\n    call target(value=known, other=missing_call)\nlabel target(value, other=missing_default):\n    return",
            "script.rns",
        )
        .unwrap();
        let diagnostics = validate(&script, Path::new("."));
        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("`missing_call`"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("`missing_default`"))
        );
    }

    #[test]
    fn label_parameters_are_assigned_only_inside_their_label() {
        let script = parse_script(
            "label start:\n    return value\nlabel target(value):\n    return value",
            "script.rns",
        )
        .unwrap();
        let diagnostics = validate(&script, Path::new("."));
        assert_eq!(
            diagnostics
                .iter()
                .filter(|item| item.message.contains("variable `value` is never assigned"))
                .count(),
            1
        );
    }
}
