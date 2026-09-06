use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use crate::archive::{ArchiveError, ResourceArchive};
use crate::diagnostic::Diagnostic;
use crate::localization::TranslationCatalog;
use crate::project::{load_project, load_project_archive};
use crate::syntax::Script;
use crate::theme::{THEME_FILE, Theme};
use crate::validator::{validate, validate_archive};

#[derive(Debug, Clone)]
pub enum ProjectSource {
    Directory(PathBuf),
    Archive(ResourceArchive),
}

#[derive(Debug, Error)]
pub enum ProjectSourceError {
    #[error("project path `{0}` is not a directory or .renrs archive")]
    InvalidPath(String),
    #[error("resource path `{0}` is not a safe relative path")]
    UnsafePath(String),
    #[error("could not read project resource: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not read project archive: {0}")]
    Archive(#[from] ArchiveError),
}

impl ProjectSource {
    /// Opens a bounded seekable stream. Archive checksums are verified with a fixed buffer.
    /// # Errors
    /// Rejects unsafe, missing or corrupt resources and inaccessible files.
    pub fn open_reader(
        &self,
        path: &str,
    ) -> Result<crate::resource_reader::ResourceReader, ProjectSourceError> {
        if !safe_relative_path(path) {
            return Err(ProjectSourceError::UnsafePath(path.to_owned()));
        }
        match self {
            Self::Directory(root) => {
                let file = fs::File::open(crate::resources::resource_path(root, path)?)?;
                let length = file.metadata()?.len();
                Ok(crate::resource_reader::ResourceReader::new(
                    file, 0, length,
                )?)
            }
            Self::Archive(archive) => Ok(archive.open_reader(path)?),
        }
    }

    /// Compiles a project after validating scripts, support files, and control flow.
    ///
    /// # Errors
    /// Returns source-aware diagnostics for invalid project content.
    pub fn compile(&self) -> Result<crate::Program, Vec<Diagnostic>> {
        let script = self.load_script()?;
        self.compile_script(&script)
    }

