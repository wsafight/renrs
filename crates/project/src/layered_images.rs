use crate::syntax::LayeredImage;
use crate::{Program, ProjectSource, diagnostic::Diagnostic};
use sha2::{Digest, Sha256};

pub(crate) fn load(source: &ProjectSource, program: &mut Program) -> Result<(), Vec<Diagnostic>> {
    let paths: std::collections::BTreeSet<_> = program
        .instructions
        .iter()
        .filter_map(|instruction| {
            if let crate::compiler::InstructionKind::Show { path, alias, .. } = &instruction.kind {
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

fn parse(source: &ProjectSource, path: &str) -> Result<LayeredImage, String> {
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
        if let Some(condition) = &layer.when {
            renrs_compiler::expression::parse_expression(condition, path, 1, 1)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(definition)
}
