use renrs::{ProjectSource, Runtime, save::SaveRepository};
use std::{fs, process::Command};

#[test]
fn release_check_validates_current_routes_and_saves_without_end_anchors() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("game");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("routes.json"),
        r#"{"routes":[{"name":"ending","choices":[],"expect_label":"start"}]}"#,
    )
    .unwrap();
    let script = "config id \"org.test.release\"\nlabel start:\n    \"Hello\"\n    return\n";
    fs::write(project.join("script.rns"), script).unwrap();
    let program = ProjectSource::open(&project).unwrap().compile().unwrap();
    let mut runtime = Runtime::new(program).unwrap();
    let saves = dir.path().join("saves");
    let repository = SaveRepository::new(&saves);
    runtime.advance().unwrap();
    repository.save("dialogue", &runtime.snapshot()).unwrap();
    runtime.continue_story().unwrap();
    repository.save("finished", &runtime.snapshot()).unwrap();
    let check = || {
        Command::new(env!("CARGO_BIN_EXE_renrs-accept"))
            .arg(&project)
            .arg("--saves")
            .arg(&saves)
            .output()
            .unwrap()
    };
    let output = check();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["checks"].as_array().unwrap().len(), 3);

    fs::write(
        project.join("script.rns"),
        script.replace("Hello", "Updated"),
    )
    .unwrap();
    let output = check();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let checks = report["checks"].as_array().unwrap();
    assert!(
        checks
            .iter()
            .filter(|check| check["kind"] == "route")
            .all(|check| check["passed"] == true)
    );
    assert!(
        checks
            .iter()
            .filter(|check| check["kind"] == "save")
            .all(|check| check["passed"] == false
                && check["error"]
                    .as_str()
                    .unwrap()
                    .contains("different script version"))
    );
}
