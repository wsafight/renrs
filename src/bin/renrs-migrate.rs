use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use renrs::migration::migrate_project;

const USAGE: &str = "usage: renrs-migrate [--strict] <renpy-game-or-script.rpy> <output-directory>";

fn main() {
    let options = parse_arguments(env::args_os().skip(1)).unwrap_or_else(|error| fail(error));
    let input = options.input;
    let output = options.output;
    let report = migrate_project(&input, &output).unwrap_or_else(|error| fail(error.to_string()));
    println!(
        "converted {} script(s), copied {} resource(s), generated {} resource(s) and {} support file(s)",
        report.converted_files,
        report.copied_resources,
        report.generated_resources,
        report.generated_support_files,
    );
    println!(
        "reported {} issue(s): {} assumption(s), {} unsupported; {} post-validation diagnostic(s)",
        report.issues.len(),
        report.summary.assumptions,
        report.summary.unsupported,
        report.post_validation_diagnostics.len()
    );
    println!("report: {}", output.join("migration-report.json").display());
    if options.strict && report.has_strict_failures() {
        fail("strict migration failed; review migration-report.json");
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    input: PathBuf,
    output: PathBuf,
    strict: bool,
}

fn parse_arguments(arguments: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut strict = false;
    let mut positional = Vec::new();
    let mut parse_options = true;
    for argument in arguments {
        if parse_options && argument == "--" {
            parse_options = false;
        } else if parse_options && argument == "--strict" {
            if strict {
                return Err(format!("`--strict` was supplied more than once\n{USAGE}"));
            }
            strict = true;
        } else if parse_options && argument.to_string_lossy().starts_with("--") {
            return Err(format!(
                "unknown option `{}`\n{USAGE}",
                argument.to_string_lossy()
            ));
        } else {
            positional.push(PathBuf::from(argument));
        }
    }
    let [input, output] = positional.as_slice() else {
        return Err(USAGE.to_owned());
    };
    Ok(Options {
        input: input.clone(),
        output: output.clone(),
        strict,
    })
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("error: {}", message.as_ref());
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_strict_before_or_after_paths() {
        let before = parse_arguments(["--strict", "input", "output"].map(OsString::from)).unwrap();
        let after = parse_arguments(["input", "output", "--strict"].map(OsString::from)).unwrap();
        assert!(before.strict);
        assert_eq!(before, after);
    }

    #[test]
    fn rejects_unknown_options() {
        let error =
            parse_arguments(["--guess", "input", "output"].map(OsString::from)).unwrap_err();
        assert!(error.contains("unknown option"));
    }
}
