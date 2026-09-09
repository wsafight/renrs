fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    run_from(std::env::args_os().skip(1).collect())
}

fn run_from(arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn std::error::Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_argument_counts() {
        assert!(run_from(Vec::new()).is_err());
        assert!(run_from(vec!["a".into()]).is_err());
    }
}
