use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use renrs::tooling::format_source;

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
    let mut check = false;
    let mut inputs = Vec::new();
    for argument in arguments {
        if argument == Path::new("--check") {
            check = true;
        } else {
            inputs.push(argument);
        }
    }
    if inputs.is_empty() {
        return Err("usage: renrs-fmt [--check] <file-or-directory>...".to_owned());
    }
    let mut files = Vec::new();
    for input in inputs {
        collect_scripts(&input, &mut files).map_err(|error| error.to_string())?;
    }
    files.sort();
    files.dedup();
    let mut changed = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let formatted = format_source(&source);
        if formatted == source {
            continue;
        }
        if check {
            changed.push(path);
        } else {
            fs::write(&path, formatted)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
            println!("formatted {}", path.display());
        }
    }
    if changed.is_empty() {
        Ok(())
    } else {
        for path in &changed {
            eprintln!("needs formatting: {}", path.display());
        }
        Err(format!("{} file(s) need formatting", changed.len()))
    }
}

fn collect_scripts(path: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if path.is_file() {
        if path.extension().and_then(|value| value.to_str()) == Some("rns") {
            output.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !entry.file_name().to_string_lossy().starts_with('.') {
                collect_scripts(&entry.path(), output)?;
            }
        } else if file_type.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("rns")
        {
            output.push(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_checks_scripts() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("script.rns");
        fs::write(&path, "label start:   \n    \"Hi\"\n").unwrap();
        run_from(vec![root.path().to_path_buf()]).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "label start:\n    \"Hi\"\n"
        );
        fs::write(&path, "label start:   \n    \"Hi\"\n").unwrap();
        assert!(run_from(vec![PathBuf::from("--check"), path.clone()]).is_err());
        assert!(run_from(Vec::new()).is_err());
    }
}
