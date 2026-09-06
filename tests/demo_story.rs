use std::path::PathBuf;

use renrs::{Runtime, WaitState, compile, load_project, validate};

fn demo_runtime() -> Runtime {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demo");
    let script = load_project(&root).expect("demo project should load");
    assert_eq!(validate(&script, &root), Vec::new());
    Runtime::new(compile(&script).expect("demo should compile")).expect("demo should start")
}

fn play_path(choices: &[usize]) -> Vec<String> {
    let mut runtime = demo_runtime();
    let mut next_choice = 0;
    let mut state = runtime.advance().expect("demo path should execute");
    loop {
        state = match state {
            WaitState::Dialogue | WaitState::Pause { .. } | WaitState::Effect { .. } => {
                runtime.continue_story().expect("demo path should continue")
            }
            WaitState::Choice { .. } => {
                let choice = choices[next_choice];
                next_choice += 1;
                runtime.choose(choice).expect("demo choice should execute")
            }
            WaitState::Finished => break,
        };
    }
    assert_eq!(next_choice, choices.len());
    runtime
        .history()
        .iter()
        .map(|dialogue| dialogue.text.clone())
        .collect()
}

#[test]
fn trusted_answer_path_reaches_its_ending() {
    let history = play_path(&[0, 0]);
    assert!(history.iter().any(|line| line.contains("Thank you")));
    assert!(
        history
            .iter()
            .any(|line| line.contains("second light wakes"))
    );
}

#[test]
fn skeptical_silence_path_reaches_its_ending() {
    let history = play_path(&[1, 1]);
    assert!(history.iter().any(|line| line.contains("not interference")));
    assert!(history.iter().any(|line| line.contains("keeps blinking")));
}
