use super::*;

#[test]
fn loads_sorted_scripts_as_one_project() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("story")).unwrap();
    fs::write(
        root.path().join("script.rns"),
        "config title \"Many Files\"\ndefine e = character \"Eileen\"\nlabel start:\n    call story_intro\n    return\n",
    )
    .unwrap();
    fs::write(
        root.path().join("story/intro.rns"),
        "label story_intro:\n    e \"Hello\"\n    return\n",
    )
    .unwrap();

    let script = load_project(root.path()).unwrap();
    assert_eq!(script.title, "Many Files");
    assert_eq!(script.labels.len(), 2);
    assert_eq!(
        script.labels["story_intro"][0].span.source,
        "story/intro.rns"
    );
}

#[test]
fn reports_cross_file_duplicate_declarations() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("a.rns"),
        "layer effects order 10\ndefine e = character \"First\"\nlabel start:\n    return\n",
    )
    .unwrap();
    fs::write(
        root.path().join("b.rns"),
        "layer effects order 20\ndefine e = character \"Second\"\nlabel start:\n    return\n",
    )
    .unwrap();

    let errors = load_project(root.path()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("character `e`"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("label `start`"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("display layer `effects`"))
    );
    assert!(errors.iter().all(|error| error.file == "b.rns"));
}

#[test]
fn loads_scripts_from_resource_archive() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("game");
    fs::create_dir_all(root.join("story")).unwrap();
    fs::write(
        root.join("script.rns"),
        "label start:\n    call chapter\n    return\n",
    )
    .unwrap();
    fs::write(
        root.join("story/chapter.rns"),
        "label chapter:\n    \"Archived\"\n    return\n",
    )
    .unwrap();
    let archive_path = temporary.path().join("game.renrs");
    crate::archive::pack_project(&root, &archive_path).unwrap();
    let archive = ResourceArchive::open(archive_path).unwrap();

    let script = load_project_archive(&archive).unwrap();
    assert!(script.labels.contains_key("start"));
    assert!(script.labels.contains_key("chapter"));
    assert_eq!(script.labels["chapter"][0].span.source, "story/chapter.rns");
}

#[test]
fn directory_and_archive_reject_oversized_scripts() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("game");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("script.rns"), "label start:\n    return\n").unwrap();
    fs::write(
        root.join("big.rns"),
        vec![b' '; crate::resource_budget::SCRIPT_BYTES + 1],
    )
    .unwrap();

    let directory_errors = load_project(&root).unwrap_err();
    assert!(
        directory_errors
            .iter()
            .any(|error| { error.file == "big.rns" && error.message.contains("byte budget") })
    );

    let archive_path = temporary.path().join("game.renrs");
    crate::archive::pack_project(&root, &archive_path).unwrap();
    let archive = ResourceArchive::open(archive_path).unwrap();
    let archive_errors = load_project_archive(&archive).unwrap_err();
    assert!(
        archive_errors
            .iter()
            .any(|error| { error.file == "big.rns" && error.message.contains("byte budget") })
    );
}

#[test]
fn open_read_limit_and_screens_validation_cover_source_edges() {
    use crate::source::{ProjectSource, ProjectSourceError};

    assert!(matches!(
        ProjectSource::open("missing-path"),
        Err(ProjectSourceError::InvalidPath(_))
    ));
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("script.rns"), "label start:\n    return\n").unwrap();
    fs::write(root.path().join("big.bin"), vec![0_u8; 32]).unwrap();
    fs::write(
        root.path().join("screens.json"),
        br#"{"hud":{"bounds":{"x":0,"y":0,"width":300,"height":80},"root":{"type":"image","path":"missing.png"}}}"#,
    )
    .unwrap();
    let source = ProjectSource::open(root.path()).unwrap();
    assert!(source.watch_root().is_some());
    assert!(source.path().ends_with(root.path().file_name().unwrap()));
    assert!(source.read_limited("big.bin", 8).is_err());
    assert!(source.read_limited("big.bin", 64).is_ok());
    assert!(
        source
            .validate_support_files()
            .iter()
            .any(|item| item.message.contains("missing") || item.file == "screens.json")
    );
    let script = crate::parse_script(
        "define e = character \"Eileen\" color \"red\" image missing\nlabel start:\n    return",
        "script.rns",
    )
    .unwrap();
    let found = crate::validator::validate(&script, root.path());
    assert!(
        found
            .iter()
            .any(|item| item.message.contains("character color"))
    );
    assert!(
        found
            .iter()
            .any(|item| item.message.contains("unknown image"))
    );
}
