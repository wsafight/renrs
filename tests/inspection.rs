use renrs::inspection::inspect_project;
use renrs::scaffold::create_project;

#[test]
fn inspects_a_scaffolded_project() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("game");
    create_project(&game, "Inspect Me", "org.renrs.inspect").unwrap();
    let (inspection, diagnostics) =
        inspect_project(&game).unwrap_or_else(|error| panic!("{}: {}", error.code, error.message));
    assert_eq!(inspection.title, "Inspect Me");
    assert_eq!(inspection.project_id, "org.renrs.inspect");
    assert_eq!(inspection.source_kind, "directory");
    assert_eq!(
        inspection.snapshot_format,
        renrs::runtime::RuntimeSnapshot::FORMAT_VERSION
    );
    assert_eq!(
        inspection.save_container_format,
        renrs::save_format::SaveFile::CONTAINER_VERSION
    );
    assert!(
        inspection
            .script_files
            .iter()
            .any(|path| path.ends_with("script.rns"))
    );
    assert!(
        inspection
            .labels
            .iter()
            .any(|label| { label.name == "start" && label.statically_reachable })
    );
    assert!(diagnostics.iter().all(|item| !item.is_error()));
}
