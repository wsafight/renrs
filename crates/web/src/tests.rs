use super::*;

#[test]
fn native_save_exchange_preserves_large_integers_and_presentation() {
    let program = renrs_compiler::compile(&renrs_compiler::parse_script(
        "config id \"org.test.exchange\"\ndefault value = 9007199254740993\nlabel start:\n    @id \"opening\" \"Value {value}\"\n    return\n", "save.rns").unwrap()).unwrap();
    let mut engine = Engine::new(&serde_json::to_string(&program).unwrap(), "").unwrap();
    engine.action("start", 0).unwrap();
    let snapshot = engine.snapshot().unwrap();
    let exported = engine.export_save(&snapshot, r#"{"time":1234000,"note":"Across platforms","remaining":250,"presentation":{"dialogue_page":2,"visible_characters":45}}"#).unwrap();
    let native: renrs_runtime::save_format::SaveFile = serde_json::from_str(&exported).unwrap();
    native.validate("org.test.exchange").unwrap();
    assert_eq!(native.presentation.as_ref().unwrap().dialogue_page, 2);
    assert_eq!(
        native.snapshot.variables["value"],
        renrs_syntax::syntax::Value::Integer(9_007_199_254_740_993)
    );
    let imported: serde_json::Value =
        serde_json::from_str(&engine.import_save(&exported).unwrap()).unwrap();
    assert_eq!(imported["snapshot"], snapshot);
    assert_eq!(imported["remaining"], 250);
    let mut corrupted = native;
    corrupted.presentation.as_mut().unwrap().note = "changed".to_owned();
    assert!(corrupted.validate("org.test.exchange").is_err());
}

#[test]
fn action_payload_is_bounded_as_history_grows() {
    let script = format!(
        "label start:\n{}",
        "    \"A line of dialogue\"\n".repeat(1200)
    );
    let program =
        renrs_compiler::compile(&renrs_compiler::parse_script(&script, "test.rns").unwrap())
            .unwrap();
    let mut engine = Engine::new(&serde_json::to_string(&program).unwrap(), "").unwrap();
    let initial = engine.action("start", 0).unwrap().len();
    for _ in 0..1100 {
        engine.action("next", 0).unwrap();
    }
    let state = engine.state().unwrap();
    assert!(state.len() < initial + 100);
    let value: serde_json::Value = serde_json::from_str(&state).unwrap();
    assert_eq!(value["history_count"], 1101);
    assert!(value.get("history").is_none());
    assert!(value.get("coverage").is_none());
    let page: Vec<serde_json::Value> =
        serde_json::from_str(&engine.history(1050, 1000).unwrap()).unwrap();
    assert_eq!(page.len(), 51);
    assert_eq!(engine.history(usize::MAX, usize::MAX).unwrap(), "[]");
    let inspection: serde_json::Value = serde_json::from_str(&engine.inspect().unwrap()).unwrap();
    assert!(inspection["coverage"].as_array().unwrap().len() >= 1101);
}

#[test]
fn repeated_state_omits_an_unchanged_stage() {
    let program = renrs_compiler::compile(
        &renrs_compiler::parse_script("label start:\n    \"Hello\"\n    return\n", "state.rns")
            .unwrap(),
    )
    .unwrap();
    let mut engine = Engine::new(&serde_json::to_string(&program).unwrap(), "").unwrap();
    let first: serde_json::Value =
        serde_json::from_str(&engine.action("start", 0).unwrap()).unwrap();
    assert!(first.get("stage").is_some_and(|stage| !stage.is_null()));
    let second: serde_json::Value = serde_json::from_str(&engine.state().unwrap()).unwrap();
    assert!(second.get("stage").is_some_and(serde_json::Value::is_null));
    assert_eq!(second["history_count"], first["history_count"]);
}

#[test]
fn state_includes_stage_mutated_between_non_checkpoint_waits() {
    let program = renrs_compiler::compile(
        &renrs_compiler::parse_script(
            "label start:\n    scene \"one.png\"\n    pause 1\n    scene \"two.png\"\n    pause 1\n    return\n",
            "state.rns",
        )
        .unwrap(),
    )
    .unwrap();
    let mut engine = Engine::new(&serde_json::to_string(&program).unwrap(), "").unwrap();
    let first: serde_json::Value =
        serde_json::from_str(&engine.action("start", 0).unwrap()).unwrap();
    assert_eq!(first["stage"]["background"], "one.png");

    let second: serde_json::Value =
        serde_json::from_str(&engine.action("next", 0).unwrap()).unwrap();
    assert_eq!(second["stage"]["background"], "two.png");
}
