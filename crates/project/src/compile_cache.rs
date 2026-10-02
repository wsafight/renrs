use crate::{
    Diagnostic, Program, ProjectSource,
    parser::{ScriptFragment, parse_fragment},
    syntax::Script,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type CachedFragment = (String, Result<ScriptFragment, Vec<Diagnostic>>);

#[derive(Default)]
pub struct CompileCache {
    fragments: BTreeMap<String, CachedFragment>,
    script: Option<Script>,
    support_diagnostics: Option<Vec<Diagnostic>>,
    initialized: bool,
    parsed: usize,
}

impl CompileCache {
    /// Reuses unchanged parsed files while validating the complete merged project.
    /// # Errors
    /// Returns the same content diagnostics as `ProjectSource::compile`.
    pub fn compile(&mut self, source: &ProjectSource) -> Result<Program, Vec<Diagnostic>> {
        self.parsed = 0;
        let names: BTreeSet<_> = source
            .resource_names()
            .map_err(|error| vec![Diagnostic::new(".", 1, 1, error.to_string())])?
            .into_iter()
            .filter(|path| is_script(path))
            .collect();
        self.fragments.retain(|name, _| names.contains(name));
        self.script = None;
        for name in names {
            self.refresh(source, &name);
        }
        self.support_diagnostics = Some(source.validate_support_files());
        self.initialized = true;
        self.finish(source)
    }

    /// Recompiles after an exact set of watched resource paths changed.
    /// Unchanged scripts are neither opened nor hashed.
    /// # Errors
    /// Returns the same content diagnostics as `ProjectSource::compile`.
    pub fn compile_changed<'a>(
        &mut self,
        source: &ProjectSource,
        changed: impl IntoIterator<Item = &'a str>,
    ) -> Result<Program, Vec<Diagnostic>> {
        if !self.initialized {
            return self.compile(source);
        }
        self.parsed = 0;
        let changed: BTreeSet<_> = changed.into_iter().collect();
        if changed.contains(crate::resources::RESOURCE_RULES_FILE) {
            return self.compile(source);
        }
        let mut support_changed = false;
        let mut scripts_changed = false;
        for name in changed {
            if is_script(name) {
                if source.contains(name) {
                    scripts_changed |= self.refresh(source, name);
                } else {
                    scripts_changed |= self.fragments.remove(name).is_some();
                }
            } else {
                support_changed = true;
            }
        }
        if support_changed || self.support_diagnostics.is_none() {
            self.support_diagnostics = Some(source.validate_support_files());
        }
        if scripts_changed {
            self.script = None;
        }
        self.finish(source)
    }

    #[must_use]
    pub const fn parsed_files(&self) -> usize {
        self.parsed
    }

    fn refresh(&mut self, source: &ProjectSource, name: &str) -> bool {
        let bytes = match source.read(name) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.fragments.insert(
                    name.to_owned(),
                    (
                        String::new(),
                        Err(vec![Diagnostic::new(name, 1, 1, error.to_string())]),
                    ),
                );
                return true;
            }
        };
        let hash = renrs_shared::hash::sha256_hex(&bytes);
        if self
            .fragments
            .get(name)
            .is_some_and(|(previous, _)| previous == &hash)
        {
            return false;
        }
        self.parsed += 1;
        let parsed = String::from_utf8(bytes)
            .map_err(|error| vec![Diagnostic::new(name, 1, 1, error.to_string())])
            .and_then(|text| parse_fragment(&text, name));
        self.fragments.insert(name.to_owned(), (hash, parsed));
        true
    }

    fn finish(&mut self, source: &ProjectSource) -> Result<Program, Vec<Diagnostic>> {
        if self.fragments.is_empty() {
            return Err(vec![Diagnostic::new(
                ".",
                1,
                1,
                "project does not contain any `.rns` files",
            )]);
        }
        if self.script.is_none() {
            let mut fragments = Vec::with_capacity(self.fragments.len());
            let mut errors = Vec::new();
            for (_, result) in self.fragments.values() {
                match result {
                    Ok(fragment) => fragments.push(fragment.clone()),
                    Err(diagnostics) => errors.extend(diagnostics.clone()),
                }
            }
            if !errors.is_empty() {
                return Err(errors);
            }
            self.script = Some(crate::project::merge_fragments(fragments)?);
        }
        source.compile_script_with_support(
            self.script.as_ref().expect("merged script was cached"),
            self.support_diagnostics.clone().unwrap_or_default(),
        )
    }
}

fn is_script(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rns"))
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
            cache
                .compile_changed(&source, ["b.rns"])
                .unwrap()
                .fingerprint,
            source.compile().unwrap().fingerprint
        );
        assert_eq!(cache.parsed_files(), 1);
        std::fs::write(&b, "label start:\n    return").unwrap();
        assert!(cache.compile_changed(&source, ["b.rns"]).is_err());
        std::fs::remove_file(&b).unwrap();
        assert!(cache.compile_changed(&source, ["b.rns"]).is_err());
    }

    #[test]
    fn exact_changes_skip_unchanged_scripts_and_refresh_resource_rules() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("script.rns"), "label start:\n    return").unwrap();
        let source = ProjectSource::Directory(root.path().to_owned());
        let mut cache = CompileCache::default();
        cache.compile(&source).unwrap();

        std::fs::write(root.path().join("theme.json"), "{}").unwrap();
        cache.compile_changed(&source, ["theme.json"]).unwrap();
        assert_eq!(cache.parsed_files(), 0);

        std::fs::write(
            root.path().join(crate::resources::RESOURCE_RULES_FILE),
            r#"{"include":["theme.json"]}"#,
        )
        .unwrap();
        assert!(
            cache
                .compile_changed(&source, [crate::resources::RESOURCE_RULES_FILE])
                .is_err()
        );
    }
}
