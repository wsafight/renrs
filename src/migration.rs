use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::analysis::analyze;
use crate::compiler::compile;
use crate::diagnostic::{Diagnostic, Severity};
use crate::source::ProjectSource;

mod assets;
mod conversion;
mod expressions;
mod menus;
mod parameters;

use assets::AssetCatalog;
use conversion::convert_script;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationIssueKind {
    Assumption,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationIssue {
    pub file: String,
    pub line: usize,
    pub kind: MigrationIssueKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostValidationDiagnostic {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub severity: PostValidationSeverity,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationReport {
    pub converted_files: usize,
    pub copied_resources: usize,
    #[serde(default)]
    pub generated_resources: usize,
    pub issues: Vec<MigrationIssue>,
    #[serde(default)]
    pub post_validation_diagnostics: Vec<PostValidationDiagnostic>,
}

impl MigrationReport {
    #[must_use]
    pub fn has_strict_failures(&self) -> bool {
        !self.issues.is_empty() || !self.post_validation_diagnostics.is_empty()
    }
}

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("could not access migration data: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not write migration report: {0}")]
    Report(#[from] serde_json::Error),
    #[error("could not generate migration resource: {0}")]
    Image(#[from] image::ImageError),
    #[error("migration output `{0}` must be empty or not exist")]
    OutputNotEmpty(String),
    #[error("input `{0}` is not a .rpy file or directory")]
    InvalidInput(String),
}

/// Converts a Ren'Py project or one `.rpy` file into a `RenRS` project tree.
///
/// # Errors
///
/// Returns an error for invalid inputs, a non-empty destination, inaccessible
/// files, or report serialization failures. Unsupported source constructs are
/// recorded in the successful report instead of being executed.
pub fn migrate_project(input: &Path, output: &Path) -> Result<MigrationReport, MigrationError> {
    ensure_empty_output(output)?;
    fs::create_dir_all(output)?;

    let root = if input.is_dir() {
        input
    } else if input.extension().and_then(|value| value.to_str()) == Some("rpy") {
        input.parent().unwrap_or_else(|| Path::new("."))
    } else {
        return Err(MigrationError::InvalidInput(input.display().to_string()));
    };
    let files = if input.is_dir() {
        collect_files(input)?
    } else {
        vec![input.to_path_buf()]
    };
    let catalog = AssetCatalog::new(root, &files);
    let mut report = MigrationReport::default();
    let mut generated_assets = BTreeMap::new();

    for path in files {
        let relative = relative_name(root, &path);
        if path.extension().and_then(|value| value.to_str()) == Some("rpy") {
            let source = fs::read_to_string(&path)?;
            let converted = convert_script(&source, &relative, &catalog);
            let mut destination = output.join(&relative);
            destination.set_extension("rns");
            write_new_file(&destination, converted.output.as_bytes())?;
            report.converted_files += 1;
            report.issues.extend(converted.issues);
            for asset in converted.generated_assets {
                generated_assets.insert(asset.path, asset.rgba);
            }
        } else if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("rpyc")
        ) {
            let destination = output.join(&relative);
            write_new_file(&destination, &fs::read(path)?)?;
            report.copied_resources += 1;
        }
    }

    for (path, rgba) in generated_assets {
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba(rgba));
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, image::ImageOutputFormat::Png)?;
        write_new_file(&output.join(path), encoded.get_ref())?;
        report.generated_resources += 1;
    }

    report.post_validation_diagnostics = post_validate(output);

    let report_path = output.join("migration-report.json");
    let encoded = serde_json::to_vec_pretty(&report)?;
    write_new_file(&report_path, &encoded)?;
    Ok(report)
}

fn post_validate(output: &Path) -> Vec<PostValidationDiagnostic> {
    let source = ProjectSource::Directory(output.to_path_buf());
    let script = match source.load_script() {
        Ok(script) => script,
        Err(diagnostics) => return diagnostics.into_iter().map(Into::into).collect(),
    };

    let mut diagnostics = source.validate(&script);
    diagnostics.extend(source.validate_support_files());
    if diagnostics.iter().all(|diagnostic| !diagnostic.is_error()) {
        match compile(&script) {
            Ok(program) => diagnostics.extend(analyze(&program)),
            Err(error) => diagnostics.push(Diagnostic::new(
                ".",
                1,
                1,
                format!("migrated project could not compile: {error}"),
            )),
        }
    }
    diagnostics.into_iter().map(Into::into).collect()
}

impl From<Diagnostic> for PostValidationDiagnostic {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            file: diagnostic.file,
            line: diagnostic.line,
            column: diagnostic.column,
            severity: match diagnostic.severity {
                Severity::Error => PostValidationSeverity::Error,
                Severity::Warning => PostValidationSeverity::Warning,
            },
            message: diagnostic.message,
            hint: diagnostic.hint,
        }
    }
}

fn issue(file: &str, line: usize, kind: MigrationIssueKind, message: String) -> MigrationIssue {
    MigrationIssue {
        file: file.to_owned(),
        line,
        kind,
        message,
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name();
            if file_type.is_dir() {
                let name = name.to_string_lossy();
                if !name.starts_with('.') && !matches!(name.as_ref(), "cache" | "saves" | "tl") {
                    pending.push(entry.path());
                }
            } else if file_type.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn ensure_empty_output(output: &Path) -> Result<(), MigrationError> {
    if output.exists() && fs::read_dir(output)?.next().transpose()?.is_some() {
        return Err(MigrationError::OutputNotEmpty(output.display().to_string()));
    }
    Ok(())
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
#[path = "migration/tests.rs"]
mod tests;
