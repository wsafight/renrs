use super::*;
use crate::{Runtime, compile, parse_script};

fn runtime() -> Runtime {
    let script = "default bag = list(list(\"shared\"))\ndefault score = 0\nlabel start:\n    \"first\"\n    set score = 1\n    \"second\"\n    \"third\"";
    let mut runtime =
        Runtime::new(compile(&parse_script(script, "test.rns").unwrap()).unwrap()).unwrap();
    runtime.advance().unwrap();
    runtime.continue_story().unwrap();
    runtime.continue_story().unwrap();
    runtime
}

#[test]
fn roundtrip_preserves_shared_collections_and_rollback() {
    let runtime = runtime();
    let encoded = serde_json::to_vec(&runtime.snapshot()).unwrap();
    let snapshot: RuntimeSnapshot = serde_json::from_slice(&encoded).unwrap();
    assert!(Arc::ptr_eq(
        &snapshot.variables,
        &snapshot.rollback.last().unwrap().variables
    ));
    let Value::List(current) = &snapshot.variables["bag"] else {
        panic!()
    };
    let Value::List(first) = &snapshot.rollback[0].variables["bag"] else {
        panic!()
    };
    assert!(Arc::ptr_eq(current, first));
    assert_eq!(serde_json::to_vec(&snapshot).unwrap(), encoded);
    let mut restored = Runtime::restore(runtime.shared_program(), snapshot).unwrap();
    restored.rollback().unwrap();
    restored.rollback().unwrap();
    assert_eq!(restored.variables()["score"], Value::Integer(0));
}

#[test]
fn rejects_cycles_missing_references_and_oversized_rollback() {
    let encoded = serde_json::to_value(runtime().snapshot()).unwrap();
    let mut invalid = encoded.clone();
    invalid["values"][0] = serde_json::json!({"List": [0]});
    assert!(serde_json::from_value::<RuntimeSnapshot>(invalid).is_err());
    let mut invalid = encoded.clone();
    invalid["current"]["variables"] = serde_json::json!(99999);
    assert!(serde_json::from_value::<RuntimeSnapshot>(invalid).is_err());
    let mut invalid = encoded;
    invalid["rollback"] = serde_json::json!(vec![invalid["rollback"][0].clone(); 257]);
    assert!(serde_json::from_value::<RuntimeSnapshot>(invalid).is_err());
}

#[test]
fn repeated_large_values_are_encoded_once_across_checkpoints() {
    let mut runtime = runtime();
    let text = "payload".repeat(10000);
    let value = Value::List(Arc::new(vec![Value::String(text.clone())]));
    let mut snapshot = runtime.snapshot();
    Arc::make_mut(&mut snapshot.variables).insert("bag".to_owned(), value.clone());
    for checkpoint in &mut snapshot.rollback {
        Arc::make_mut(&mut checkpoint.variables).insert("bag".to_owned(), value.clone());
    }
    let json = serde_json::to_string(&snapshot).unwrap();
    assert_eq!(json.matches(&text).count(), 1);
    assert!(json.len() < text.len() + 10000);
    runtime.continue_story().unwrap();
}
