use renrs::protocol::{self, MachineEnvelope, MachineError};
use renrs::{Diagnostic, Program, ProjectSource, analyze, compile};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let json = arguments.iter().any(|argument| argument == "--json");
    let path = match parse_arguments(&arguments) {
        Ok(path) => path,
        Err(message) => {
            if json {
                let _ = protocol::print(&MachineEnvelope::<serde_json::Value>::failure(
                    "check",
                    protocol::code::INVALID_ARGUMENTS,
                    message,
                ));
            } else {
                eprintln!("{message}");
            }
            return ExitCode::from(protocol::EXIT_USAGE);
        }
    };
    let (program, diagnostics) = inspect(path);
    let ok = !diagnostics.iter().any(Diagnostic::is_error);
    if json {
        let data = serde_json::json!({
            "labels": program.as_ref().map(|program| program.labels.len()),
            "instructions": program.as_ref().map(|program| program.instructions.len()),
            "fingerprint": program.as_ref().map(|program| &program.fingerprint),
        });
        let error = (!ok)
            .then(|| MachineError::new(protocol::code::VALIDATION_FAILED, "project check failed"));
        if let Err(error) = protocol::print(&MachineEnvelope::completed(
            "check",
            ok,
            Some(data),
            diagnostics,
            error,
        )) {
            eprintln!("error: {error}");
            return ExitCode::from(protocol::EXIT_FAILURE);
        }
    } else {
        for diagnostic in diagnostics {
            eprintln!("{diagnostic}");
        }
        if let Some(program) = program {
            println!(
                "OK: {} labels, {} instructions, script {}",
                program.labels.len(),
                program.instructions.len(),
                &program.fingerprint[..12]
            );
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(protocol::EXIT_FAILURE)
    }
}

fn parse_arguments(arguments: &[std::ffi::OsString]) -> Result<PathBuf, &'static str> {
    let mut path = None;
    for argument in arguments {
        if argument == "--json" {
            continue;
        }
        if path.is_none() && !argument.to_string_lossy().starts_with('-') {
            path = Some(PathBuf::from(argument));
        } else {
            return Err("usage: renrs-check [--json] <project|archive>");
        }
    }
    Ok(path.unwrap_or_else(|| PathBuf::from("demo")))
}

fn inspect(path: PathBuf) -> (Option<Program>, Vec<Diagnostic>) {
    let source = match ProjectSource::open(path) {
        Ok(source) => source,
        Err(error) => return (None, vec![Diagnostic::new(".", 1, 1, error.to_string())]),
    };
    let script = match source.load_script() {
        Ok(script) => script,
        Err(errors) => return (None, errors),
    };
    let mut diagnostics = source.validate(&script);
    diagnostics.extend(source.validate_support_files());
    if diagnostics.iter().any(Diagnostic::is_error) {
        return (None, diagnostics);
    }
    match compile(&script) {
        Ok(program) => {
            diagnostics.extend(analyze(&program));
            (Some(program), diagnostics)
        }
        Err(error) => (None, vec![Diagnostic::new(".", 1, 1, error.to_string())]),
    }
}
