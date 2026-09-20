use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use super::asset_output::{
    converted_image_with_assumption, generated_or_assumed, unsupported_scene_transform,
};
use super::atl::TransformCatalog;
use super::atl_parameters::normalize_and_specialize;
use super::conversion::{LineConversion, unsupported, unsupported_with_code};
use super::expressions::{escape_string, quoted_argument, valid_identifier};
use super::generated_assets::{GeneratedAsset, GeneratedAssetKind};
use super::image_expression::{StaticImageExpression, parse_static_image_expression};
use super::relative_name;

#[derive(Debug)]
pub(super) struct TransitionConversion {
    pub(super) value: String,
    pub(super) assumption: Option<String>,
}
pub(super) struct AssetCatalog {
    by_name: HashMap<String, String>,
    resources: Vec<String>,
}
impl AssetCatalog {
    pub(super) fn new(root: &Path, files: &[PathBuf]) -> Self {
        let mut by_name = HashMap::new();
        for path in files {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let relative = relative_name(root, path);
            by_name
                .entry(stem.to_ascii_lowercase())
                .or_insert_with(|| relative.clone());
            by_name
                .entry(stem.replace('_', " ").to_ascii_lowercase())
                .or_insert(relative);
        }
        let resources = files.iter().map(|path| relative_name(root, path)).collect();
        Self { by_name, resources }
    }
    fn resolve(&self, tokens: &[&str]) -> (String, bool) {
        let name = tokens.join(" ").to_ascii_lowercase();
        if let Some(path) = self.by_name.get(&name) {
            (path.clone(), false)
        } else {
            (format!("images/{}.png", tokens.join("_")), true)
        }
    }
    #[cfg(test)]
    pub(super) fn empty() -> Self {
        Self {
            by_name: HashMap::new(),
            resources: Vec::new(),
        }
    }
    #[cfg(test)]
    pub(super) fn with_image(name: &str, path: &str) -> Self {
        Self {
            by_name: HashMap::from([(name.to_owned(), path.to_owned())]),
            resources: vec![path.to_owned()],
        }
    }
    pub(super) fn find_resource(&self, requested: &str) -> Option<&str> {
        self.resources
            .iter()
            .find(|path| {
                path.eq_ignore_ascii_case(requested)
                    || Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.eq_ignore_ascii_case(requested))
            })
            .map(String::as_str)
    }
}
#[allow(clippy::too_many_lines)]
pub(super) fn convert_image_statement(
    kind: &str,
    source: &str,
    catalog: &AssetCatalog,
    transforms: &TransformCatalog,
) -> LineConversion {
    let expression = match source.strip_prefix("expression ") {
        Some(source) => match parse_static_image_expression(source) {
            Ok(expression) => Some(expression),
            Err(error) => {
                return unsupported_with_code(error.message, false, error.code);
            }
        },
        None => None,
    };
    let source = expression
        .as_ref()
        .map_or(source, |expression| expression.modifiers.as_str());
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
    let modifier = tokens
        .iter()
        .position(|token| {
            matches!(
                *token,
                "at" | "as" | "with" | "onlayer" | "zorder" | "behind"
            )
        })
        .unwrap_or(tokens.len());
    let image_tokens = &tokens[..modifier];
    if image_tokens.is_empty() && expression.is_none() {
        return unsupported("image statement has no static image name", false);
    }
    let mut generated_asset = None;
    let (path, assumed) = if let Some(expression) = &expression {
        if let Some(composite) = expression.composite.as_ref() {
            if kind == "scene" {
                return unsupported_with_code(
                    "scene Composite expressions require a manual background binding",
                    false,
                    "image_expression_composite_unsupported",
                );
            }
            let mut paths = Vec::with_capacity(composite.layers.len());
            for layer in &composite.layers {
                let Some(path) = catalog.find_resource(&layer.path) else {
                    return unsupported_with_code(
                        "Composite layer path does not name a source resource",
                        false,
                        "image_expression_resource_missing",
                    );
                };
                paths.push(path.to_owned());
            }
            let path = composite.generated_path();
            generated_asset = Some(GeneratedAsset {
                path: path.clone(),
                kind: GeneratedAssetKind::Bytes(composite.json_bytes(&paths)),
            });
            (path, false)
        } else {
            let Some(path) = catalog.find_resource(&expression.path) else {
                return unsupported_with_code(
                    "static image expression path does not name a source resource",
                    false,
                    "image_expression_resource_missing",
                );
            };
            (path.to_owned(), false)
        }
    } else {
        let (path, assumed) = catalog.resolve(image_tokens);
        (path, assumed)
    };
    let scene_transform = expression
        .as_ref()
        .is_some_and(StaticImageExpression::has_transform);
    if kind == "scene" && scene_transform {
        return unsupported_scene_transform();
    }
    if kind == "scene" && parameterized_name.is_some() {
        return unsupported_with_code(
            "parameterized ATL scene calls require a manual scene transform binding",
            false,
            "atl_parameters_unsupported",
        );
    }
    if kind == "scene" {
        let transition = match tokens.get(modifier..) {
            None | Some([]) => None,
            Some(["with", transition]) => match convert_transition(transition) {
                Ok(value) => Some(value),
                Err(message) => return unsupported(&message, false),
            },
            Some(_) => {
                return unsupported(
                    "scene modifiers (`at`, `as`, `with`, `onlayer`, `zorder`, or `behind`) require manual migration",
                    false,
                );
            }
        };
        if assumed && image_tokens == ["black"] {
            let mut value = format!("scene \"{path}\"");
            if let Some(transition) = &transition {
                value.push('\n');
                value.push_str(&transition.value);
            }
            return LineConversion::Generated {
                value,
                asset: GeneratedAsset {
                    path,
                    kind: GeneratedAssetKind::Png([0, 0, 0, 255]),
                },
                assumption: transition.and_then(|value| value.assumption),
            };
        }
        let mut value = format!("scene \"{path}\"");
        let assumption = transition.map(|transition| {
            value.push('\n');
            value.push_str(&transition.value);
            transition.assumption
        });
        return converted_image_with_assumption(value, &path, assumed, assumption.flatten());
    }
    let mut alias = image_tokens.first().copied().unwrap_or("expression");
    let mut position = expression
        .as_ref()
        .map_or("center", |expression| expression.position());
    let mut display_layer = None;
    let mut zorder = None;
    let mut transition = None;
    let mut index = modifier;
    let mut saw_alias = false;
    let mut saw_position = false;
    while index < tokens.len() {
        match tokens[index] {
            "as" if !saw_alias => {
                let Some(value) = tokens.get(index + 1).copied() else {
                    return unsupported("show `as` clause is missing an alias", false);
                };
                if !valid_identifier(value) {
                    return unsupported("show alias is not a RenRS identifier", false);
                }
                alias = value;
                saw_alias = true;
                index += 2;
            }
            "at" if !saw_position => {
                let Some(value) = tokens.get(index + 1).copied() else {
                    return unsupported("show `at` clause is missing a position", false);
                };
                if !matches!(value, "left" | "center" | "right")
                    && transforms.get(value).is_none()
                    && parameterized_name.as_deref() != Some(value)
                {
                    if let Some(reason) = transforms.unsupported_reason(value) {
                        return unsupported(reason, false);
                    }
                    return unsupported(
                        "only the static `left`, `center`, and `right` transforms can be migrated",
                        false,
                    );
                }
                position = if matches!(value, "left" | "center" | "right") {
                    value
                } else {
                    transforms.get(value).map_or_else(
                        || {
                            specialized_transform
                                .as_ref()
                                .filter(|_| parameterized_name.as_deref() == Some(value))
                                .and_then(|transform| transform.position)
                                .unwrap_or("center")
                        },
                        |transform| transform.position.unwrap_or("center"),
                    )
                };
                saw_position = true;
                index += 2;
            }
            "with" if transition.is_none() => {
                let Some(value) = tokens.get(index + 1).copied() else {
                    return unsupported("show `with` clause is missing a transition", false);
                };
                transition = match convert_transition(value) {
                    Ok(value) => Some(value),
                    Err(message) => return unsupported(&message, false),
                };
                index += 2;
            }
            "onlayer" if display_layer.is_none() => {
                let Some(value) = tokens.get(index + 1).copied() else {
                    return unsupported("show `onlayer` clause is missing a layer", false);
                };
                if !matches!(value, "master" | "transient" | "screens" | "overlay") {
                    return unsupported(
                        "only standard static Ren'Py display layers can be migrated",
                        false,
                    );
                }
                display_layer = Some(value);
                index += 2;
            }
            "zorder" if zorder.is_none() => {
                let Some(value) = tokens
                    .get(index + 1)
                    .and_then(|value| value.parse::<i32>().ok())
                else {
                    return unsupported("show `zorder` must be a static 32-bit integer", false);
                };
                zorder = Some(value);
                index += 2;
            }
            "behind" => {
                return unsupported("Ren'Py `behind` ordering requires manual migration", false);
            }
            "as" => return unsupported("multiple show `as` clauses are not supported", false),
            "at" => return unsupported("multiple show `at` clauses are not supported", false),
            _ => {
                return unsupported(
                    "show modifier is outside the supported migration subset",
                    false,
                );
            }
        }
    }
    if expression.is_some() && !saw_alias {
        return unsupported_with_code(
            "static show image expressions require an explicit `as` alias",
            false,
            "image_expression_alias_required",
        );
    }
    if !valid_identifier(alias) {
        return unsupported("image alias is not a RenRS identifier", false);
    }
    let mut value = format!("show \"{path}\" as {alias} at {position}");
    if let Some(display_layer) = display_layer {
        write!(value, " onlayer {display_layer}").expect("writing to a String cannot fail");
    }
    if let Some(zorder) = zorder {
        write!(value, " zorder {zorder}").expect("writing to a String cannot fail");
    }
    let transition_assumption = transition.map(|transition| {
        value.push('\n');
        value.push_str(&transition.value);
        transition.assumption
    });
    if let Some(expression) = expression.as_ref() {
        for statement in expression.transform_statements(alias) {
            write!(value, "\n{statement}").expect("writing to a String cannot fail");
        }
    }
    if let Some(name) = tokens.get(modifier..).and_then(|tokens| {
        tokens
            .windows(2)
            .find(|pair| pair[0] == "at")
            .map(|pair| pair[1])
    }) && let Some(transform) = transforms.get(name)
    {
        for statement in transform.statements(alias) {
            write!(value, "\n{statement}").expect("writing to a String cannot fail");
        }
        if let Some(message) = transform.assumption() {
            let message = transition_assumption
                .flatten()
                .map_or(message.clone(), |extra| format!("{message}; {extra}"));
            return generated_or_assumed(value, &path, assumed, Some(message), generated_asset);
        }
    }
    if let Some(transform) = specialized_transform.as_ref().filter(|_| {
        parameterized_name.as_deref()
            == tokens.get(modifier..).and_then(|tokens| {
                tokens
                    .windows(2)
                    .find(|pair| pair[0] == "at")
                    .map(|pair| pair[1])
            })
    }) {
        for statement in transform.statements(alias) {
            write!(value, "\n{statement}").expect("writing to a String cannot fail");
        }
        if let Some(message) = transform.assumption() {
            let message = transition_assumption
                .flatten()
                .map_or(message.clone(), |extra| format!("{message}; {extra}"));
            return generated_or_assumed(value, &path, assumed, Some(message), generated_asset);
        }
    }
    generated_or_assumed(
        value,
        &path,
        assumed,
        transition_assumption.flatten(),
        generated_asset,
    )
}
pub(super) fn convert_image_declaration(source: &str, catalog: &AssetCatalog) -> LineConversion {
    let Some((name, value)) = source
        .strip_prefix("image ")
        .and_then(|rest| rest.split_once('='))
    else {
        return unsupported("image declaration must use a static assignment", false);
    };
    let parts = name.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() || parts.iter().any(|part| !valid_identifier(part)) {
        return unsupported("image declaration name must be static", false);
    }
    let Some(path) = quoted_argument(value.trim()) else {
        return unsupported(
            "only image declarations with a static file path are supported",
            false,
        );
    };
    let Some(path) = catalog.find_resource(&path) else {
        return unsupported(
            "image declaration path does not name a source resource",
            false,
        );
    };
    LineConversion::One(format!(
        "image {} = \"{}\"",
        parts.join("_"),
        escape_string(path)
    ))
}
pub(super) fn convert_transition(value: &str) -> Result<TransitionConversion, String> {
    let (value, assumption) = match value {
        "fade" | "dissolve" => (
            "transition fade 0.5".to_owned(),
            Some(format!("mapped Ren'Py `{value}` to a 0.5 second fade")),
        ),
        "pushleft" => ("transition push left 0.5".to_owned(), None),
        "pushright" => ("transition push right 0.5".to_owned(), None),
        "wipeleft" => ("transition wipe left 0.5".to_owned(), None),
        "wiperight" => ("transition wipe right 0.5".to_owned(), None),
        "hpunch" => ("transition punch h 0.25".to_owned(), None),
        "vpunch" => ("transition punch v 0.25".to_owned(), None),
        _ => return Err("custom transitions require manual migration".to_owned()),
    };
    Ok(TransitionConversion { value, assumption })
}
pub(super) fn static_target(kind: &str, target: &str) -> LineConversion {
    let target = target.trim();
    if valid_identifier(target) {
        LineConversion::One(format!("{kind} {target}"))
    } else {
        let code = match kind {
            "jump" => "jump_target_dynamic",
            "call" => "call_target_dynamic",
            _ => "control_flow_unsupported",
        };
        unsupported_with_code(
            format!(
                "{kind} target `{target}` is dynamic or malformed; only static label targets can be converted"
            ),
            false,
            code,
        )
    }
}
#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
