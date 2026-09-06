use renrs::debugger::{ExploreLimits, Route, explore, run_route};
use renrs::{Runtime, WaitState};

fn program() -> renrs::Program {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("game");
    renrs::scaffold::create_project(&game, "Test", "org.example.test").unwrap();
    renrs::ProjectSource::open(game).unwrap().compile().unwrap()
}

#[test]
fn verifies_ending_and_variables_and_rejects_incomplete_routes() {
    let program = program();
    let route: Route = serde_json::from_str(
        r#"{"name":"answer","choices":[0],"expect_label":"answer","expect_variables":{"trust":1}}"#,
    )
    .unwrap();
    assert_eq!(
        run_route(&program, &route, 100).unwrap().label.as_deref(),
        Some("answer")
    );
    let mut invalid = route.clone();
    invalid.choices = vec![1];
    assert!(run_route(&program, &invalid, 100).is_err());
    invalid.choices.clear();
    assert!(run_route(&program, &invalid, 100).is_err());
    invalid.choices = vec![0, 1];
    assert!(run_route(&program, &invalid, 100).is_err());
    assert!(run_route(&program, &route, 1).is_err());
}

#[test]
fn explores_both_endings_and_reports_truncation() {
    let program = program();
    let report = explore(&program, ExploreLimits::default()).unwrap();
    assert!(report.complete);
    assert_eq!(report.finished_paths.len(), 2);
    assert!(report.uncovered_labels.is_empty());
    let report = explore(
        &program,
        ExploreLimits {
            max_runs: 1,
            ..ExploreLimits::default()
        },
    )
    .unwrap();
    assert!(!report.complete);
    assert_eq!(report.truncated_branches, 2);
}

#[test]
fn finished_label_survives_saving_restoring_and_rollback() {
    let program = program();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    runtime.choose(0).unwrap();
    assert_eq!(runtime.continue_story().unwrap(), WaitState::Finished);
    let mut restored = Runtime::restore(program, runtime.snapshot()).unwrap();
    assert_eq!(restored.current_label(), Some("answer"));
    restored.rollback().unwrap();
    assert_eq!(restored.current_label(), Some("answer"));
}
