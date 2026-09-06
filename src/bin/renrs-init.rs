use std::env;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let destination = PathBuf::from(
        args.next()
            .ok_or("usage: renrs-init <new-directory> [--title <title>] [--id <project-id>]")?,
    );
    let name = destination
        .file_name()
        .ok_or("destination must name a directory")?
        .to_string_lossy();
    let mut title = name.to_string();
    let mut template = "story".to_owned();
    let mut id = format!("org.renrs.{}", renrs::normalize_project_id(&name));
    while let Some(option) = args.next() {
        let value = args
            .next()
            .ok_or("missing option value")?
            .into_string()
            .map_err(|_| "option values must be UTF-8")?;
        match option.to_str() {
            Some("--title") => title = value,
            Some("--id") => id = value,
            Some("--template") => template = value,
            _ => return Err(format!("unknown option: {}", option.to_string_lossy())),
        }
    }
    renrs::scaffold::create_from_template(&destination, &title, &id, &template)?;
    println!("Created {} at {}", title, destination.display());
    Ok(())
}
