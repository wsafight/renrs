use std::env;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, directory, chapters, lines] if command == "generate" => {
            renrs::benchmark::generate(Path::new(directory), chapters.parse().map_err(|_| "invalid chapters")?, lines.parse().map_err(|_| "invalid line count")?)?;
            println!("Generated benchmark project at {directory}");
            Ok(())
        }
        [project] | [project, _] => {
            let iterations = args.get(1).map_or(Ok(5), |value| value.parse::<usize>()).map_err(|_| "invalid iteration count")?;
            let report = renrs::benchmark::measure(Path::new(project), iterations)?;
            println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
            Ok(())
        }
        _ => Err("usage: renrs-bench <project> [iterations] | renrs-bench generate <new-directory> <chapters> <lines-per-chapter>".to_owned()),
    }
}
