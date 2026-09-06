use std::env;
use std::path::PathBuf;

use renrs::build_distribution;

fn main() {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    let (project, destination, player) = match arguments.as_slice() {
        [project, destination] => (
            PathBuf::from(project),
            PathBuf::from(destination),
            sibling_player().unwrap_or_else(|error| fail(error)),
        ),
        [project, destination, option, player] if option == "--player" => (
            PathBuf::from(project),
            PathBuf::from(destination),
            PathBuf::from(player),
        ),
        _ => fail("usage: renrs-build <project|archive> <output-directory> [--player <path>]"),
    };
    match build_distribution(&project, &destination, &player) {
        Ok(manifest) => println!(
            "Built {} ({}) at {}",
            manifest.title,
            manifest.project_id,
            destination.display()
        ),
        Err(error) => fail(error.to_string()),
    }
}

fn sibling_player() -> Result<PathBuf, String> {
    let current = env::current_exe().map_err(|error| error.to_string())?;
    let directory = current
        .parent()
        .ok_or_else(|| "could not locate the current executable directory".to_owned())?;
    let name = if cfg!(windows) { "renrs.exe" } else { "renrs" };
    let player = directory.join(name);
    if player.is_file() {
        Ok(player)
    } else {
        Err(format!(
            "player executable `{}` is missing; build the `renrs` binary first or pass --player",
            player.display()
        ))
    }
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("error: {}", message.as_ref());
    std::process::exit(1);
}
