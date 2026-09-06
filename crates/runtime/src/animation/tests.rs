use crate::runtime::{RuntimeSnapshot, VisualEffect};
use crate::{Runtime, WaitState, compile, parse_script};

#[test]
fn camera_works_without_sprites_and_survives_disk_roundtrip_and_rollback() {
    let source =
        "label start:\n    \"Before\"\n    transform camera scale 2 x 100 over 2\n    \"After\"";
    let program = compile(&parse_script(source, "camera.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    runtime.advance().unwrap();
    assert!(matches!(
        runtime.continue_story().unwrap(),
        WaitState::Effect {
            effect: VisualEffect::Transform { .. }
        }
    ));
    let snapshot: RuntimeSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&runtime.snapshot()).unwrap()).unwrap();
    let mut runtime = Runtime::restore(program, snapshot).unwrap();
    runtime.continue_story().unwrap();
    assert!((runtime.stage().camera.scale - 2.0).abs() < f32::EPSILON);
    runtime.rollback().unwrap();
    assert!((runtime.stage().camera.scale - 1.0).abs() < f32::EPSILON);
}
