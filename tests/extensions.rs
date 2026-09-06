use renrs::{Runtime, compile, parse_script, syntax::Value};

#[test]
fn script_and_screen_extensions_preserve_exact_values_and_rollback() {
    let script = "default value = 9007199254740993\nlabel start:\n    \"Before\"\n    extend value = \"double\" value\n    \"After\"";
    let mut program = compile(&parse_script(script, "extension.rns").unwrap()).unwrap();
    program
        .extensions
        .insert("double".into(), "input * 2".into());
    program
        .extensions
        .insert("fail".into(), "throw \"failure\";".into());
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    assert_eq!(
        runtime.variables()["value"],
        Value::Integer(18_014_398_509_481_986)
    );
    let saved = serde_json::to_vec(&runtime.snapshot()).unwrap();
    assert!(
        runtime
            .apply_extension("value", "fail", &Value::Integer(0))
            .is_err()
    );
    assert_eq!(saved, serde_json::to_vec(&runtime.snapshot()).unwrap());
    let mut restored = Runtime::restore(program, serde_json::from_slice(&saved).unwrap()).unwrap();
    restored.rollback().unwrap();
    assert_eq!(
        restored.variables()["value"],
        Value::Integer(9_007_199_254_740_993)
    );
    restored
        .apply_extension_expression("value", "double", "value")
        .unwrap();
    assert_eq!(
        restored.variables()["value"],
        Value::Integer(18_014_398_509_481_986)
    );
}
