use std::fs;
use std::path::Path;

const MAX_RUST_LINES: usize = 500;

#[test]
fn rust_source_files_stay_below_the_cohesion_limit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut oversized = Vec::new();
    let mut packages = vec![root.to_owned()];
    packages.extend(
        fs::read_dir(root.join("crates"))
            .expect("could not inspect workspace crates")
            .map(|entry| entry.expect("crate directory entry").path())
            .filter(|path| path.is_dir()),
    );
    for package in packages {
        for scope in ["src", "tests", "benches", "examples"] {
            let directory = package.join(scope);
            if directory.is_dir() {
                inspect(&directory, &mut oversized);
            }
        }
    }
    assert!(
        oversized.is_empty(),
        "split Rust files over {MAX_RUST_LINES} lines by responsibility:\n{}",
        oversized.join("\n")
    );
}

fn inspect(directory: &Path, oversized: &mut Vec<String>) {
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("could not inspect {}: {error}", directory.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            inspect(&path, oversized);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            inspect_file(&path, oversized);
        }
    }
}

fn inspect_file(path: &Path, oversized: &mut Vec<String>) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
    let lines = source.lines().count();
    if lines > MAX_RUST_LINES {
        oversized.push(format!("{}: {lines}", path.display()));
    }
}
