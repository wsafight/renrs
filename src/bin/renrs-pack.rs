use std::env;
use std::path::PathBuf;

use renrs::archive::pack_project;

fn main() {
    if let Err(error) = run_from(env::args_os().skip(1).map(PathBuf::from).collect()) {
        fail(error);
    }
}

fn run_from(arguments: Vec<PathBuf>) -> Result<(), String> {
    let mut arguments = arguments.into_iter();
    let root = arguments
        .next()
        .ok_or("usage: renrs-pack <game-directory> <output.renrs>")?;
    let output = arguments
        .next()
        .ok_or("usage: renrs-pack <game-directory> <output.renrs>")?;
    if arguments.next().is_some() {
        return Err("usage: renrs-pack <game-directory> <output.renrs>".to_owned());
    }
    pack_project(&root, &output).map_err(|error| error.to_string())?;
    println!("packed {}", output.display());
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
}
