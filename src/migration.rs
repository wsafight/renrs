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
mod atl;
mod audio;
mod conversion;
mod expressions;
mod menus;
mod parameters;
mod static_values;
mod story;
mod support;

use assets::AssetCatalog;
use conversion::convert_script;
use support::SupportAccumulator;

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
    #[serde(default)]
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationSupportStatus {
    Converted,
    Partial,
    BuiltInReplacement,
    ManualReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationSupportFile {
    pub file: String,
    pub kind: String,
    pub status: MigrationSupportStatus,
    pub mapped_keys: Vec<String>,
    pub unmapped_keys: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationSummary {
    pub assumptions: usize,
    pub unsupported: usize,
    pub by_code: BTreeMap<String, usize>,
    pub by_file: BTreeMap<String, usize>,
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
    #[serde(default = "report_version")]
    pub version: u32,
    #[serde(default)]
    pub source_root: String,
    #[serde(default)]
    pub output_root: String,
    pub converted_files: usize,
    pub copied_resources: usize,
    #[serde(default)]
    pub generated_resources: usize,
    #[serde(default)]
    pub generated_support_files: usize,
    #[serde(default)]
    pub support_files: Vec<MigrationSupportFile>,
    pub issues: Vec<MigrationIssue>,
    #[serde(default)]
    pub summary: MigrationSummary,
    #[serde(default)]
    pub post_validation_diagnostics: Vec<PostValidationDiagnostic>,
}

const fn report_version() -> u32 {
    1
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
    let mut report = MigrationReport {
        version: report_version(),
        source_root: root
            .canonicalize()
            .unwrap_or_else(|_| root.to_path_buf())
            .display()
            .to_string(),
        output_root: output
            .canonicalize()
            .unwrap_or_else(|_| output.to_path_buf())
            .display()
            .to_string(),
        ..MigrationReport::default()
    };
    let mut generated_assets = BTreeMap::new();
    let mut support = SupportAccumulator::default();

    for path in files {
        let relative = relative_name(root, &path);
        if path.extension().and_then(|value| value.to_str()) == Some("rpy") {
            let source = fs::read_to_string(&path)?;
            let converted = support
                .convert(&relative, &source, &catalog)
                .unwrap_or_else(|| convert_script(&source, &relative, &catalog));
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

    report.support_files = std::mem::take(&mut support.files);
    let generated_support = support.finish()?;
    for (path, bytes) in [
        ("theme.json", generated_support.theme),
        ("screens.json", generated_support.screens),
    ] {
        if let Some(bytes) = bytes {
            let destination = output.join(path);
            if destination.exists() {
                report.issues.push(coded_issue(
                    path,
                    1,
                    MigrationIssueKind::Unsupported,
                    "support_output_conflict",
                    format!(
                        "could not generate `{path}` because the source project already contains it"
                    ),
                ));
            } else {
                write_new_file(&destination, &bytes)?;
                report.generated_support_files += 1;
            }
        }
    }

    report.post_validation_diagnostics = post_validate(output);
    report.summary = summarize(&report.issues);

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
    let code = issue_code(kind, &message).to_owned();
    coded_issue(file, line, kind, &code, message)
}

fn coded_issue(
    file: &str,
    line: usize,
    kind: MigrationIssueKind,
    code: &str,
    message: String,
) -> MigrationIssue {
    MigrationIssue {
        file: file.to_owned(),
        line,
        kind,
        code: code.to_owned(),
        message,
    }
}

fn summarize(issues: &[MigrationIssue]) -> MigrationSummary {
    let mut summary = MigrationSummary::default();
    for issue in issues {
        match issue.kind {
            MigrationIssueKind::Assumption => summary.assumptions += 1,
            MigrationIssueKind::Unsupported => summary.unsupported += 1,
        }
        *summary.by_code.entry(issue.code.clone()).or_default() += 1;
        *summary.by_file.entry(issue.file.clone()).or_default() += 1;
    }
    summary
}

fn issue_code(kind: MigrationIssueKind, message: &str) -> &'static str {
    if message.contains("fallthrough") {
        "label_fallthrough"
    } else if message.contains("`ease` interpolation") {
        "atl_easing_assumed"
    } else if message.starts_with("assumed image path") {
        "image_path_assumed"
    } else if message.starts_with("mapped Ren'Py") {
        "transition_duration_assumed"
    } else if message.contains("Character") || message.contains("character names") {
        "character_declaration_unsupported"
    } else if message.contains("Python") || message.contains("screen language") {
        "block_unsupported"
    } else if message.contains("image") || message.contains("show") || message.contains("layer") {
        "display_statement_unsupported"
    } else if message.contains("label") || message.contains("call") || message.contains("return") {
        "control_flow_unsupported"
    } else if message.contains("menu") {
        "menu_unsupported"
    } else if kind == MigrationIssueKind::Assumption {
        "migration_assumption"
    } else {
        "statement_unsupported"
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
