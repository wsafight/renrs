use std::env;
use std::path::PathBuf;

use renrs::archive::{ResourceArchive, pack_project};

const USAGE: &str = "usage: renrs-pack <game-directory|archive.renrs> <output.renrs>";

fn main() {
    if let Err(error) = run_from(env::args_os().skip(1).map(PathBuf::from).collect()) {
        fail(error);
    }
}

fn run_from(arguments: Vec<PathBuf>) -> Result<(), String> {
    let mut arguments = arguments.into_iter();
    let input = arguments.next().ok_or(USAGE)?;
    let output = arguments.next().ok_or(USAGE)?;
    if arguments.next().is_some() {
        return Err(USAGE.to_owned());
    }
    if input.is_dir() {
        pack_project(&input, &output).map_err(|error| error.to_string())?;
        println!("packed {}", output.display());
    } else {
        let archive = ResourceArchive::open(&input).map_err(|error| error.to_string())?;
        let entries = archive.repack(&output).map_err(|error| error.to_string())?;
        println!("repacked {entries} entries into {}", output.display());
    }
    Ok(())
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("error: {}", message.as_ref());
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn packs_a_minimal_project() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        fs::create_dir(&game).unwrap();
        fs::write(game.join("script.rns"), "label start:\n    return\n").unwrap();
        let archive = root.path().join("game.renrs");
        run_from(vec![game, archive.clone()]).unwrap();
        assert!(archive.is_file());
        assert!(run_from(Vec::new()).is_err());
    }

    #[test]
    fn repacks_an_existing_archive() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        fs::create_dir(&game).unwrap();
        fs::write(game.join("script.rns"), "label start:\n    return\n").unwrap();
        let source = root.path().join("game.renrs");
        pack_project(&game, &source).unwrap();
        let destination = root.path().join("normalized.renrs");
        run_from(vec![source, destination.clone()]).unwrap();
        let repacked = ResourceArchive::open(&destination).unwrap();
        assert!(repacked.contains("script.rns"));
    }
}
