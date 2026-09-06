use renrs::{Diagnostic, Program, ProjectSource, analyze, compile};
use std::path::PathBuf;

fn main() {
    let mut json = false;
    let mut path = None;
    for argument in std::env::args_os().skip(1) {
        if argument == "--json" {
            json = true;
        } else if path.is_none() && !argument.to_string_lossy().starts_with('-') {
            path = Some(PathBuf::from(argument));
        } else {
            eprintln!("usage: renrs-check [--json] <project|archive>");
            std::process::exit(2);
        }
    }
    let (program, diagnostics) = inspect(path.unwrap_or_else(|| PathBuf::from("demo")));
    let ok = !diagnostics.iter().any(Diagnostic::is_error);
    if json {
        println!(
            "{}",
            serde_json::json!({"ok": ok, "diagnostics": diagnostics,
            "labels": program.as_ref().map(|program| program.labels.len()),
            "instructions": program.as_ref().map(|program| program.instructions.len()),
            "fingerprint": program.as_ref().map(|program| &program.fingerprint)})
        );
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
    if !ok {
        std::process::exit(1);
    }
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
