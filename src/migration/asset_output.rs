use super::conversion::{LineConversion, unsupported_with_code};
use super::generated_assets::GeneratedAsset;

pub(super) fn unsupported_scene_transform() -> LineConversion {
    unsupported_with_code(
        "scene Transform requires manual binding",
        false,
        "image_expression_transform_unsupported",
    )
}

pub(super) fn converted_image_with_assumption(
    value: String,
    path: &str,
    assumed: bool,
    extra_assumption: Option<String>,
) -> LineConversion {
    let message = match (assumed, extra_assumption) {
        (true, Some(extra)) => Some(format!("assumed image path `{path}`; {extra}")),
        (true, None) => Some(format!("assumed image path `{path}`")),
        (false, Some(extra)) => Some(extra),
        (false, None) => None,
    };
    message.map_or(LineConversion::One(value.clone()), |message| {
        LineConversion::Assumed { value, message }
    })
}

pub(super) fn generated_or_assumed(
    value: String,
    path: &str,
    assumed: bool,
    extra_assumption: Option<String>,
    generated_asset: Option<GeneratedAsset>,
) -> LineConversion {
    if let Some(asset) = generated_asset {
        let message = match (assumed, extra_assumption) {
            (true, Some(extra)) => Some(format!("assumed image path `{path}`; {extra}")),
            (true, None) => Some(format!("assumed image path `{path}`")),
            (false, extra) => extra,
        };
        return LineConversion::Generated {
            value,
            asset,
            assumption: message,
        };
    }
    converted_image_with_assumption(value, path, assumed, extra_assumption)
}
