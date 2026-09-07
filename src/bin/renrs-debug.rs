use renrs::debugger::{ExploreLimits, Route, RouteSuite, explore, run_route};
use renrs::protocol::{self, MachineEnvelope, MachineError};
use renrs::{Diagnostic, Program, ProjectSource, Runtime, WaitState};
use serde::Serialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

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
    let args = env::args().skip(1).collect::<Vec<_>>();
    let interactive = args.first().is_some_and(|command| command == "record");
    let command = args
        .first()
        .map_or_else(|| "debug".to_owned(), |value| format!("debug.{value}"));
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(protocol::EXIT_FAILURE),
        Err(error) => {
            let exit = if error.usage {
                protocol::EXIT_USAGE
            } else {
                protocol::EXIT_FAILURE
            };
            if interactive {
                eprintln!("error: {}", error.message);
            } else {
                let envelope = MachineEnvelope::<serde_json::Value>::completed(
                    command,
                    false,
                    None,
                    error.diagnostics,
                    Some(MachineError::new(error.code, error.message)),
                );
                if let Err(output_error) = protocol::print(&envelope) {
                    eprintln!("error: {output_error}");
                }
            }
            ExitCode::from(exit)
        }
    }
}

fn run(args: &[String]) -> Result<bool, CliError> {
    let [command, project, rest @ ..] = args else {
        return Err(CliError::usage(
            "usage: renrs-debug <inspect|record|replay|test|explore> <project> [route.json|routes.json] [--max-runs N] [--max-steps N] [--max-depth N]",
        ));
    };
    let program = ProjectSource::open(project)
        .map_err(|error| CliError::new(protocol::code::PROJECT_INVALID, error.to_string()))?
        .compile()
        .map_err(CliError::project)?;
    let machine_command = format!("debug.{command}");
    match (command.as_str(), rest) {
        ("inspect", []) => {
            let mut runtime = Runtime::new(program).map_err(|error| {
                CliError::new(protocol::code::RUNTIME_FAILED, error.to_string())
            })?;
            runtime.advance().map_err(|error| {
                CliError::new(protocol::code::RUNTIME_FAILED, error.to_string())
            })?;
            print_success(&machine_command, runtime.debug_state())?;
            Ok(true)
        }
        ("record", [destination]) => record(program, Path::new(destination))
            .map(|()| true)
            .map_err(|error| CliError::new(protocol::code::RUNTIME_FAILED, error)),
        ("replay", [path]) => {
            let route: Route = read_json(path)
                .map_err(|error| CliError::new(protocol::code::INVALID_ARGUMENTS, error))?;
            let result = run_route(&program, &route, 10_000)
                .map_err(|error| CliError::new(protocol::code::ROUTE_FAILED, error))?;
            print_success(&machine_command, result)?;
            Ok(true)
        }
        ("test", [path]) => {
            let suite: RouteSuite = read_json(path)
                .map_err(|error| CliError::new(protocol::code::INVALID_ARGUMENTS, error))?;
            if suite.routes.is_empty() {
                return Err(CliError::new(
                    protocol::code::ROUTE_FAILED,
                    "route suite is empty",
                ));
            }
            let results = suite
                .routes
                .iter()
                .map(|route| run_route(&program, route, 10_000))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| CliError::new(protocol::code::ROUTE_FAILED, error))?;
            print_success(&machine_command, results)?;
            Ok(true)
        }
        ("explore", options) => {
            let mut limits = ExploreLimits::default();
            for pair in options.chunks(2) {
                let [flag, value] = pair else {
                    return Err(CliError::usage("missing exploration bound"));
                };
                let value = value
                    .parse()
                    .map_err(|_| CliError::usage("bounds must be positive integers"))?;
                match flag.as_str() {
                    "--max-runs" => limits.max_runs = value,
                    "--max-steps" => limits.max_steps = value,
                    "--max-depth" => limits.max_depth = value,
                    _ => return Err(CliError::usage(format!("unknown option {flag}"))),
                }
            }
            let report = explore(&program, limits)
                .map_err(|error| CliError::new(protocol::code::RUNTIME_FAILED, error))?;
            if report.complete {
                print_success(&machine_command, report)?;
                Ok(true)
            } else {
                let envelope = MachineEnvelope::completed(
                    machine_command,
                    false,
                    Some(report),
                    Vec::new(),
                    Some(MachineError::new(
                        protocol::code::EXPLORATION_INCOMPLETE,
                        "exploration is incomplete; see failures and truncated_branches",
                    )),
                );
                protocol::print(&envelope)
                    .map_err(|error| CliError::new(protocol::code::RUNTIME_FAILED, error))?;
                Ok(false)
            }
        }
        _ => Err(CliError::usage("invalid arguments for debugger command")),
    }
}

fn record(program: Program, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err("route destination already exists".to_owned());
    }
    let mut route = Route {
        name: "recorded".to_owned(),
        project_id: Some(program.project_id.clone()),
        fingerprint: Some(program.fingerprint.clone()),
        ..Route::default()
    };
    let mut runtime = Runtime::new(program).map_err(|error| error.to_string())?;
    let mut state = runtime.advance().map_err(|error| error.to_string())?;
    for _ in 0..10_000 {
        if state == WaitState::Finished {
            route.expect_label = runtime.current_label().map(ToOwned::to_owned);
            route.expect_variables = runtime.variables().clone();
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(|error| error.to_string())?;
            serde_json::to_writer_pretty(file, &route).map_err(|error| error.to_string())?;
            return Ok(());
        }
        match &state {
            WaitState::Dialogue => println!(
                "{}",
                runtime
                    .stage()
                    .dialogue
                    .as_ref()
                    .map_or("", |dialogue| dialogue.text.as_str())
            ),
            WaitState::Choice { options } => {
                for (index, text) in options.iter().enumerate() {
                    println!("{}: {text}", index + 1);
                }
            }
            _ => {}
        }
        print!("next / choice number / state / quit > ");
        io::stdout().flush().map_err(|error| error.to_string())?;
        let mut input = String::new();
        if io::stdin()
            .read_line(&mut input)
            .map_err(|error| error.to_string())?
            == 0
        {
            return Err("recording ended before the story finished".to_owned());
        }
        match input.trim() {
            "quit" => return Err("recording cancelled".to_owned()),
            "state" => {
                print_interactive_json(&runtime.debug_state())?;
                continue;
            }
            "next" | "" if !matches!(state, WaitState::Choice { .. }) => {
                state = runtime
                    .continue_story()
                    .map_err(|error| error.to_string())?;
            }
            number if matches!(state, WaitState::Choice { .. }) => {
                let Some(index) = number
                    .parse::<usize>()
                    .ok()
                    .and_then(|number| number.checked_sub(1))
                else {
                    continue;
                };
                let ids = runtime.choice_ids().map_err(|error| error.to_string())?;
                let Some(id) = ids.get(index) else {
                    continue;
                };
                route.choice_ids.push(id.clone());
                route.choices.push(index);
                state = runtime.choose(index).map_err(|error| error.to_string())?;
            }
            _ => {}
        }
        runtime.drain_audio_events().for_each(drop);
    }
    Err("recording exceeded 10000 interactions".to_owned())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn print_success(command: &str, value: impl Serialize) -> Result<(), CliError> {
    protocol::print(&MachineEnvelope::success(command, value))
        .map_err(|error| CliError::new(protocol::code::RUNTIME_FAILED, error))
}

fn print_interactive_json(value: &impl Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}
