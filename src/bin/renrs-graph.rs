use std::env;
use std::fs;
use std::path::PathBuf;

use renrs::tooling::story_graph;
use renrs::{load_project, validate};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    let game_root = arguments
        .next()
        .map_or_else(|| PathBuf::from("demo"), PathBuf::from);
    let output = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        return Err("usage: renrs-graph [game-directory] [output.dot]".to_owned());
    }
    let script = load_project(&game_root).map_err(join_diagnostics)?;
    let diagnostics = validate(&script, &game_root);
    if !diagnostics.is_empty() {
        return Err(join_diagnostics(diagnostics));
    }
    let graph = story_graph(&script);
    if let Some(path) = output {
        fs::write(&path, graph)
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        println!("wrote {}", path.display());
    } else {
        print!("{graph}");
    }
    Ok(())
}

fn join_diagnostics(diagnostics: Vec<renrs::Diagnostic>) -> String {
    diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
