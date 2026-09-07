use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn impact(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_renrs-impact"))
        .args(arguments)
        .output()
        .unwrap()
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid JSON: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn project(path: &Path) {
    renrs::scaffold::create_project(path, "Impact", "org.renrs.impact").unwrap();
    fs::write(
        path.join("progress.json"),
        r#"{"endings":[{"id":"answer","title":"Answer","label":"answer"},{"id":"wait","title":"Wait","label":"wait"}]}"#,
    )
    .unwrap();
}

fn change_candidate(path: &Path) {
    let script_path = path.join("script.rns");
    let script = fs::read_to_string(&script_path).unwrap();
    let changed = script.replace(
        "The signal is back. Shall we answer?",
        "The signal changed. Shall we answer?",
    ) + "\nlabel orphan:\n    \"Nobody reaches this line.\"\n";
    fs::write(script_path, changed).unwrap();
    let progress = fs::read_to_string(path.join("progress.json")).unwrap();
    fs::write(
        path.join("progress.json"),
        progress.replace("\"Answer\"", "\"Changed answer\""),
    )
    .unwrap();
}

#[test]
fn compares_two_projects_and_reports_semantic_impact() {
    let temporary = tempfile::tempdir().unwrap();
    let baseline = temporary.path().join("baseline");
    let candidate = temporary.path().join("candidate");
    project(&baseline);
    project(&candidate);
    change_candidate(&candidate);

    let output = impact(&[baseline.to_str().unwrap(), candidate.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = json(&output);
    assert_eq!(value["protocol_version"], 1);
    assert_eq!(value["command"], "impact");
    assert_eq!(value["data"]["changed"], true);
    assert_eq!(
        value["data"]["labels"]["added"],
        serde_json::json!(["orphan"])
    );
    assert_eq!(
        value["data"]["new_unreachable_labels"],
        serde_json::json!(["orphan"])
    );
    assert!(
        value["data"]["routes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|route| {
                route["reasons"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::json!("executed_content_changed"))
            })
    );
    assert_eq!(value["data"]["endings"][0]["id"], "answer");
    assert!(
        value["data"]["translations"]["source_ids_changed"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("opening"))
    );
    assert_eq!(value["data"]["save_compatibility"]["risk"], "high");
    assert_eq!(
        value["data"]["save_compatibility"]["requires_save_fixture_validation"],
        true
    );
}

#[test]
fn compares_worktree_to_a_git_revision() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path();
    let game = repository.join("game");
    project(&game);
    for arguments in [
        vec!["init", "-q"],
        vec!["config", "user.email", "renrs@example.invalid"],
        vec!["config", "user.name", "RenRS Test"],
        vec!["add", "game"],
        vec!["commit", "-q", "-m", "baseline"],
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    change_candidate(&game);
    let output = impact(&["--git", game.to_str().unwrap(), "HEAD"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = json(&output);
    assert_eq!(
        value["data"]["labels"]["added"],
        serde_json::json!(["orphan"])
    );
    assert_ne!(
        value["data"]["baseline"]["fingerprint"],
        value["data"]["candidate"]["fingerprint"]
    );
}
