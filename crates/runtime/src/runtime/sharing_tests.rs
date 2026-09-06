use super::*;

#[test]
fn checkpoints_share_large_inventory_but_edits_rollback_and_saved_copies_are_independent() {
    let program = crate::compile(&crate::parse_script("default bag = list()\ndefault score = 0\nlabel start:\n    \"Before\"\n    \"Still before\"\n    set score = 1\n    \"Scored\"\n    set bag = put(bag, 0, \"used\")\n    \"Used\"\n", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program).unwrap();
    runtime.advance().unwrap();
    let inventory = Arc::new(vec![Value::String("item".repeat(32)); 2000]);
    runtime
        .set_screen_variable("bag", Value::List(inventory.clone()))
        .unwrap();
    let saved = runtime.snapshot();
    assert!(Arc::ptr_eq(&saved.stage, &runtime.stage));
    assert!(Arc::ptr_eq(&saved.variables, &runtime.variables));
    runtime.continue_story().unwrap();
    assert!(Arc::ptr_eq(&saved.variables, &runtime.variables));
    runtime.continue_story().unwrap();
    assert!(!Arc::ptr_eq(&saved.variables, &runtime.variables));
    let Value::List(current) = &runtime.variables["bag"] else {
        panic!("list");
    };
    assert!(Arc::ptr_eq(current, &inventory));
    runtime.continue_story().unwrap();
    let Value::List(used) = &runtime.variables["bag"] else {
        panic!("list");
    };
    assert!(!Arc::ptr_eq(used, &inventory));
    assert_eq!(used[0], Value::String("used".to_owned()));
    assert_eq!(saved.variables["score"], Value::Integer(0));
    assert_eq!(saved.stage.dialogue.as_ref().unwrap().text, "Before");
    runtime.rollback().unwrap();
    assert_eq!(runtime.variables["bag"], saved.variables["bag"]);
    assert_eq!(runtime.variables["score"], Value::Integer(1));
    let mut restored = Runtime::restore(runtime.program.clone(), saved.clone()).unwrap();
    restored
        .apply_screen_expression("bag", "push(bag, \"extra\")")
        .unwrap();
    assert_eq!(saved.variables["bag"], Value::List(inventory));
}

#[test]
fn failed_screen_edit_preserves_shared_state_and_choice_checkpoint() {
    let program = crate::compile(&crate::parse_script("default denominator = 1\nlabel start:\n    menu:\n        \"Allowed\" if 10 / denominator > 0:\n            return\n        \"Cancel\":\n            return\n", "test.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program).unwrap();
    runtime.advance().unwrap();
    let before = runtime.snapshot();
    assert!(
        runtime
            .set_screen_variable("denominator", Value::Integer(0))
            .is_err()
    );
    assert!(Arc::ptr_eq(&before.variables, &runtime.variables));
    assert_eq!(
        serde_json::to_string(&before).unwrap(),
        serde_json::to_string(&runtime.snapshot()).unwrap()
    );
}
