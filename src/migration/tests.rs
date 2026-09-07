use super::*;
use crate::{load_project, validate};

#[test]
fn migrates_supported_story_and_reports_python() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("renpy-game");
    let output = temporary.path().join("renrs-game");
    fs::create_dir_all(input.join("images")).unwrap();
    fs::write(input.join("images/bg_room.jpg"), b"image").unwrap();
    fs::write(
        input.join("script.rpy"),
        r##"define config.name = "Migrated Story"
define e = Character("Eileen", color="#ef6a6a")
label start:
    scene bg room
    $ score = 1
    e "Hello"
    menu:
        "Continue":
            jump ending
        "Wait":
            e "Waiting"
            jump ending
label ending:
    python:
        print("not executed")
    return
"##,
    )
    .unwrap();

    let report = migrate_project(&input, &output).unwrap();
    assert_eq!(report.converted_files, 1);
    assert_eq!(report.copied_resources, 1);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == MigrationIssueKind::Unsupported)
    );
    let script = load_project(&output).unwrap();
    assert!(validate(&script, &output).is_empty());
    assert_eq!(script.title, "Migrated Story");
}

#[test]
fn converts_safe_defaults_without_turning_them_into_assignments() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("script.rpy"),
        "default score = 1\ndefault enabled = True\nlabel start:\n    \"Score [score]\"\n    return\n",
    )
    .unwrap();

    let report = migrate_project(&input, &output).unwrap();
    let migrated = fs::read_to_string(output.join("script.rns")).unwrap();
    assert!(migrated.contains("default score = 1"));
    assert!(migrated.contains("default enabled = true"));
    assert!(!migrated.contains("set score = 1"));
    assert!(!report.has_strict_failures());
}

#[test]
fn converts_static_parameterized_calls() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("script.rpy"),
        "default base = 2\nlabel start:\n    call add(base, amount=3)\n    return\nlabel add(current, amount=1):\n    return current + amount\n",
    )
    .unwrap();

    let report = migrate_project(&input, &output).unwrap();
    assert!(report.post_validation_diagnostics.is_empty());
    assert!(report.issues.is_empty(), "{:?}", report.issues);
    let migrated = fs::read_to_string(output.join("script.rns")).unwrap();
    assert!(migrated.contains("call add(base, amount=3)"));
    assert!(migrated.contains("label add(current, amount=1):"));
    let script = load_project(&output).unwrap();
    compile(&script).unwrap();
}

#[test]
fn records_post_validation_diagnostics_in_the_report_file() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("script.rpy"),
        "label start:\n    missing_character \"Hello\"\n    return\n",
    )
    .unwrap();

    let report = migrate_project(&input, &output).unwrap();
    assert!(report.post_validation_diagnostics.iter().any(|item| {
        item.severity == PostValidationSeverity::Error && item.message.contains("unknown character")
    }));
    let saved: MigrationReport =
        serde_json::from_slice(&fs::read(output.join("migration-report.json")).unwrap()).unwrap();
    assert_eq!(
        saved.post_validation_diagnostics,
        report.post_validation_diagnostics
    );
    assert!(saved.has_strict_failures());
}
