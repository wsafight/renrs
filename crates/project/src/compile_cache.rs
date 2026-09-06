use crate::{
    Diagnostic, Program, ProjectSource,
    parser::{ScriptFragment, parse_fragment},
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type CachedFragment = (Vec<u8>, Result<ScriptFragment, Vec<Diagnostic>>);

#[derive(Default)]
pub struct CompileCache {
    fragments: HashMap<String, CachedFragment>,
    parsed: usize,
}

impl CompileCache {
    /// Reuses unchanged parsed files while validating the complete merged project.
    /// # Errors
    /// Returns the same content diagnostics as `ProjectSource::compile`.
    pub fn compile(&mut self, source: &ProjectSource) -> Result<Program, Vec<Diagnostic>> {
        self.parsed = 0;
        let names: Vec<_> = source
            .resource_names()
            .map_err(|error| vec![Diagnostic::new(".", 1, 1, error.to_string())])?
            .into_iter()
            .filter(|path| {
                std::path::Path::new(path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("rns"))
            })
            .collect();
        self.fragments.retain(|name, _| names.contains(name));
        if names.is_empty() {
            return Err(vec![Diagnostic::new(
                ".",
                1,
                1,
                "project does not contain any `.rns` files",
            )]);
        }
        let mut fragments = Vec::new();
        let mut errors = Vec::new();
        for name in names {
            let bytes = match source.read(&name) {
                Ok(bytes) => bytes,
                Err(error) => {
                    errors.push(Diagnostic::new(&name, 1, 1, error.to_string()));
                    continue;
                }
            };
            let hash = Sha256::digest(&bytes).to_vec();
            if self
                .fragments
                .get(&name)
                .is_none_or(|(previous, _)| previous != &hash)
            {
                self.parsed += 1;
                let parsed = String::from_utf8(bytes)
                    .map_err(|error| vec![Diagnostic::new(&name, 1, 1, error.to_string())])
                    .and_then(|text| parse_fragment(&text, &name));
                self.fragments.insert(name.clone(), (hash, parsed));
            }
            match &self.fragments[&name].1 {
                Ok(fragment) => fragments.push(fragment.clone()),
                Err(diagnostics) => errors.extend(diagnostics.clone()),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        source.compile_script(&crate::project::merge_fragments(fragments)?)
    }

    #[must_use]
    pub const fn parsed_files(&self) -> usize {
        self.parsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reuses_unchanged_files_and_revalidates_declarations_and_deletions() {
        let root = tempfile::tempdir().unwrap();
        let a = root.path().join("a.rns");
        let b = root.path().join("b.rns");
        std::fs::write(&a, "label start:\n    call chapter\n    return").unwrap();
        std::fs::write(&b, "label chapter:\n    \"First\"\n    return").unwrap();
        let source = ProjectSource::Directory(root.path().to_owned());
        let mut cache = CompileCache::default();
        cache.compile(&source).unwrap();
        assert_eq!(cache.parsed_files(), 2);
        cache.compile(&source).unwrap();
        assert_eq!(cache.parsed_files(), 0);
        std::fs::write(&b, "label chapter:\n    \"Other\"\n    return").unwrap();
        assert_eq!(
            cache.compile(&source).unwrap().fingerprint,
            source.compile().unwrap().fingerprint
        );
        assert_eq!(cache.parsed_files(), 1);
        std::fs::write(&b, "label start:\n    return").unwrap();
        assert!(cache.compile(&source).is_err());
        std::fs::remove_file(&b).unwrap();
        assert!(cache.compile(&source).is_err());
    }
}
