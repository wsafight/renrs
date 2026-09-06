use renrs::{ProjectSource, Runtime};
use serde_json::json;
use std::sync::Arc;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [project, options @ ..] = args.as_slice() else {
        return Err("usage: renrs-accept <project> [--saves directory]".into());
    };
    let mut saves = None;
    for pair in options.chunks(2) {
        match pair {
            [flag, value] if flag == "--saves" => saves = Some(value),
            _ => return Err("expected --saves <directory>".into()),
        }
    }
    let source = ProjectSource::open(project)?;
    let program = Arc::new(compile(&source)?);
    let mut checks = Vec::new();
    if source.contains("routes.json") {
        let suite: renrs::debugger::RouteSuite =
            serde_json::from_slice(&source.read("routes.json")?)?;
        if suite.routes.is_empty() {
            return Err("routes.json has no routes".into());
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
        checks.push(json!({"kind":"routes", "passed":false, "error":"routes.json is required for release acceptance"}));
    }
    if let Some(directory) = saves {
        check_saves(directory, &program, &mut checks)?;
    }
    let passed = checks.iter().all(|check| check["passed"] == true);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "project_id":program.project_id, "fingerprint":program.fingerprint,
            "platform":std::env::consts::OS, "passed":passed, "checks":checks,
            "external_acceptance":{"real_project":false,"remote_platforms":false,"publisher_signing":false}
        }))?
    );
    if !passed {
        return Err("release acceptance failed; inspect the JSON report".into());
    }
    Ok(())
}

fn compile(source: &ProjectSource) -> Result<renrs::Program, String> {
    source.compile().map_err(|errors| {
        errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn check_saves(
    directory: &str,
    program: &Arc<renrs::Program>,
    checks: &mut Vec<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = renrs::save::SaveRepository::new(directory);
    let entries = repository.list()?;
    if entries.is_empty() {
        return Err("save acceptance directory has no saves".into());
    }
    for slot in entries {
        let result = repository
            .load(&slot.name)
            .map_err(|error| error.to_string())
            .and_then(|save| {
                save.validate(&program.project_id)?;
                Runtime::restore(program.clone(), save.snapshot)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            });
        checks.push(match result {
            Ok(()) => json!({"kind":"save", "name":slot.name, "passed":true}),
            Err(error) => json!({"kind":"save", "name":slot.name, "passed":false, "error":error}),
        });
    }
    Ok(())
}
