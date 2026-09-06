use renrs::{ProjectSource, Runtime, runtime::RuntimeSnapshot};
use std::fs;

#[test]
fn layers_resolve_on_show_and_restore_without_rechecking_current_conditions() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("body.png"), []).unwrap();
    fs::write(root.path().join("face.png"), []).unwrap();
    fs::write(root.path().join("actor.layers.json"), br#"{"width":400,"height":600,"layers":[{"path":"body.png"},{"path":"face.png","when":"smile"}]}"#).unwrap();
    fs::write(root.path().join("script.rns"), "default smile = true\nlabel start:\n    show \"actor.layers.json\" as actor\n    \"First\"\n    set smile = false\n    show \"actor.layers.json\" as actor\n    \"Second\"").unwrap();
    let program = ProjectSource::open(root.path()).unwrap().compile().unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    assert_eq!(runtime.stage().sprites[0].image_paths().len(), 2);
    runtime.continue_story().unwrap();
    assert_eq!(runtime.stage().sprites[0].image_paths(), ["body.png"]);
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
