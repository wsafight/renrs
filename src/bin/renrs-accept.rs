use renrs::protocol::{self, MachineEnvelope, MachineError};
use renrs::{Diagnostic, ProjectSource, Runtime};
use serde::Serialize;
use serde_json::json;
use std::process::ExitCode;
use std::sync::Arc;

#[derive(Serialize)]
struct AcceptanceReport {
    project_id: String,
    fingerprint: String,
    platform: &'static str,
    passed: bool,
    checks: Vec<serde_json::Value>,
    external_acceptance: serde_json::Value,
}

struct CliError {
    code: &'static str,
    message: String,
    diagnostics: Vec<Diagnostic>,
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

    fn project(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            code: protocol::code::PROJECT_INVALID,
            message: "project compilation failed".to_owned(),
            diagnostics,
            usage: false,
        }
    }

    fn usage(message: impl Into<String>) -> Self {
        Self {
            usage: true,
            ..Self::new(protocol::code::INVALID_ARGUMENTS, message)
        }
    }
}

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match run(&arguments) {
        Ok(report) => {
            let passed = report.passed;
            let error = (!passed).then(|| {
                MachineError::new(
                    protocol::code::ACCEPTANCE_FAILED,
                    "release acceptance failed; inspect the report",
                )
            });
            let envelope =
                MachineEnvelope::completed("accept", passed, Some(report), Vec::new(), error);
            if let Err(error) = protocol::print(&envelope) {
                eprintln!("error: {error}");
                return ExitCode::from(protocol::EXIT_FAILURE);
            }
            if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(protocol::EXIT_FAILURE)
            }
        }
        Err(error) => {
            let exit = if error.usage {
                protocol::EXIT_USAGE
            } else {
                protocol::EXIT_FAILURE
            };
            let envelope = MachineEnvelope::<serde_json::Value>::completed(
                "accept",
                false,
                None,
                error.diagnostics,
                Some(MachineError::new(error.code, error.message)),
            );
            if let Err(output_error) = protocol::print(&envelope) {
                eprintln!("error: {output_error}");
            }
            ExitCode::from(exit)
        }
    }
}

fn run(args: &[String]) -> Result<AcceptanceReport, CliError> {
    let [project, options @ ..] = args else {
        return Err(CliError::usage(
            "usage: renrs-accept <project> [--saves directory]",
        ));
    };
    let mut saves = None;
    for pair in options.chunks(2) {
        match pair {
            [flag, value] if flag == "--saves" => saves = Some(value),
            _ => return Err(CliError::usage("expected --saves <directory>")),
        }
    }
    let source = ProjectSource::open(project)
        .map_err(|error| CliError::new(protocol::code::PROJECT_INVALID, error.to_string()))?;
    let program = Arc::new(source.compile().map_err(CliError::project)?);
    let mut checks = Vec::new();
    if source.contains("routes.json") {
        let suite: renrs::debugger::RouteSuite = serde_json::from_slice(
            &source
                .read("routes.json")
                .map_err(|error| CliError::new(protocol::code::ROUTE_FAILED, error.to_string()))?,
        )
        .map_err(|error| CliError::new(protocol::code::ROUTE_FAILED, error.to_string()))?;
        if suite.routes.is_empty() {
            return Err(CliError::new(
                protocol::code::ROUTE_FAILED,
                "routes.json has no routes",
            ));
        }
        for route in suite.routes {
            match renrs::debugger::run_route(&program, &route, 10_000) {
                Ok(result) => checks.push(
                    json!({"kind":"route", "name":route.name, "passed":true, "result":result}),
                ),
                Err(error) => checks.push(
                    json!({"kind":"route", "name":route.name, "passed":false, "error":error}),
                ),
            }
        }
    } else {
        checks.push(json!({"kind":"routes", "passed":false,
            "error":"routes.json is required for release acceptance"}));
    }
    if let Some(directory) = saves {
        check_saves(directory, &program, &mut checks)
            .map_err(|error| CliError::new(protocol::code::ACCEPTANCE_FAILED, error))?;
    }
    let passed = checks.iter().all(|check| check["passed"] == true);
    Ok(AcceptanceReport {
        project_id: program.project_id.clone(),
        fingerprint: program.fingerprint.clone(),
        platform: std::env::consts::OS,
        passed,
        checks,
        external_acceptance: json!({
            "real_project": false,
            "target_platforms": false,
            "publisher_signing": false,
        }),
    })
}

fn check_saves(
    directory: &str,
    program: &Arc<renrs::Program>,
    checks: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    let repository = renrs::save::SaveRepository::new(directory);
    let entries = repository.list().map_err(|error| error.to_string())?;
    if entries.is_empty() {
        return Err("save acceptance directory has no saves".into());
    }
    for slot in entries {
        let result = repository
            .load(&slot.name)
            .map_err(|error| error.to_string())
            .and_then(|save| {
                save.validate(&program.project_id)?;
                Runtime::restore_compatible(program.clone(), save.snapshot)
                    .map(|(_, report)| report)
                    .map_err(|error| error.to_string())
            });
        checks.push(match result {
            Ok(report) => {
                json!({"kind":"save", "name":slot.name, "passed":true, "compatibility":report})
            }
            Err(error) => json!({"kind":"save", "name":slot.name, "passed":false, "error":error}),
        });
    }
    Ok(())
}
