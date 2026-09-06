use std::process::Command;

#[test]
fn initializes_a_playable_localized_project_and_refuses_to_overwrite_it() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("my story");
    let output = Command::new(env!("CARGO_BIN_EXE_renrs-init"))
        .arg(&game)
        .args(["--title", "A \"quoted\" title", "--id", "org.example.story"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let program = renrs::ProjectSource::open(&game)
        .unwrap()
        .compile()
        .unwrap();
    assert_eq!(program.title, "A \"quoted\" title");
    for choice in [0, 1] {
        let mut runtime = renrs::Runtime::new(program.clone()).unwrap();
        runtime.advance().unwrap();
        runtime.continue_story().unwrap();
        runtime.choose(choice).unwrap();
        assert_eq!(
            runtime.continue_story().unwrap(),
            renrs::WaitState::Finished
        );
    }
    let before = std::fs::read(game.join("script.rns")).unwrap();
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_renrs-init"))
            .arg(&game)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(before, std::fs::read(game.join("script.rns")).unwrap());
}
