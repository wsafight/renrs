use super::{Runtime, RuntimeError, WaitState};
use crate::{compile, parse_script};

#[test]
fn named_display_layers_order_clear_and_roundtrip() {
    let program = compile(
        &parse_script(
            "layer effects order 50\nlabel start:\n    show \"front.png\" as front onlayer effects zorder -2\n    show \"main.png\" as main zorder 20\n    \"Both\"\n    clear effects\n    \"Cleared\"",
            "layers.rns",
        )
        .unwrap(),
    )
    .unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().sprites.len(), 2);
    assert_eq!(runtime.stage().sprites[0].display_layer, "effects");
    assert_eq!(runtime.stage().sprites[0].display_order, 50);
    assert_eq!(runtime.stage().sprites[0].layer, -2);

    let mut restored = Runtime::restore(program, runtime.snapshot()).unwrap();
    assert_eq!(restored.stage().sprites[0].display_layer, "effects");
    assert_eq!(restored.continue_story().unwrap(), WaitState::Dialogue);
    assert_eq!(restored.stage().sprites.len(), 1);
    assert_eq!(restored.stage().sprites[0].alias, "main");
    restored.rollback().unwrap();
    assert_eq!(restored.stage().sprites.len(), 2);
}

#[test]
fn compatible_restore_checks_layers_held_by_dissolve_effects() {
    let original = compile(
        &parse_script(
            "layer effects order 50\nlabel start:\n    show \"front.png\" onlayer effects\n    scene \"room.png\"\n    @id \"blend\" transition dissolve 1",
            "layers.rns",
        )
        .unwrap(),
    )
    .unwrap();
    let edited = compile(
        &parse_script(
            "label start:\n    scene \"room.png\"\n    @id \"blend\" transition dissolve 1",
            "layers.rns",
        )
        .unwrap(),
    )
    .unwrap();
    let mut runtime = Runtime::new(original).unwrap();
    assert!(matches!(
        runtime.advance().unwrap(),
        WaitState::Effect { .. }
    ));
    assert!(matches!(
        Runtime::restore_compatible(edited, runtime.snapshot()),
        Err(RuntimeError::SavedDisplayLayerMissing(ref name)) if name == "effects"
    ));
}
