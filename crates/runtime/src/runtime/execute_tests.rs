use super::*;
use crate::{compile, parse_script};

fn runtime(source: &str) -> Runtime {
    Runtime::new(compile(&parse_script(source, "test.rns").unwrap()).unwrap()).unwrap()
}

#[test]
fn pause_scene_hide_and_jump_execute() {
    let mut runtime = runtime(
        r#"label start:
    scene "room.png"
    show "hero.png" as hero
    pause 0.5
    hide hero
    jump done
label done:
    "Arrived"
"#,
    );
    assert_eq!(
        runtime.advance().unwrap(),
        WaitState::Pause { seconds: 0.5 }
    );
    assert_eq!(runtime.stage().background.as_deref(), Some("room.png"));
    assert_eq!(runtime.stage().sprites.len(), 1);
    assert_eq!(runtime.continue_story().unwrap(), WaitState::Dialogue);
    assert!(runtime.stage().sprites.is_empty());
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Arrived");
}

#[test]
fn nvl_mode_and_video_wait_for_the_player() {
    let mut runtime = runtime(
        r#"label start:
    nvl on
    video "clip.mp4" over 1.5
    nvl off
    "Done"
"#,
    );
    let waiting = runtime.advance().unwrap();
    assert!(matches!(
        waiting,
        WaitState::Effect {
            effect: VisualEffect::Video { .. }
        }
    ));
    assert!(runtime.stage().nvl);
    assert_eq!(runtime.continue_story().unwrap(), WaitState::Dialogue);
    assert!(!runtime.stage().nvl);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Done");
}

#[test]
fn hide_screen_and_invalid_choices_are_reported() {
    let mut runtime = runtime(
        r#"label start:
    show screen bag
    hide screen bag
    menu:
        "Stay":
            return
        "Leave":
            return
"#,
    );
    assert!(matches!(
        runtime.advance().unwrap(),
        WaitState::Choice { .. }
    ));
    assert!(!runtime.stage().shown_screens.contains(&"bag".to_owned()));
    assert!(matches!(
        runtime.choose(3),
        Err(RuntimeError::InvalidChoice { index: 3, count: 2 })
    ));
}

#[test]
fn evaluate_expression_and_format_text_use_story_variables() {
    let mut runtime = runtime("default score = 3\nlabel start:\n    \"Ready {score}\"");
    runtime.advance().unwrap();
    assert_eq!(
        runtime
            .evaluate_expression(&Expr::Variable("score".into()))
            .unwrap(),
        Value::Integer(3)
    );
    assert_eq!(
        format_text("Score {score}", runtime.variables()).unwrap(),
        "Score 3"
    );
    assert!(matches!(runtime.choose(0), Err(RuntimeError::NotChoosing)));
}
