use crate::{Program, ProjectSource, diagnostic::Diagnostic};
use renrs_model::{CompiledImageLayer, CompiledLayeredImage};
use renrs_syntax::presentation::LayeredImage;
use sha2::{Digest, Sha256};

pub(crate) fn load(source: &ProjectSource, program: &mut Program) -> Result<(), Vec<Diagnostic>> {
    let paths: std::collections::BTreeSet<_> = program
        .instructions
        .iter()
        .filter_map(|instruction| {
            if let renrs_model::InstructionKind::Show { path, alias, .. } = &instruction.kind {
                if alias == "camera" {
                    return Some(Err("camera is a reserved image alias".to_owned()));
                }
                if path.ends_with(".layers.json") {
                    return Some(Ok(path.clone()));
                }
            }
            None
        })
        .collect::<Result<_, _>>()
        .map_err(|error| vec![Diagnostic::new(".", 1, 1, error)])?;
    for path in paths {
        let definition =
            parse(source, &path).map_err(|error| vec![Diagnostic::new(&path, 1, 1, error)])?;
        program.layered_images.insert(path, definition);
    }
    if !program.layered_images.is_empty() {
        let payload = serde_json::to_vec(&program.layered_images)
            .expect("validated layered images serialize");
        let mut hash = Sha256::new();
        hash.update(program.fingerprint.as_bytes());
        hash.update(payload);
        program.fingerprint = format!("{:x}", hash.finalize());
    }
    Ok(())
}

fn parse(source: &ProjectSource, path: &str) -> Result<CompiledLayeredImage, String> {
    let definition: LayeredImage = serde_json::from_slice(
        &source
            .read_limited(path, 1024 * 1024)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if !(1..=8192).contains(&definition.width)
        || !(1..=8192).contains(&definition.height)
        || definition.layers.is_empty()
        || definition.layers.len() > 32
    {
        return Err("layered images need a canvas in 1..8192 and 1..32 layers".to_owned());
    }
    for layer in &definition.layers {
        if !layer.x.is_finite()
            || !layer.y.is_finite()
            || layer.frames.len() > 64
            || layer.frames.iter().any(|frame| {
                !frame.seconds.is_finite() || !(0.016..=3600.0).contains(&frame.seconds)
            })
        {
            return Err("invalid image layer offset or animation timing".to_owned());
        }
        for image in layer.paths() {
            if !source.contains(image) {
                return Err(format!("missing or unsafe layer image: {image}"));
            }
        }
    }
    let layers = definition
        .layers
        .into_iter()
        .map(|layer| {
            let condition = layer
                .when
                .as_deref()
                .map(|condition| {
                    renrs_compiler::expression::parse_expression(condition, path, 1, 1)
                        .map_err(|error| error.to_string())
                })
                .transpose()?;
            Ok(CompiledImageLayer {
                path: layer.path,
                condition,
                x: layer.x,
                y: layer.y,
                frames: layer.frames,
                speaking: layer.speaking,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(CompiledLayeredImage {
        width: definition.width,
        height: definition.height,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::ProjectSource;
    use std::fs;

    fn compile_layers(definition: &str) -> Result<Program, Vec<Diagnostic>> {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("body.png"), []).unwrap();
        fs::write(root.path().join("actor.layers.json"), definition).unwrap();
        fs::write(
            root.path().join("script.rns"),
            "label start:\n    show \"actor.layers.json\" as actor\n    return\n",
        )
        .unwrap();
        ProjectSource::open(root.path()).unwrap().compile()
    }

    #[test]
    fn rejects_invalid_canvas_missing_images_and_camera_aliases() {
        assert!(
            compile_layers(r#"{"width":0,"height":100,"layers":[{"path":"body.png"}]}"#).is_err()
        );
        assert!(compile_layers(r#"{"width":100,"height":100,"layers":[]}"#).is_err());
        assert!(
            compile_layers(r#"{"width":100,"height":100,"layers":[{"path":"missing.png"}]}"#)
                .is_err()
        );

        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("script.rns"),
            "label start:\n    show \"a.png\" as camera\n    return\n",
        )
        .unwrap();
        fs::write(root.path().join("a.png"), []).unwrap();
        let errors = ProjectSource::open(root.path())
            .unwrap()
            .compile()
            .unwrap_err();
        assert!(errors.iter().any(|item| item.message.contains("reserved")));
    }

    #[test]
    fn compiles_layer_conditions_and_frame_paths() {
        let program = compile_layers(
            r#"{"width":100,"height":200,"layers":[{"path":"body.png","when":"true","frames":[{"path":"body.png","seconds":0.2}]}]}"#,
        )
        .unwrap();
        let layer = &program.layered_images["actor.layers.json"].layers[0];
        assert!(layer.condition.is_some());
        assert_eq!(layer.paths().count(), 2);
    }
}