    pub(crate) fn compile_script(
        &self,
        script: &Script,
    ) -> Result<crate::Program, Vec<Diagnostic>> {
        let mut diagnostics = self.validate(script);
        diagnostics.extend(self.validate_support_files());
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
        let mut program = crate::compile(script)
            .map_err(|error| vec![Diagnostic::new(".", 1, 1, error.to_string())])?;
        crate::layered_images::load(self, &mut program)?;
        crate::extensions::load(self, &mut program)?;
        for instruction in &program.instructions {
            if let crate::compiler::InstructionKind::Video { path, seconds } = &instruction.kind {
                let result = self
                    .read(path)
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| crate::video::VideoClip::from_slice(&bytes))
                    .and_then(|clip| {
                        if clip.audio.as_ref().is_some_and(|path| !self.contains(path))
                            || clip.frames.iter().any(|frame| !self.contains(frame))
                            || clip
                                .stream
                                .as_ref()
                                .is_some_and(|stream| !self.contains(&stream.path))
                        {
                            return Err("missing video frame".to_owned());
                        }
                        let duration = clip.duration();
                        if (duration - seconds).abs() > 1.0 / clip.fps + 0.001 {
                            return Err(format!("video duration must match clip: {duration:.3}s"));
                        }
                        Ok(())
                    });
                if let Err(error) = result {
                    return Err(vec![Diagnostic::new(
                        &instruction.span.source,
                        instruction.span.line,
                        instruction.span.column,
                        error,
                    )]);
                }
            }
        }
        if self.contains(crate::progress::PROGRESS_FILE) {
            let result = self
                .read(crate::progress::PROGRESS_FILE)
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<crate::progress::ProgressConfig>(&bytes)
                        .map_err(|error| error.to_string())
                })
                .and_then(|config| {
                    config.validate(&program)?;
                    for image in config.gallery.iter().filter_map(|item| item.image.as_ref()) {
                        if !self.contains(image) {
                            return Err(format!("missing gallery image {image}"));
                        }
                    }
                    Ok(config)
                });
            program.progress = result.map_err(|error| {
                vec![Diagnostic::new(crate::progress::PROGRESS_FILE, 1, 1, error)]
            })?;
        }
        let errors: Vec<_> = crate::analyze(&program)
            .into_iter()
            .filter(Diagnostic::is_error)
            .collect();
        if errors.is_empty() {
            Ok(program)
        } else {
            Err(errors)
        }
    }

    /// Opens a directory project or a `.renrs` resource archive.
    ///
    /// # Errors
    ///
    /// Returns an error if the path has the wrong type or the archive header is
    /// invalid.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ProjectSourceError> {
        let path = path.into();
        if path.is_dir() {
            Ok(Self::Directory(path))
        } else if path.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("renrs")
        {
            Ok(Self::Archive(ResourceArchive::open(path)?))
        } else {
            Err(ProjectSourceError::InvalidPath(path.display().to_string()))
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Directory(path) => path,
            Self::Archive(archive) => archive.path(),
        }
    }

    #[must_use]
    pub fn watch_root(&self) -> Option<&Path> {
        match self {
            Self::Directory(path) => Some(path),
            Self::Archive(_) => None,
        }
    }

    /// Loads and merges every script exposed by this source.
    ///
    /// # Errors
    ///
    /// Returns source-aware project diagnostics.
    pub fn load_script(&self) -> Result<Script, Vec<Diagnostic>> {
        match self {
            Self::Directory(path) => load_project(path),
            Self::Archive(archive) => load_project_archive(archive),
        }
    }

    #[must_use]
    pub fn validate(&self, script: &Script) -> Vec<Diagnostic> {
        match self {
            Self::Directory(path) => validate(script, path),
            Self::Archive(archive) => validate_archive(script, archive),
        }
    }

    /// Validates optional project-level theme, font, and translation files.
    #[must_use]
    pub fn validate_support_files(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        self.validate_theme(&mut diagnostics);
        self.validate_catalogs(&mut diagnostics);
        self.validate_screens(&mut diagnostics);
        diagnostics
    }

    #[must_use]
    pub fn contains(&self, relative_path: &str) -> bool {
        if !safe_relative_path(relative_path) {
            return false;
        }
        match self {
            Self::Directory(root) => crate::resources::resource_path(root, relative_path).is_ok(),
            Self::Archive(archive) => archive.contains(relative_path),
        }
    }

    /// Reads one checksum-validated resource from a directory or archive.
    ///
    /// # Errors
    ///
    /// Returns an error for unsafe paths, filesystem failures, missing archive
    /// entries, or checksum failures.
    pub fn read(&self, relative_path: &str) -> Result<Vec<u8>, ProjectSourceError> {
        if !safe_relative_path(relative_path) {
            return Err(ProjectSourceError::UnsafePath(relative_path.to_owned()));
        }
        match self {
            Self::Directory(root) => Ok(fs::read(crate::resources::resource_path(
                root,
                relative_path,
            )?)?),
            Self::Archive(archive) => Ok(archive.read(relative_path)?),
        }
    }

    /// Reads a resource only when its encoded size fits the caller's byte budget.
    /// # Errors
    /// Returns an error for unsafe paths, oversized resources, or I/O failure.
    pub fn read_limited(&self, path: &str, limit: usize) -> Result<Vec<u8>, ProjectSourceError> {
        if !safe_relative_path(path) {
            return Err(ProjectSourceError::UnsafePath(path.to_owned()));
        }
        match self {
            Self::Directory(root) => {
                use std::io::Read;
                let file = fs::File::open(crate::resources::resource_path(root, path)?)?;
                if file.metadata()?.len() > limit as u64 {
                    return Err(
                        std::io::Error::other("encoded resource exceeds byte budget").into(),
                    );
                }
                let mut bytes = Vec::new();
                file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
                if bytes.len() > limit {
                    return Err(
                        std::io::Error::other("encoded resource exceeds byte budget").into(),
                    );
                }
                Ok(bytes)
            }
            Self::Archive(archive) => {
                if archive
                    .entries()
                    .iter()
                    .any(|entry| entry.path == path && entry.length > limit as u64)
                {
                    return Err(
                        std::io::Error::other("encoded resource exceeds byte budget").into(),
                    );
                }
                Ok(archive.read(path)?)
            }
        }
    }

    /// Lists every visible resource using normalized forward-slash paths.
    ///
    /// # Errors
    ///
    /// Returns an error when a directory project cannot be traversed.
    pub fn resource_names(&self) -> Result<Vec<String>, ProjectSourceError> {
        let mut names = match self {
            Self::Archive(archive) => archive
                .entries()
                .iter()
                .map(|entry| entry.path.clone())
                .collect(),
            Self::Directory(root) => collect_resource_names(root)?,
        };
        names.sort_unstable();
        Ok(names)
    }

    fn validate_theme(&self, diagnostics: &mut Vec<Diagnostic>) {
        if !self.contains(THEME_FILE) {
            return;
        }
        let theme = self
            .read(THEME_FILE)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                Theme::from_slice(&bytes, THEME_FILE).map_err(|error| error.to_string())
            });
        match theme {
            Ok(theme) => {
                for font in theme.font_paths().filter(|font| !self.contains(font)) {
                    diagnostics.push(Diagnostic::new(
                        THEME_FILE,
                        1,
                        1,
                        format!("theme font `{font}` does not exist"),
                    ));
                }
            }
            Err(error) => diagnostics.push(Diagnostic::new(THEME_FILE, 1, 1, error)),
        }
    }

    fn validate_screens(&self, diagnostics: &mut Vec<Diagnostic>) {
        let path = crate::screens::SCREENS_FILE;
        if !self.contains(path) {
            return;
        }
        match self
            .read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| crate::screens::Screens::from_slice(&bytes))
        {
            Ok(screens) => {
                for image in screens.images() {
                    if !self.contains(&image) {
                        diagnostics.push(Diagnostic::new(
                            path,
                            1,
                            1,
                            format!("screen image is missing or unsafe: {image}"),
                        ));
                    }
                }
            }
            Err(error) => diagnostics.push(Diagnostic::new(path, 1, 1, error)),
        }
    }

    fn validate_catalogs(&self, diagnostics: &mut Vec<Diagnostic>) {
        let names = match self.resource_names() {
            Ok(names) => names,
            Err(error) => {
                diagnostics.push(Diagnostic::new(".", 1, 1, error.to_string()));
                return;
            }
        };
        let mut languages = BTreeMap::<String, String>::new();
        for path in names.into_iter().filter(|path| {
            path.starts_with("locales/")
                && Path::new(path)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        }) {
            let catalog = self
                .read(&path)
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    TranslationCatalog::from_reader(std::io::Cursor::new(bytes))
                        .map_err(|error| error.to_string())
                });
            match catalog {
                Ok(catalog) => {
                    if let Some(first) = languages.insert(catalog.language.clone(), path.clone()) {
                        diagnostics.push(Diagnostic::new(
                            &path,
                            1,
                            1,
                            format!(
                                "language `{}` is also declared by `{first}`",
                                catalog.language
                            ),
                        ));
                    }
                }
                Err(error) => diagnostics.push(Diagnostic::new(&path, 1, 1, error)),
            }
        }
    }
}

