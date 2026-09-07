use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use super::conversion::{LineConversion, unsupported};
use super::expressions::valid_identifier;
use super::relative_name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GeneratedAsset {
    pub(super) path: String,
    pub(super) rgba: [u8; 4],
}

pub(super) struct AssetCatalog {
    by_name: HashMap<String, String>,
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
        Self { by_name }
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
        }
    }

    #[cfg(test)]
    pub(super) fn with_image(name: &str, path: &str) -> Self {
        Self {
            by_name: HashMap::from([(name.to_owned(), path.to_owned())]),
        }
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn convert_image_statement(
    kind: &str,
    source: &str,
    catalog: &AssetCatalog,
) -> LineConversion {
    if source.starts_with("expression ") {
        return unsupported("dynamic image expressions require manual migration", false);
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
    if image_tokens.is_empty() {
        return unsupported("image statement has no static image name", false);
    }
    let (path, assumed) = catalog.resolve(image_tokens);
    if kind == "scene" {
        if modifier != tokens.len() {
            return unsupported(
                "scene modifiers (`at`, `as`, `with`, `onlayer`, `zorder`, or `behind`) require manual migration",
                false,
            );
        }
        if assumed && image_tokens == ["black"] {
            return LineConversion::Generated {
                value: format!("scene \"{path}\""),
                asset: GeneratedAsset {
                    path,
                    rgba: [0, 0, 0, 255],
                },
            };
        }
        return converted_image(format!("scene \"{path}\""), &path, assumed);
    }

    let mut alias = image_tokens[0];
    let mut position = "center";
    let mut display_layer = None;
    let mut zorder = None;
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
                if !matches!(value, "left" | "center" | "right") {
                    return unsupported(
                        "only the static `left`, `center`, and `right` transforms can be migrated",
                        false,
                    );
                }
                position = value;
                saw_position = true;
                index += 2;
            }
            "with" => {
                return unsupported(
                    "a `with` clause attached to `show` requires manual transition migration",
                    false,
                );
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
    converted_image(value, &path, assumed)
}

fn converted_image(value: String, path: &str, assumed: bool) -> LineConversion {
    if assumed {
        LineConversion::Assumed {
            value,
            message: format!("assumed image path `{path}`"),
        }
    } else {
        LineConversion::One(value)
    }
}

pub(super) fn static_target(kind: &str, target: &str) -> LineConversion {
    let target = target.trim();
    if valid_identifier(target) {
        LineConversion::One(format!("{kind} {target}"))
    } else {
        unsupported("only static label targets can be converted", false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_alias_and_rejects_attached_transition() {
        let catalog = AssetCatalog::with_image("eileen happy", "images/eileen_happy.png");
        let supported = convert_image_statement("show", "eileen happy as hero at left", &catalog);
        assert!(matches!(
            supported,
            LineConversion::One(ref value)
                if value == "show \"images/eileen_happy.png\" as hero at left"
        ));
        let attached = convert_image_statement("show", "eileen happy with dissolve", &catalog);
        assert!(matches!(
            attached,
            LineConversion::Unsupported { ref message, .. }
                if message.contains("attached to `show`")
        ));
    }

    #[test]
    fn converts_standard_display_layer_and_static_zorder() {
        let catalog = AssetCatalog::with_image("eileen happy", "images/eileen_happy.png");
        let converted =
            convert_image_statement("show", "eileen happy onlayer transient zorder 20", &catalog);
        assert!(matches!(
            converted,
            LineConversion::One(ref value)
                if value == "show \"images/eileen_happy.png\" as eileen at center onlayer transient zorder 20"
        ));
        assert!(matches!(
            convert_image_statement("show", "eileen happy onlayer custom", &catalog),
            LineConversion::Unsupported { ref message, .. }
                if message.contains("standard static")
        ));
    }
}
