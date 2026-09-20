use renrs::{ProjectSource, Runtime, runtime::RuntimeSnapshot, syntax::Value};
use std::fs;

#[test]
fn layers_refresh_when_story_or_screen_variables_change_and_restore() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("body.png"), []).unwrap();
    fs::write(root.path().join("face.png"), []).unwrap();
    fs::write(root.path().join("actor.layers.json"), br#"{"width":400,"height":600,"layers":[{"path":"body.png"},{"path":"face.png","when":"smile"}]}"#).unwrap();
    fs::write(root.path().join("script.rns"), "default smile = true\nlabel start:\n    show \"actor.layers.json\" as actor\n    \"First\"\n    set smile = false\n    \"Second\"").unwrap();
    let program = ProjectSource::open(root.path()).unwrap().compile().unwrap();
    let encoded = serde_json::to_value(&program).unwrap();
    let layer = &encoded["layered_images"]["actor.layers.json"]["layers"][1];
    assert!(layer.get("condition").is_some());
    assert!(layer.get("when").is_none());
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    assert_eq!(runtime.stage().sprites[0].image_paths().len(), 2);
    runtime.continue_story().unwrap();
    assert_eq!(runtime.stage().sprites[0].image_paths(), ["body.png"]);
    runtime
        .set_screen_variable("smile", Value::Boolean(true))
        .unwrap();
    assert_eq!(runtime.stage().sprites[0].image_paths().len(), 2);
    let snapshot: RuntimeSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&runtime.snapshot()).unwrap()).unwrap();
    let mut restored = Runtime::restore(program.clone(), snapshot).unwrap();
    restored.rollback().unwrap();
    assert_eq!(restored.stage().sprites[0].image_paths().len(), 2);
    fs::write(
        root.path().join("actor.layers.json"),
        br#"{"width":401,"height":600,"layers":[{"path":"body.png"}]}"#,
    )
    .unwrap();
    assert_ne!(
        program.fingerprint,
        ProjectSource::open(root.path())
            .unwrap()
            .compile()
            .unwrap()
            .fingerprint
    );
}

#[test]
fn grouped_layers_choose_the_last_matching_variant() {
    let root = tempfile::tempdir().unwrap();
    for path in [
        "body.png",
        "neutral.png",
        "smile.png",
        "wink.png",
        "hat.png",
    ] {
        fs::write(root.path().join(path), []).unwrap();
    }
    fs::write(
        root.path().join("actor.layers.json"),
        br#"{"width":400,"height":600,"layers":[
            {"path":"body.png"},
            {"path":"neutral.png","group":"face"},
            {"path":"smile.png","group":"face","when":"smiling"},
            {"path":"wink.png","group":"face","when":"winking"},
            {"path":"hat.png","when":"wearing_hat"}
        ]}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("script.rns"),
        "default smiling = true\ndefault winking = false\ndefault wearing_hat = true\nlabel start:\n    show \"actor.layers.json\" as actor\n    \"First\"\n    set smiling = false\n    set winking = true\n    \"Second\"",
    )
    .unwrap();

    let program = ProjectSource::open(root.path()).unwrap().compile().unwrap();
    let mut runtime = Runtime::new(program).unwrap();
    runtime.advance().unwrap();
    assert_eq!(
        runtime.stage().sprites[0].image_paths(),
        ["body.png", "smile.png", "hat.png"]
    );
    runtime.continue_story().unwrap();
    assert_eq!(
        runtime.stage().sprites[0].image_paths(),
        ["body.png", "wink.png", "hat.png"]
    );
}

#[test]
fn grouped_layers_reject_invalid_group_names() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("body.png"), []).unwrap();
    fs::write(
        root.path().join("actor.layers.json"),
        br#"{"width":100,"height":100,"layers":[{"path":"body.png","group":"face/name"}]}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("script.rns"),
        "label start:\n    show \"actor.layers.json\" as actor\n    return\n",
    )
    .unwrap();
    let errors = ProjectSource::open(root.path())
        .unwrap()
        .compile()
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("layer group names"))
    );
}
