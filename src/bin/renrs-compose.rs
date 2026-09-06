fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let [project, manifest, destination] = arguments.as_slice() else {
        return Err("usage: renrs-compose <project> <character.json> <images/variants>".into());
    };
    let count = renrs::composition::compose(
        std::path::Path::new(project),
        std::path::Path::new(manifest),
        destination.to_str().ok_or("invalid destination")?,
    )?;
    println!("Composed {count} character presets");
    Ok(())
}
