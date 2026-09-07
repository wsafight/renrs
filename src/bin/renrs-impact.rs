use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};

use renrs::impact::analyze_impact;
use renrs::protocol::{self, MachineEnvelope, MachineError};

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(report) => {
            if let Err(error) = protocol::print(&MachineEnvelope::success("impact", report)) {
                eprintln!("error: {error}");
                ExitCode::from(protocol::EXIT_FAILURE)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            let exit = if error.usage {
                protocol::EXIT_USAGE
            } else {
                protocol::EXIT_FAILURE
            };
            let envelope = MachineEnvelope::<serde_json::Value>::completed(
                "impact",
                false,
                None,
                error.diagnostics,
                Some(MachineError::new(error.code, error.message)),
            );
            let _ = protocol::print(&envelope);
            ExitCode::from(exit)
        }
    }
}

struct CliError {
    code: &'static str,
    message: String,
    diagnostics: Vec<renrs::Diagnostic>,
    usage: bool,
}

impl CliError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            diagnostics: Vec::new(),
            usage: false,
        }
    }

    fn usage() -> Self {
        Self {
            usage: true,
            ..Self::new(
                protocol::code::INVALID_ARGUMENTS,
                "usage: renrs-impact <baseline> <candidate>\n       renrs-impact --git <candidate> [base-ref]",
            )
        }
    }
}

fn run(arguments: &[std::ffi::OsString]) -> Result<renrs::impact::ImpactReport, CliError> {
    match arguments {
        [baseline, candidate] if baseline != "--git" => {
            compare(Path::new(baseline), Path::new(candidate))
        }
        [flag, candidate] if flag == "--git" => {
            compare_git(Path::new(candidate), OsStr::new("HEAD"))
        }
        [flag, candidate, revision] if flag == "--git" => {
            compare_git(Path::new(candidate), revision)
        }
        _ => Err(CliError::usage()),
    }
}

fn compare(baseline: &Path, candidate: &Path) -> Result<renrs::impact::ImpactReport, CliError> {
    analyze_impact(baseline, candidate).map_err(|failure| CliError {
        code: failure.code,
        message: failure.message,
        diagnostics: failure.diagnostics,
        usage: false,
    })
}

fn compare_git(
    candidate: &Path,
    revision: &OsStr,
) -> Result<renrs::impact::ImpactReport, CliError> {
    let baseline = git_baseline(candidate, revision)
        .map_err(|message| CliError::new(protocol::code::GIT_BASELINE_FAILED, message))?;
    compare(baseline.path(), candidate)
}

fn git_baseline(candidate: &Path, revision: &OsStr) -> Result<tempfile::TempDir, String> {
    let candidate = candidate
        .canonicalize()
        .map_err(|error| format!("could not resolve candidate: {error}"))?;
    if !candidate.is_dir() {
        return Err("Git impact mode requires a candidate directory".to_owned());
    }
    let repository = git_text(&candidate, ["rev-parse", "--show-toplevel"])?;
    let repository = PathBuf::from(repository.trim());
    let prefix = git_text(&candidate, ["rev-parse", "--show-prefix"])?;
    let prefix = prefix.trim().trim_end_matches('/');
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&repository)
        .args(["ls-tree", "-r", "-z", "--name-only"])
        .arg(revision);
    if !prefix.is_empty() {
        command.args(["--", prefix]);
    }
    let listing = command.output().map_err(|error| error.to_string())?;
    if !listing.status.success() {
        return Err(String::from_utf8_lossy(&listing.stderr).trim().to_owned());
    }
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut count = 0;
    for encoded in listing
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let repository_path = std::str::from_utf8(encoded).map_err(|error| error.to_string())?;
        let relative = if prefix.is_empty() {
            repository_path
        } else {
            repository_path
                .strip_prefix(prefix)
                .and_then(|path| path.strip_prefix('/'))
                .ok_or("Git returned a path outside the candidate project")?
        };
        if relative.is_empty()
            || Path::new(relative)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(format!("Git returned an unsafe project path: {relative}"));
        }
        let content = Command::new("git")
            .arg("-C")
            .arg(&repository)
            .arg("show")
            .arg(format!("{}:{repository_path}", revision.to_string_lossy()))
            .output()
            .map_err(|error| error.to_string())?;
        if !content.status.success() {
            return Err(String::from_utf8_lossy(&content.stderr).trim().to_owned());
        }
        let output = temporary.path().join(relative);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(output, content.stdout).map_err(|error| error.to_string())?;
        count += 1;
    }
    if count == 0 {
        return Err("Git baseline contains no files for the candidate project".to_owned());
    }
    Ok(temporary)
}

fn git_text<const N: usize>(directory: &Path, arguments: [&str; N]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| error.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}
