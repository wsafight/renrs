use renrs::debugger::{ExploreLimits, Route, RouteSuite, explore, run_route};
use renrs::{Program, ProjectSource, Runtime, WaitState};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [command, project, rest @ ..] = args.as_slice() else {
        return Err("usage: renrs-debug <inspect|record|replay|test|explore> <project> [route.json|routes.json] [--max-runs N] [--max-steps N] [--max-depth N]".to_owned());
    };
    let program = ProjectSource::open(project)
        .map_err(|error| error.to_string())?
        .compile()
        .map_err(|errors| {
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    match (command.as_str(), rest) {
        ("inspect", []) => {
            let mut runtime = Runtime::new(program).map_err(|error| error.to_string())?;
            runtime.advance().map_err(|error| error.to_string())?;
            print_json(&runtime.debug_state())
        }
        ("record", [destination]) => record(program, Path::new(destination)),
        ("replay", [path]) => {
            let route: Route = read_json(path)?;
            print_json(&run_route(&program, &route, 10_000)?)
        }
        ("test", [path]) => {
            let suite: RouteSuite = read_json(path)?;
            if suite.routes.is_empty() {
                return Err("route suite is empty".to_owned());
            }
            let results = suite
                .routes
                .iter()
                .map(|route| run_route(&program, route, 10_000))
                .collect::<Result<Vec<_>, _>>()?;
            print_json(&results)
        }
        ("explore", options) => {
            let mut limits = ExploreLimits::default();
            for pair in options.chunks(2) {
                let [flag, value] = pair else {
                    return Err("missing exploration bound".to_owned());
                };
                let value = value
                    .parse()
                    .map_err(|_| "bounds must be positive integers")?;
                match flag.as_str() {
                    "--max-runs" => limits.max_runs = value,
                    "--max-steps" => limits.max_steps = value,
                    "--max-depth" => limits.max_depth = value,
                    _ => return Err(format!("unknown option {flag}")),
                }
            }
            let report = explore(&program, limits)?;
            print_json(&report)?;
            if report.complete {
                Ok(())
            } else {
                Err("exploration is incomplete; see failures and truncated_branches".to_owned())
            }
        }
        _ => Err("invalid arguments for debugger command".to_owned()),
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
                print_json(&runtime.debug_state())?;
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

fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())?
    );
    Ok(())
}
