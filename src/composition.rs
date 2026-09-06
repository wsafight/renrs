use image::{RgbaImage, imageops::overlay};
use serde::Deserialize;
use std::fmt::Write;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Composition {
    version: u32,
    width: u32,
    height: u32,
    layers: BTreeMap<String, Layer>,
    presets: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Layer {
    path: String,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
}

/// Composes explicitly named character presets and their script declarations.
/// # Errors
/// Rejects unsafe paths, oversized assets, missing layers and existing output.
pub fn compose(
    root: &Path,
    manifest: &Path,
    relative: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    if !renrs_project::resources::visible_path(relative) {
        return Err("unsafe composition destination".into());
    }
    let destination = root.join(relative);
    if destination.exists() {
        return Err("composition destination exists".into());
    }
    let composition: Composition = serde_json::from_slice(&fs::read(manifest)?)?;
    if composition.version != 1
        || !(1..=4096).contains(&composition.width)
        || !(1..=4096).contains(&composition.height)
        || composition.layers.is_empty()
        || composition.layers.len() > 64
        || composition.presets.is_empty()
        || composition.presets.len() > 128
    {
        return Err(
            "composition requires version 1, dimensions 1..4096, 1..64 layers and 1..128 presets"
                .into(),
        );
    }
    let root = root.canonicalize()?;
    let mut decoded = BTreeMap::new();
    let mut bytes = 0_u64;
    for (name, layer) in &composition.layers {
        if !renrs_project::resources::visible_path(&layer.path) {
            return Err("unsafe layer path".into());
        }
        let path = root.join(&layer.path).canonicalize()?;
        if !path.starts_with(&root) {
            return Err("layer escapes project".into());
        }
        let (width, height) = image::image_dimensions(&path)?;
        bytes += u64::from(width) * u64::from(height) * 4;
        if width > 4096 || height > 4096 || bytes > 128 * 1024 * 1024 {
            return Err("composition exceeds decoded image budget".into());
        }
        decoded.insert(name, image::open(path)?.into_rgba8());
    }
    let parent = destination.parent().ok_or("invalid destination")?;
    fs::create_dir_all(parent)?;
    if !parent.canonicalize()?.starts_with(&root) {
        return Err("destination escapes project".into());
    }
    let temporary = tempfile::tempdir_in(parent)?;
    let mut declarations = String::new();
    for (name, layers) in &composition.presets {
        if name.is_empty()
            || !name.bytes().enumerate().all(|(index, byte)| {
                byte == b'_' || byte.is_ascii_alphabetic() || index > 0 && byte.is_ascii_digit()
            })
            || layers.is_empty()
            || layers.len() > 32
        {
            return Err("invalid preset name or layer count".into());
        }
        let mut canvas = RgbaImage::new(composition.width, composition.height);
        for name in layers {
            let layer = composition.layers.get(name).ok_or("unknown preset layer")?;
            overlay(
                &mut canvas,
                &decoded[name],
                i64::from(layer.x),
                i64::from(layer.y),
            );
        }
        canvas.save(temporary.path().join(format!("{name}.png")))?;
        writeln!(
            declarations,
            "image {name} = {}",
            serde_json::to_string(&format!("{relative}/{name}.png"))?
        )?;
    }
    fs::write(temporary.path().join("images.rns"), declarations)?;
    fs::rename(temporary.path(), destination)?;
    Ok(composition.presets.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_layers_produce_reusable_variants_and_refuse_overwrite() {
        let root = tempfile::tempdir().unwrap();
        RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]))
            .save(root.path().join("base.png"))
            .unwrap();
        RgbaImage::from_pixel(1, 1, image::Rgba([0, 255, 0, 255]))
            .save(root.path().join("face.png"))
            .unwrap();
        let manifest = root.path().join("character.json");
        fs::write(&manifest, r#"{"version":1,"width":4,"height":4,"layers":{"body":{"path":"base.png"},"face":{"path":"face.png","x":1,"y":2}},"presets":{"mira_smile":["body","face"]}}"#).unwrap();
        assert_eq!(
            compose(root.path(), &manifest, "images/variants").unwrap(),
            1
        );
        let image = image::open(root.path().join("images/variants/mira_smile.png"))
            .unwrap()
            .into_rgba8();
        assert_eq!(image.get_pixel(1, 2).0, [0, 255, 0, 255]);
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert!(compose(root.path(), &manifest, "images/variants").is_err());
        assert!(compose(root.path(), &manifest, "../escape").is_err());
    }
}
