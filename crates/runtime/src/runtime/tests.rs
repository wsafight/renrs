use super::*;
use crate::{compile, parse_script};

fn runtime(source: &str) -> Runtime {
    Runtime::new(compile(&parse_script(source, "test.rns").unwrap()).unwrap()).unwrap()
}

#[test]
fn executes_choice_and_condition() {
    let mut runtime = runtime(
        r#"label start:
    set score = 1
    menu "Score: {score}. Choose":
        "Gain":
            set score = score + 1
        "Wait":
            set score = 0
    if score >= 2:
        "Score: {score}"
    else:
        "Low"
"#,
    );
    assert!(matches!(
        runtime.advance().unwrap(),
        WaitState::Choice { .. }
    ));
    assert_eq!(
        runtime.stage().dialogue.as_ref().unwrap().text,
        "Score: 1. Choose"
    );
    assert_eq!(runtime.history().len(), 1);
    assert_eq!(runtime.choose(0).unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Score: 2");
}

#[test]
fn call_returns_to_next_instruction() {
    let mut runtime = runtime(
        r#"label start:
    call intro
    "After"
    return
label intro:
    "Intro"
    return
"#,
    );
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Intro");
    assert_eq!(runtime.continue_story().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "After");
}

#[test]
fn snapshot_restores_waiting_state() {
    let program =
        compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    let snapshot = runtime.snapshot();
    let restored = Runtime::restore(program, snapshot).unwrap();
    assert_eq!(restored.waiting(), Some(&WaitState::Dialogue));
    assert_eq!(restored.stage().dialogue.as_ref().unwrap().text, "Hello");
}

#[test]
fn strict_restore_rejects_edits_while_explicit_positions_restore_compatibly() {
    let original =
        compile(&parse_script("label start:\n    @id \"line\" \"Before\"", "test.rns").unwrap())
            .unwrap();
    let edited =
        compile(&parse_script("label start:\n    @id \"line\" \"After\"", "test.rns").unwrap())
            .unwrap();
    let mut runtime = Runtime::new(original).unwrap();
    runtime.advance().unwrap();
    let snapshot = runtime.snapshot();

    assert!(matches!(
        Runtime::restore(edited.clone(), snapshot.clone()),
        Err(RuntimeError::ScriptChanged)
    ));
    let (compatible, report) = Runtime::restore_compatible(edited.clone(), snapshot).unwrap();
    assert!(report.alias_resolutions.is_empty());
    assert_eq!(compatible.waiting(), Some(&WaitState::Dialogue));
    assert_eq!(compatible.stage().dialogue.as_ref().unwrap().text, "Before");
    runtime.reload(edited).unwrap();
    assert_eq!(runtime.waiting(), Some(&WaitState::Dialogue));
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Before");
    assert!(runtime.audio_events.is_empty());
}

#[test]
fn compatible_restore_rejects_automatic_positions_after_an_edit() {
    let original =
        compile(&parse_script("label start:\n    \"Before\"", "test.rns").unwrap()).unwrap();
    let edited =
        compile(&parse_script("label start:\n    \"After\"", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(original).unwrap();
    runtime.advance().unwrap();

    assert!(matches!(
        Runtime::restore_compatible(edited, runtime.snapshot()),
        Err(RuntimeError::UnstableSavePosition(_))
    ));
}

#[test]
fn reload_refreshes_choice_options_without_replaying() {
    let original = compile(
            &parse_script(
                "label start:\n    @id \"menu\" menu:\n        \"Old A\":\n            return\n        \"Old B\":\n            return",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
    let edited = compile(
            &parse_script(
                "label start:\n    @id \"menu\" menu:\n        \"New A\":\n            return\n        \"New B\":\n            return",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
    let mut runtime = Runtime::new(original).unwrap();
    runtime.advance().unwrap();

    runtime.reload(edited).unwrap();
    assert_eq!(
        runtime.waiting(),
        Some(&WaitState::Choice {
            options: vec!["New A".to_owned(), "New B".to_owned()]
        })
    );
    assert!(runtime.audio_events.is_empty());
}

#[test]
fn rejects_noncurrent_snapshot_formats() {
    let program =
        compile(&parse_script("label start:\n    \"Hello\"", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    for version in (0..RuntimeSnapshot::FORMAT_VERSION)
        .chain(std::iter::once(RuntimeSnapshot::FORMAT_VERSION + 1))
    {
        let mut snapshot = runtime.snapshot();
        snapshot.format_version = version;
        assert!(matches!(
            Runtime::restore(program.clone(), snapshot),
            Err(RuntimeError::SaveVersion { found, .. }) if found == version
        ));
    }
}

#[test]
fn rollback_restores_choice_and_branch_state() {
    let mut runtime = runtime(
        "label start:\n    set score = 0\n    menu:\n        \"Gain\":\n            set score = 1\n            \"Gained\"\n        \"Wait\":\n            \"Waited\"\n    \"After\"",
    );
    assert!(matches!(
        runtime.advance().unwrap(),
        WaitState::Choice { .. }
    ));
    assert_eq!(runtime.choose(0).unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.variables().get("score"), Some(&Value::Integer(1)));

    let waiting = runtime.rollback().unwrap();
    assert!(matches!(waiting, WaitState::Choice { .. }));
    assert_eq!(runtime.variables().get("score"), Some(&Value::Integer(0)));
    assert_eq!(runtime.choose(1).unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Waited");
}

#[test]
fn snapshot_preserves_rollback_checkpoints() {
    let program =
        compile(&parse_script("label start:\n    \"First\"\n    \"Second\"", "test.rns").unwrap())
            .unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    let mut restored = Runtime::restore(program, runtime.snapshot()).unwrap();

    assert!(restored.can_rollback());
    assert_eq!(restored.rollback().unwrap(), WaitState::Dialogue);
    assert_eq!(restored.stage().dialogue.as_ref().unwrap().text, "First");
}

#[test]
fn emits_layered_tween_and_fade_effects() {
    let mut runtime = runtime(
        "label start:\n    show \"hero.png\" as hero at left layer 7\n    move hero to right over 0.5\n    transition fade 0.25\n    \"Done\"",
    );
    assert_eq!(
        runtime.advance().unwrap(),
        WaitState::Effect {
            effect: VisualEffect::Tween {
                alias: "hero".to_owned(),
                from: Position::Left,
                to: Position::Right,
                seconds: 0.5,
            }
        }
    );
    assert_eq!(runtime.stage().sprites[0].layer, 7);
    assert_eq!(
        runtime.continue_story().unwrap(),
        WaitState::Effect {
            effect: VisualEffect::Fade { seconds: 0.25 }
        }
    );
    assert_eq!(runtime.continue_story().unwrap(), WaitState::Dialogue);
}

#[test]
fn transforms_are_eased_serializable_and_rollback_safe() {
    let program = compile(
            &parse_script(
                "label start:\n    show \"hero.png\" as hero\n    transform hero x 20 y -10 scale 0.75 rotate 15 alpha 0.5 anchor 0.5 1 crop 0 0 100 200 over 0.4 ease in_out\n    \"Done\"\n",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    let waiting = runtime.advance().unwrap();
    assert!(matches!(
        waiting,
        WaitState::Effect {
            effect: VisualEffect::Transform {
                easing: Easing::EaseInOut,
                ..
            }
        }
    ));
    let transform = runtime.stage().sprites[0].transform;
    assert!((transform.x - 20.0).abs() < f32::EPSILON);
    assert!((transform.scale - 0.75).abs() < f32::EPSILON);
    assert!((transform.crop.unwrap().height - 200.0).abs() < f32::EPSILON);
    let restored = Runtime::restore(program, runtime.snapshot()).unwrap();
    assert_eq!(restored.stage().sprites[0].transform, transform);
    assert!((Easing::EaseInOut.sample(0.25) - 0.125).abs() < f32::EPSILON);
}

#[test]
fn voice_channel_stops_when_dialogue_advances() {
    let mut runtime =
        runtime("label start:\n    voice \"audio/line.wav\"\n    \"Spoken\"\n    \"Silent\"");
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().voice.as_deref(), Some("audio/line.wav"));
    assert_eq!(
        runtime.drain_audio_events().collect::<Vec<_>>(),
        vec![AudioEvent::PlayVoice {
            path: "audio/line.wav".to_owned()
        }]
    );

    assert_eq!(runtime.continue_story().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().voice, None);
    assert_eq!(
        runtime.drain_audio_events().collect::<Vec<_>>(),
        vec![AudioEvent::StopVoice]
    );
}

#[test]
fn music_queue_and_fades_are_serializable_channel_state() {
    let program = compile(
            &parse_script(
                "label start:\n    play music \"audio/one.ogg\" fadein 0.5 volume 0.4\n    queue music \"audio/two.ogg\" loop volume 0.7 fadein 0.25\n    play sound \"audio/click.wav\" volume 0.25\n    \"Playing\"\n    stop music fadeout 0.4\n    \"Stopped\"\n",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert!((runtime.stage().music.as_ref().unwrap().fade_in - 0.5).abs() < f32::EPSILON);
    assert!((runtime.stage().music.as_ref().unwrap().volume - 0.4).abs() < f32::EPSILON);
    assert_eq!(runtime.stage().music_queue.len(), 1);
    assert!((runtime.stage().music_queue[0].volume - 0.7).abs() < f32::EPSILON);
    assert!(runtime.stage().music_queue[0].repeat);
    assert!(
        runtime
            .drain_audio_events()
            .any(|event| matches!(event, AudioEvent::PlaySound { volume: 0.25, .. }))
    );
    let snapshot = runtime.snapshot();
    let mut restored = Runtime::restore(program, snapshot).unwrap();
    assert_eq!(restored.stage().music_queue.len(), 1);
    assert!((restored.stage().music_queue[0].volume - 0.7).abs() < f32::EPSILON);
    restored.complete_music_track();
    assert_eq!(
        restored
            .stage()
            .music
            .as_ref()
            .map(|music| music.path.as_str()),
        Some("audio/two.ogg")
    );
    assert!(restored.stage().music_queue.is_empty());
}

#[test]
fn detects_uninitialized_branch_variable() {
    let mut runtime = runtime(
        r#"label start:
    if false:
        set score = 1
    if score > 0:
        "Never"
    return
"#,
    );
    let error = runtime.advance().unwrap_err();
    assert!(error.to_string().contains("not been assigned"));
}

#[test]
fn defaults_parameters_returns_and_conditional_choices_execute() {
    let mut runtime = runtime(
        r#"default score = 2
label start:
    call double(score)
    menu:
        "Shown {_return}" if _return == 4:
            "Selected"
        "Hidden" if false:
            "Never"
label double(value):
    return value * 2
"#,
    );
    assert_eq!(
        runtime.advance().unwrap(),
        WaitState::Choice {
            options: vec!["Shown 4".to_owned()]
        }
    );
    assert_eq!(runtime.choose(0).unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Selected");
}

#[test]
fn named_and_default_arguments_use_the_callers_variable_snapshot() {
    let mut runtime = runtime(
        r#"default value = 10
label start:
    call combine(value=2)
    "Result {_return}"
label combine(value, other=value):
    return value * 100 + other
"#,
    );
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(
        runtime.stage().dialogue.as_ref().unwrap().text,
        "Result 210"
    );
    assert_eq!(runtime.variables().get("value"), Some(&Value::Integer(10)));
    assert!(!runtime.variables().contains_key("other"));
}

#[test]
fn nested_parameter_scopes_survive_save_and_rollback() {
    let mut runtime = runtime(
        r#"default value = 9
label start:
    call outer(1)
    "Result {_return}, value {value}"
label outer(value):
    call inner(2)
    return value * 10 + _return
label inner(value):
    "Inner {value}"
    return value
"#,
    );
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Inner 2");

    let encoded = serde_json::to_vec(&runtime.snapshot()).unwrap();
    let snapshot = serde_json::from_slice(&encoded).unwrap();
    let mut restored = Runtime::restore(runtime.shared_program(), snapshot).unwrap();
    assert_eq!(restored.continue_story().unwrap(), WaitState::Dialogue);
    assert_eq!(
        restored.stage().dialogue.as_ref().unwrap().text,
        "Result 12, value 9"
    );
    assert_eq!(restored.variables().get("value"), Some(&Value::Integer(9)));

    assert_eq!(restored.rollback().unwrap(), WaitState::Dialogue);
    assert_eq!(restored.stage().dialogue.as_ref().unwrap().text, "Inner 2");
    assert_eq!(restored.variables().get("value"), Some(&Value::Integer(2)));
    assert_eq!(restored.continue_story().unwrap(), WaitState::Dialogue);
    assert_eq!(
        restored.stage().dialogue.as_ref().unwrap().text,
        "Result 12, value 9"
    );
}

#[test]
fn restore_rejects_parameter_scope_that_does_not_match_its_call() {
    let mut runtime = runtime(
        "label start:\n    call target(1)\nlabel target(value):\n    \"Inside\"\n    return",
    );
    runtime.advance().unwrap();
    let mut snapshot = runtime.snapshot();
    snapshot.call_stack[0].previous_variables.clear();
    assert!(matches!(
        Runtime::restore(runtime.shared_program(), snapshot),
        Err(RuntimeError::InvalidStablePositions)
    ));
}

#[test]
fn localizes_before_interpolation_and_markup() {
    let mut runtime = runtime(
        r#"default name = "Eileen"
label start:
    @id "hello" "Hello, {name}"
"#,
    );
    runtime
        .insert_translation_catalog(TranslationCatalog {
            language: "zh".to_owned(),
            fallback: None,
            plurals: BTreeMap::new(),
            messages: BTreeMap::from([(
                TranslationId::new("hello").unwrap(),
                "{b}你好，{name}{/b}".to_owned(),
            )]),
        })
        .unwrap();
    runtime.set_language(Some("zh-CN".to_owned())).unwrap();
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(
        runtime.stage().dialogue.as_ref().unwrap().text,
        "你好，Eileen"
    );
}

#[test]
fn anchored_dialogue_reloads_through_alias_without_replay() {
    let original = compile(
        &parse_script(
            "label start:\n    @id \"line.old\" \"Before\"\n    return",
            "test.rns",
        )
        .unwrap(),
    )
    .unwrap();
    let edited = compile(
        &parse_script(
            "label start:\n    @id \"line.new\" alias \"line.old\" \"After\"\n    return",
            "test.rns",
        )
        .unwrap(),
    )
    .unwrap();
    let mut runtime = Runtime::new(original).unwrap();
    runtime.advance().unwrap();
    let report = runtime.reload_with_report(edited).unwrap();
    assert_eq!(report.alias_resolutions.len(), 1);
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "Before");
    assert_eq!(runtime.waiting(), Some(&WaitState::Dialogue));
}

#[test]
fn checkpoints_store_only_history_offsets() {
    let mut runtime = runtime("label start:\n    \"One\"\n    \"Two\"");
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    let snapshot = runtime.snapshot();
    assert_eq!(snapshot.history.len(), 2);
    let encoded = serde_json::to_value(&snapshot).unwrap();
    assert!(
        encoded["rollback"]
            .as_array()
            .unwrap()
            .iter()
            .all(|checkpoint| checkpoint.get("history").is_none())
    );
    assert_eq!(snapshot.rollback.last().unwrap().history_len, 2);
}

#[test]
fn corrupt_rollback_is_rejected_instead_of_discarded_on_load() {
    let mut runtime = runtime("label start:\n    \"One\"\n    \"Two\"");
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    let mut snapshot = runtime.snapshot();
    snapshot.rollback[0].history_len = snapshot.history.len() + 1;
    assert!(matches!(
        Runtime::restore(runtime.shared_program(), snapshot),
        Err(RuntimeError::InvalidWaitState)
    ));
}
