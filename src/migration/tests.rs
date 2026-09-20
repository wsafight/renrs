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
        concat!(
            "\u{feff}",
            r##"define config.name = "Migrated Story"
define e = Character("Eileen", color="#ef6a6a")
label start:
    scene bg room
    $ score = 1
    e "Hello"
    menu:
        e "Choose a path."
        "Continue":
            jump ending
        "Wait":
            e "Waiting"
            jump ending
label ending:
    python:
        print("not executed")
    return
"##
        ),
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
    assert!(
        report
            .issues
            .iter()
            .all(|issue| !issue.message.contains("fallthrough"))
    );
    let script = load_project(&output).unwrap();
    assert!(validate(&script, &output).is_empty());
    assert_eq!(script.title, "Migrated Story");
    let migrated = fs::read_to_string(output.join("script.rns")).unwrap();
    assert!(migrated.contains("menu e \"Choose a path.\":"));
    assert!(!migrated.contains("TODO migration: define config.name"));
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
fn generates_the_builtin_black_scene_resource() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("script.rpy"),
        "label start:\n    scene black\n    return\n",
    )
    .unwrap();

    let report = migrate_project(&input, &output).unwrap();
    assert_eq!(report.generated_resources, 1);
    assert!(report.issues.is_empty(), "{:?}", report.issues);
    assert!(report.post_validation_diagnostics.is_empty());
    let image = image::open(output.join("images/black.png"))
        .unwrap()
        .into_rgba8();
    assert_eq!(image.dimensions(), (1, 1));
    assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 255]);
    let script = load_project(&output).unwrap();
    compile(&script).unwrap();
}

#[test]
fn migrates_static_composite_expression_to_a_layered_image_resource() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    let output = temporary.path().join("output");
    fs::create_dir_all(input.join("images")).unwrap();
    fs::write(input.join("images/bg.png"), []).unwrap();
    fs::write(input.join("images/hero.png"), []).unwrap();
    fs::write(
        input.join("script.rpy"),
        r#"label start:
    show expression Composite((640, 360), (0, 0), "images/bg.png", (40, 20), "images/hero.png") as hero at left
    return
"#,
    )
    .unwrap();

    let report = migrate_project(&input, &output).unwrap();
    assert!(report.issues.is_empty(), "{:?}", report.issues);
    assert!(report.post_validation_diagnostics.is_empty());
    assert_eq!(report.generated_resources, 1);
    let migrated = fs::read_to_string(output.join("script.rns")).unwrap();
    let generated = migrated
        .lines()
        .find_map(|line| line.strip_prefix("    show "))
        .and_then(|line| line.split('"').nth(1))
        .expect("Composite should become a generated layered image show");
    let definition: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join(generated)).unwrap()).unwrap();
    assert_eq!(definition["width"], 640);
    assert_eq!(definition["layers"].as_array().unwrap().len(), 2);
    let script = load_project(&output).unwrap();
    compile(&script).unwrap();
}

#[test]
fn diagnoses_composite_expression_boundaries_without_guessing() {
    let converted = convert_script(
        r#"label start:
    scene expression Composite((640, 360), (0, 0), "images/bg.png")
    show expression Composite((640, 360), (0, 0), image_path) as hero
    return
"#,
        "script.rpy",
        &AssetCatalog::with_image("bg", "images/bg.png"),
    );
    assert!(converted.issues.iter().any(|issue| {
        issue.code == "image_expression_composite_unsupported"
            && issue.message.contains("scene Composite")
    }));
    assert!(converted.issues.iter().any(|issue| {
        issue.code == "image_expression_composite_unsupported"
            && issue.message.contains("quoted static image paths")
    }));
    assert!(
        converted
            .output
            .contains("# TODO migration: scene expression Composite")
    );
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
