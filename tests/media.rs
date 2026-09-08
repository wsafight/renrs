#[test]
fn keyframes_dissolve_and_video_restore_at_each_wait() {
    let script = "label start:\n    scene \"old.png\"\n    show \"person.png\" as p\n    \"Before\"\n    timeline:\n        transform p x 80 over 0.2\n        transform p alpha 0.5 over 0.3\n    scene \"new.png\"\n    transition dissolve 0.4\n    video \"clip.json\" over 1\n    \"After\"";
    let program = renrs::compile(&renrs::parse_script(script, "test.rns").unwrap()).unwrap();
    let mut runtime = renrs::Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    for _ in 0..4 {
        let wait = runtime.continue_story().unwrap();
        assert!(matches!(wait, renrs::WaitState::Effect { .. }));
        let restored = renrs::Runtime::restore(program.clone(), runtime.snapshot()).unwrap();
        assert_eq!(restored.waiting(), Some(&wait));
        assert_eq!(restored.stage(), runtime.stage());
    }
    runtime.continue_story().unwrap();
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "After");
    runtime.rollback().unwrap();
    assert_eq!(runtime.stage().background.as_deref(), Some("old.png"));
}

#[test]
fn rejected_screen_edit_preserves_variables_profile_and_snapshot() {
    let script = "default persistent_divisor = 1\nlabel start:\n    menu:\n        \"Continue\" if 10 / persistent_divisor > 0:\n            return\n        \"Cancel\":\n            return\n";
    let program = renrs::compile(&renrs::parse_script(script, "screen.rns").unwrap()).unwrap();
    let mut runtime = renrs::Runtime::new(program).unwrap();
    runtime.advance().unwrap();
    let before = serde_json::to_value(runtime.snapshot()).unwrap();
    let profile = runtime.profile().clone();
    assert!(
        runtime
            .set_screen_variable("persistent_divisor", renrs::syntax::Value::Integer(0))
            .is_err()
    );
    assert_eq!(serde_json::to_value(runtime.snapshot()).unwrap(), before);
    assert_eq!(runtime.profile(), &profile);
    let invalid = renrs::expression::parse_expression("get(list(0), 0)", "screen", 1, 1).unwrap();
    assert!(
        runtime
            .apply_screen_expression("persistent_divisor", &invalid)
            .is_err()
    );
    assert_eq!(serde_json::to_value(runtime.snapshot()).unwrap(), before);
    let valid =
        renrs::expression::parse_expression("get(record(\"safe\", 2), \"safe\")", "screen", 1, 1)
            .unwrap();
    runtime
        .apply_screen_expression("persistent_divisor", &valid)
        .unwrap();
    let restored = renrs::Runtime::restore(runtime.shared_program(), runtime.snapshot()).unwrap();
    assert_eq!(
        restored.variables()["persistent_divisor"],
        renrs::syntax::Value::Integer(2)
    );
}
