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
    run_from(env::args_os().skip(1).map(PathBuf::from).collect())
}

fn run_from(arguments: Vec<PathBuf>) -> Result<(), String> {
    let mut arguments = arguments.into_iter();
    let game_root = arguments.next().unwrap_or_else(|| PathBuf::from("demo"));
    let output = arguments.next();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn writes_a_story_graph_for_a_minimal_project() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("script.rns"), "label start:\n    return\n").unwrap();
        let output = root.path().join("story.dot");
        run_from(vec![root.path().to_path_buf(), output.clone()]).unwrap();
        let graph = fs::read_to_string(output).unwrap();
        assert!(graph.contains("start"));
        assert!(
            run_from(vec![
                root.path().to_path_buf(),
                root.path().join("a.dot"),
                root.path().join("b.dot")
            ])
            .is_err()
        );
        assert!(
            join_diagnostics(vec![renrs::Diagnostic::new("a.rns", 1, 1, "x")]).contains("a.rns")
        );
    }
}
