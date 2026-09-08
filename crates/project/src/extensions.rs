use crate::{Diagnostic, Program, ProjectSource};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    modules: BTreeMap<String, String>,
}

pub(crate) fn load(source: &ProjectSource, program: &mut Program) -> Result<(), Vec<Diagnostic>> {
    load_inner(source, program)
        .map_err(|error| vec![Diagnostic::new("extensions.json", 1, 1, error)])
}

fn load_inner(source: &ProjectSource, program: &mut Program) -> Result<(), String> {
    if source.contains("extensions.json") {
        let manifest: Manifest = serde_json::from_slice(
            &source
                .read_limited("extensions.json", 65536)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if manifest.version != 1 || manifest.modules.len() > 64 {
            return Err("unsupported extension manifest".to_owned());
        }
        for (name, path) in manifest.modules {
            let bytes = source
                .read_limited(&path, 1024 * 1024)
                .map_err(|error| error.to_string())?;
            program.extensions.insert(
                name,
                String::from_utf8(bytes).map_err(|error| error.to_string())?,
            );
        }
        renrs_extensions::Extensions::new(&program.extensions)?;
        let mut hash = Sha256::new();
        hash.update(program.fingerprint.as_bytes());
        hash.update(serde_json::to_vec(&program.extensions).map_err(|error| error.to_string())?);
        program.fingerprint = format!("{:x}", hash.finalize());
    }
    for instruction in &program.instructions {
        if let renrs_model::InstructionKind::Extension { name, .. } = &instruction.kind
            && !program.extensions.contains_key(name)
        {
            return Err(format!(
                "unknown extension {name} at {}:{}",
                instruction.span.source, instruction.span.line
            ));
        }
    }
    Ok(())
}
