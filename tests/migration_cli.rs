use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use renrs::ProjectSource;
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

#[test]
fn official_the_question_sample_matches_the_migration_baseline() {
    let temporary = tempfile::tempdir().unwrap();
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("references/renpy/the_question/game");
    let output = temporary.path().join("output");
    let baseline: serde_json::Value = serde_json::from_slice(include_bytes!(
        "fixtures/the_question_migration_baseline.json"
    ))
    .unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_renrs-migrate"))
        .arg(&input)
        .arg(&output)
        .output()
        .expect("official sample migration should run");

    assert!(
        result.status.success(),
        "migration failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report = read_report(&output);
    let expected = |key: &str| usize::try_from(baseline[key].as_u64().unwrap()).unwrap();
    assert_eq!(report.converted_files, expected("converted_files"));
    assert_eq!(report.copied_resources, expected("copied_resources"));
    assert_eq!(report.generated_resources, expected("generated_resources"));
    assert_eq!(
        report.generated_support_files,
        expected("generated_support_files")
    );
    assert_eq!(report.issues.len(), expected("issues"));
    assert_eq!(report.summary.assumptions, expected("assumptions"));
    assert_eq!(report.summary.unsupported, expected("unsupported"));
    for code in baseline["required_codes"].as_array().unwrap() {
        assert!(report.summary.by_code.contains_key(code.as_str().unwrap()));
    }
    assert!(report.post_validation_diagnostics.is_empty());
    assert!(output.join("theme.json").is_file());
    assert!(output.join("screens.json").is_file());
    assert!(
        report
            .support_files
            .iter()
            .any(|file| file.kind == "options"
                && file.mapped_keys.contains(&"config.name".to_owned()))
    );
    ProjectSource::open(output).unwrap().compile().unwrap();
}
