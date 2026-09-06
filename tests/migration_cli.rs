use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use renrs::migration::MigrationReport;

fn run_strict_migration(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_renrs-migrate"))
        .arg("--strict")
        .arg(input)
        .arg(output)
        .output()
        .expect("migration CLI should run")
}

fn read_report(output: &Path) -> MigrationReport {
    let bytes = fs::read(output.join("migration-report.json"))
        .expect("migration should write its report before exiting");
    serde_json::from_slice(&bytes).expect("migration report should be valid JSON")
}

#[test]
fn strict_migration_succeeds_for_a_clean_project() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("script.rpy"),
        "label start:\n    \"Hello\"\n    return\n",
    )
    .unwrap();

    let result = run_strict_migration(&input, &output);

    assert!(
        result.status.success(),
        "strict migration failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report = read_report(&output);
    assert!(!report.has_strict_failures());
}

#[test]
fn strict_migration_fails_for_unsupported_source_but_writes_the_report() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("script.rpy"),
        "label start:\n    python:\n        pass\n    return\n",
    )
    .unwrap();

    let result = run_strict_migration(&input, &output);

    assert!(!result.status.success());
    let report = read_report(&output);
    assert!(!report.issues.is_empty());
    assert!(report.has_strict_failures());
}

#[test]
fn strict_migration_fails_for_post_validation_diagnostics_and_writes_the_report() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("script.rpy"),
        "label start:\n    missing_character \"Hello\"\n    return\n",
    )
    .unwrap();

    let result = run_strict_migration(&input, &output);

    assert!(!result.status.success());
    let report = read_report(&output);
    assert!(report.issues.is_empty());
    assert!(!report.post_validation_diagnostics.is_empty());
    assert!(report.has_strict_failures());
}
