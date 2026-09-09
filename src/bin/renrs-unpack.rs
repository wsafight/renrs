use std::env;
use std::path::PathBuf;

use renrs::archive::ResourceArchive;
#[cfg(test)]
use renrs::archive::pack_project;

fn main() {
    if let Err(error) = run_from(env::args_os().skip(1).map(PathBuf::from).collect()) {
        fail(error);
    }
}

fn run_from(arguments: Vec<PathBuf>) -> Result<(), String> {
    let mut arguments = arguments.into_iter();
    let archive_path = arguments
        .next()
        .ok_or("usage: renrs-unpack <archive.renrs> <output-directory>")?;
    let output = arguments
        .next()
        .ok_or("usage: renrs-unpack <archive.renrs> <output-directory>")?;
    if arguments.next().is_some() {
        return Err("usage: renrs-unpack <archive.renrs> <output-directory>".to_owned());
    }
    let archive = ResourceArchive::open(&archive_path).map_err(|error| error.to_string())?;
    archive
        .extract(&output)
        .map_err(|error| error.to_string())?;
    println!("unpacked {} entries", archive.entries().len());
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
    fn round_trips_an_archive() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        fs::create_dir(&game).unwrap();
        fs::write(game.join("script.rns"), "label start:\n    return\n").unwrap();
        let archive = root.path().join("game.renrs");
        pack_project(&game, &archive).unwrap();
        let output = root.path().join("out");
        run_from(vec![archive, output.clone()]).unwrap();
        assert!(output.join("script.rns").is_file());
        assert!(run_from(Vec::new()).is_err());
    }
}
