use std::path::Path;
use std::process::{Command, Output};

fn run(binary: &str, arguments: &[&str]) -> Output {
    let executable = match binary {
        "check" => env!("CARGO_BIN_EXE_renrs-check"),
        "debug" => env!("CARGO_BIN_EXE_renrs-debug"),
        "inspect" => env!("CARGO_BIN_EXE_renrs-inspect"),
        _ => panic!("unknown binary"),
    };
    Command::new(executable).args(arguments).output().unwrap()
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

fn project(root: &Path) {
    renrs::scaffold::create_project(root, "Machine API", "org.renrs.machine").unwrap();
}

#[test]
fn machine_commands_share_the_v1_envelope() {
    let temporary = tempfile::tempdir().unwrap();
    let game = temporary.path().join("game");
    project(&game);

    let check = run("check", &["--json", game.to_str().unwrap()]);
    assert!(check.status.success());
    let check = json(&check);
    assert_eq!(check["protocol_version"], 1);
    assert_eq!(check["command"], "check");
    assert_eq!(check["ok"], true);
    assert!(check["data"]["fingerprint"].is_string());

    let routes = game.join("routes.json");
    let debug = run(
        "debug",
        &["test", game.to_str().unwrap(), routes.to_str().unwrap()],
    );
    assert!(debug.status.success());
    let debug = json(&debug);
    assert_eq!(debug["protocol_version"], 1);
    assert_eq!(debug["command"], "debug.test");
    assert_eq!(debug["data"].as_array().unwrap().len(), 2);

    let inspect = run("inspect", &[game.to_str().unwrap()]);
    assert!(inspect.status.success());
    let inspect = json(&inspect);
    assert_eq!(inspect["protocol_version"], 1);
    assert_eq!(inspect["command"], "inspect");
    assert_eq!(inspect["data"]["project_id"], "org.renrs.machine");
    assert_eq!(inspect["data"]["characters"].as_array().unwrap().len(), 1);
    assert_eq!(inspect["data"]["variables"].as_array().unwrap().len(), 1);
    assert_eq!(inspect["data"]["routes"]["passed"], 2);
    assert_eq!(
        inspect["data"]["routes"]["coverage"]["uncovered_labels"],
        serde_json::json!([])
    );
    assert_eq!(
        inspect["data"]["localization"][0]["missing"],
        serde_json::json!([])
    );
}

#[test]
fn usage_errors_are_json_and_exit_with_two() {
    for binary in ["check", "debug", "inspect"] {
        let arguments = match binary {
            "check" => vec!["--json", "--unknown"],
            "inspect" => vec!["--unknown", "extra"],
            _ => vec!["--unknown"],
        };
        let output = run(binary, &arguments);
        assert_eq!(output.status.code(), Some(2));
        let value = json(&output);
        assert_eq!(value["protocol_version"], 1);
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "invalid_arguments");
    }
}
