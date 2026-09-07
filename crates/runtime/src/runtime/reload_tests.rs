use super::*;
use crate::{compile, parse_script};

fn program(source: &str) -> Program {
    compile(&parse_script(source, "story.rns").unwrap()).unwrap()
}

#[test]
fn ambiguous_reload_is_rejected_without_changing_the_session() {
    let mut runtime = Runtime::new(program(
        "label start:\n    \"A\"\n    \"B\"\n    \"C\"\n    \"D\"\n",
    ))
    .unwrap();
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    for edited in [
        "label start:\n    \"B\"\n    \"C\"\n    \"D\"\n",
        "label start:\n    \"X\"\n    \"A\"\n    \"B\"\n    \"C\"\n    \"D\"\n",
        "label start:\n    \"B\"\n    \"A\"\n    \"C\"\n    \"D\"\n",
    ] {
        assert!(matches!(
            runtime.reload(program(edited)),
            Err(RuntimeError::UnstableSavePosition(_))
        ));
        assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "B");
    }
}

#[test]
fn explicit_dialogue_and_call_site_survive_insertions() {
    let original = "label start:\n    @id \"call\" call chapter\n    \"After\"\nlabel chapter:\n    @id \"checkpoint\" \"Saved\"\n    return\n";
    let edited = "label start:\n    \"Before call\"\n    @id \"call\" call chapter\n    \"Inserted after call\"\n    \"After\"\nlabel chapter:\n    \"Before checkpoint\"\n    @id \"checkpoint\" \"Saved\"\n    return\n";
    let mut runtime = Runtime::new(program(original)).unwrap();
    runtime.advance().unwrap();
    runtime.reload(program(edited)).unwrap();
    runtime.continue_story().unwrap();
    assert_eq!(
        runtime.stage().dialogue.as_ref().unwrap().text,
        "Inserted after call"
    );
}

#[test]
fn reload_initializes_new_defaults_in_the_session_and_rollback() {
    let original = "default score = 1\nlabel start:\n    set score = 7\n    @id \"one\" \"One\"\n    @id \"two\" \"Two\"\n    return\n";
    let edited = "default score = 99\ndefault bonus = score + 1\nlabel start:\n    set score = 7\n    @id \"one\" \"One\"\n    @id \"two\" \"Two\"\n    \"{bonus}\"\n    return\n";
    let mut runtime = Runtime::new(program(original)).unwrap();
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    let report = runtime.reload_with_report(program(edited)).unwrap();
    assert_eq!(report.initialized_defaults, ["bonus"]);
    assert_eq!(runtime.variables()["score"], Value::Integer(7));
    runtime.rollback().unwrap();
    assert_eq!(runtime.variables()["bonus"], Value::Integer(8));
    runtime.continue_story().unwrap();
    runtime.continue_story().unwrap();
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "8");
}

#[test]
fn current_snapshot_restores_call_sites_through_rollback() {
    let program = program(
        "label start:\n    call chapter\n    \"After\"\nlabel chapter:\n    \"One\"\n    \"Two\"\n    return\n",
    );
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    let mut restored = Runtime::restore(program.clone(), runtime.snapshot()).unwrap();
    restored.rollback().unwrap();
    let mut restored = Runtime::restore(program, restored.snapshot()).unwrap();
    restored.continue_story().unwrap();
    restored.continue_story().unwrap();
    assert_eq!(restored.stage().dialogue.as_ref().unwrap().text, "After");
}

#[test]
fn reload_preserves_matching_parameter_scopes_and_rejects_signature_changes() {
    let original = "label start:\n    @id \"call\" call chapter(1)\n    \"After {value}\"\nlabel chapter(value):\n    @id \"inside\" \"Inside {value}\"\n    return\n";
    let edited = "default value = 9\nlabel start:\n    @id \"call\" call chapter(1)\n    \"After {value}\"\nlabel chapter(value):\n    @id \"inside\" \"Edited {value}\"\n    return\n";
    let incompatible = "default value = 9\nlabel start:\n    @id \"call\" call chapter(1)\n    \"After {value}\"\nlabel chapter(value, extra=2):\n    @id \"inside\" \"Edited {value}\"\n    return\n";

    let mut runtime = Runtime::new(program(original)).unwrap();
    runtime.advance().unwrap();
    assert_eq!(runtime.variables()["value"], Value::Integer(1));
    assert!(matches!(
        runtime.reload(program(incompatible)),
        Err(RuntimeError::InvalidStablePositions)
    ));
    assert_eq!(runtime.variables()["value"], Value::Integer(1));

    let report = runtime.reload_with_report(program(edited)).unwrap();
    assert_eq!(report.initialized_defaults, ["value"]);
    runtime.continue_story().unwrap();
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "After 9");
    assert_eq!(runtime.variables()["value"], Value::Integer(9));
}

#[test]
fn reload_refreshes_layer_order_and_rejects_removed_active_layers() {
    let original = "layer effects order 10\nlabel start:\n    show \"glow.png\" onlayer effects\n    @id \"line\" \"Before\"\n";
    let reordered = "layer effects order 80\nlabel start:\n    show \"glow.png\" onlayer effects\n    @id \"line\" \"After\"\n";
    let removed = "label start:\n    @id \"line\" \"After\"\n";
    let mut runtime = Runtime::new(program(original)).unwrap();
    runtime.advance().unwrap();
    assert_eq!(runtime.stage().sprites[0].display_order, 10);

    runtime.reload(program(reordered)).unwrap();
    assert_eq!(runtime.stage().sprites[0].display_order, 80);
    assert!(matches!(
        runtime.reload(program(removed)),
        Err(RuntimeError::SavedDisplayLayerMissing(ref name)) if name == "effects"
    ));
    assert_eq!(runtime.stage().sprites[0].display_order, 80);
}

#[test]
fn snapshot_shares_history_until_mutation_without_changing_saved_data() {
    let program = crate::compile(
        &crate::parse_script("label start:\n    \"Before\"\n    \"After\"", "test.rns").unwrap(),
    )
    .unwrap();
    let program = std::sync::Arc::new(program);
    let mut runtime = super::Runtime::new(program.clone()).unwrap();
    assert!(std::sync::Arc::ptr_eq(&program, &runtime.shared_program()));
    runtime.advance().unwrap();
    let snapshot = runtime.snapshot();
    assert!(std::sync::Arc::ptr_eq(
        &snapshot.history,
        &runtime.snapshot().history
    ));
    runtime.continue_story().unwrap();
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(runtime.history().len(), 2);
    assert!(!std::sync::Arc::ptr_eq(
        &snapshot.history,
        &runtime.snapshot().history
    ));
    let saved = serde_json::to_string(&snapshot).unwrap();
    let restored = super::Runtime::restore(program, serde_json::from_str(&saved).unwrap()).unwrap();
    assert_eq!(restored.history().len(), 1);
}