fn collect_resource_names(root: &Path) -> Result<Vec<String>, std::io::Error> {
    Ok(crate::resources::collect_files(root)?
        .iter()
        .map(|path| crate::resources::relative_name(root, path))
        .collect())
}

fn safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::pack_project;

    #[test]
    fn directory_and_archive_expose_the_same_project() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("game");
        fs::create_dir_all(root.join("images")).unwrap();
        fs::write(
            root.join("script.rns"),
            "label start:\n    scene \"images/bg.bin\"\n    return\n",
        )
        .unwrap();
        fs::write(root.join("images/bg.bin"), [1, 2, 3]).unwrap();
        let archive_path = temporary.path().join("game.renrs");
        pack_project(&root, &archive_path).unwrap();

        let directory = ProjectSource::open(&root).unwrap();
        let archive = ProjectSource::open(&archive_path).unwrap();
        let directory_script = directory.load_script().unwrap();
        let archive_script = archive.load_script().unwrap();
        assert!(directory.validate(&directory_script).is_empty());
        assert!(archive.validate(&archive_script).is_empty());
        assert_eq!(directory.read("images/bg.bin").unwrap(), [1, 2, 3]);
        assert_eq!(archive.read("images/bg.bin").unwrap(), [1, 2, 3]);
        assert_eq!(
            directory.resource_names().unwrap(),
            archive.resource_names().unwrap()
        );
    }

    #[test]
    fn rejects_unsafe_resource_paths() {
        let source = ProjectSource::Directory(PathBuf::from("unused"));
        assert!(matches!(
            source.read("../secret"),
            Err(ProjectSourceError::UnsafePath(_))
        ));
    }

    #[test]
    fn validates_theme_fonts_and_translation_catalogs() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        fs::create_dir(root.join("locales")).unwrap();
        fs::write(
            root.join(THEME_FILE),
            r#"{"font_path":"fonts/missing.ttf"}"#,
        )
        .unwrap();
        fs::write(root.join("locales/zh.json"), b"not json").unwrap();
        let source = ProjectSource::open(root).unwrap();

        let diagnostics = source.validate_support_files();

        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("theme font"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|item| item.file == "locales/zh.json")
        );
    }

    #[test]
    fn rejects_duplicate_catalog_languages_from_an_archive() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("game");
        fs::create_dir_all(root.join("locales")).unwrap();
        fs::write(root.join("script.rns"), "label start:\n    return\n").unwrap();
        let catalog = r#"{"language":"zh","messages":{}}"#;
        fs::write(root.join("locales/a.json"), catalog).unwrap();
        fs::write(root.join("locales/b.json"), catalog).unwrap();
        let archive_path = temporary.path().join("game.renrs");
        pack_project(&root, &archive_path).unwrap();
        let source = ProjectSource::open(archive_path).unwrap();

        let diagnostics = source.validate_support_files();

        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains("also declared"))
        );
    }
}
