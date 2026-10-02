use std::fmt::Write;

use super::asset_output::converted_image_with_assumption;
use super::assets::convert_transition;
use super::atl::{StaticTransform, TransformCatalog};
use super::conversion::{LineConversion, unsupported, unsupported_with_code};
use super::generated_assets::{GeneratedAsset, GeneratedAssetKind};

#[allow(clippy::too_many_arguments)]
pub(super) fn convert_scene_statement(
    tokens: &[&str],
    modifier: usize,
    image_tokens: &[&str],
    path: &str,
    assumed: bool,
    transforms: &TransformCatalog,
    parameterized_name: Option<&str>,
    specialized_transform: Option<&StaticTransform>,
) -> LineConversion {
    let mut transform_name = None;
    let mut transition = None;
    let mut index = modifier;
    while index < tokens.len() {
        match tokens[index] {
            "at" if transform_name.is_none() => {
                let Some(name) = tokens.get(index + 1).copied() else {
                    return unsupported("scene `at` clause is missing a transform", false);
                };
                transform_name = Some(name);
                index += 2;
            }
            "with" if transition.is_none() => {
                let Some(name) = tokens.get(index + 1).copied() else {
                    return unsupported("scene `with` clause is missing a transition", false);
                };
                transition = match convert_transition(name) {
                    Ok(value) => Some(value),
                    Err(message) => return unsupported(&message, false),
                };
                index += 2;
            }
            _ => {
                return unsupported(
                    "scene modifiers outside one static `at` and `with` clause require manual migration",
                    false,
                );
            }
        }
    }
    let scene_transform = transform_name.and_then(|name| {
        transforms
            .get(name)
            .or_else(|| specialized_transform.filter(|_| parameterized_name == Some(name)))
    });
    if let Some(name) = transform_name
        && scene_transform.is_none()
    {
        if let Some(reason) = transforms.unsupported_reason(name) {
            return unsupported(reason, false);
        }
        return unsupported_with_code(
            "scene references an unsupported static ATL transform",
            false,
            parameterized_name.map_or("statement_unsupported", |_| "atl_parameters_unsupported"),
        );
    }

    let mut assumptions = Vec::new();
    let mut value = format!("scene \"{path}\"");
    if let Some(transform) = scene_transform {
        let Some(statements) = transform.background_statements() else {
            return unsupported(
                "scene transforms using xalign/yalign require manual migration",
                false,
            );
        };
        for statement in statements {
            write!(value, "\n{statement}").expect("writing to a String cannot fail");
        }
        if let Some(message) = transform.assumption() {
            assumptions.push(message);
        }
    }
    if let Some(transition) = transition {
        value.push('\n');
        value.push_str(&transition.value);
        if let Some(message) = transition.assumption {
            assumptions.push(message);
        }
    }
    let assumption = (!assumptions.is_empty()).then(|| assumptions.join("; "));
    if assumed && image_tokens == ["black"] {
        return LineConversion::Generated {
            value,
            asset: GeneratedAsset {
                path: path.to_owned(),
                kind: GeneratedAssetKind::Png([0, 0, 0, 255]),
            },
            assumption,
        };
    }
    converted_image_with_assumption(value, path, assumed, assumption)
}
