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

#[test]
fn layer_camera_survives_animation_sampling_snapshot_and_rollback() {
    let source = "layer effects order 50\nlabel start:\n    show \"front.png\" as front onlayer effects\n    \"Before\"\n    transform camera onlayer effects scale 2 x 100 over 2\n    \"After\"";
    let program = compile(&parse_script(source, "layer-camera.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert!(matches!(
        runtime.continue_story().unwrap(),
        WaitState::Effect {
            effect: VisualEffect::Transform { ref alias, .. }
        } if alias == "camera@effects"
    ));
    let sampled = super::sample(
        &crate::runtime::StageState::default(),
        &[vec![crate::syntax::AnimationStep::Transform {
            alias: "camera@effects".to_owned(),
            properties: crate::syntax::TransformProperties {
                x: Some(100.0),
                scale: Some(2.0),
                ..Default::default()
            },
            seconds: 2.0,
            easing: crate::syntax::Easing::Linear,
        }]],
        1.0,
    );
    assert!((sampled.layer_cameras["effects"].x - 50.0).abs() < 0.001);
    let snapshot = serde_json::from_slice(&serde_json::to_vec(&runtime.snapshot()).unwrap())
        .expect("layer camera snapshot should roundtrip");
    let mut restored = Runtime::restore(program, snapshot).unwrap();
    restored.continue_story().unwrap();
    assert!((restored.stage().layer_cameras["effects"].scale - 2.0).abs() < f32::EPSILON);
    restored.rollback().unwrap();
    assert_eq!(restored.stage().layer_cameras.len(), 0);
}

#[test]
fn background_transform_survives_snapshot_scene_reset_and_rollback() {
    let source = "label start:\n    scene \"one.png\"\n    \"Before\"\n    transform background scale 2 x 100 over 2\n    \"After\"\n    scene \"two.png\"\n    \"Reset\"";
    let program = compile(&parse_script(source, "background.rns").unwrap()).unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().background.as_deref(), Some("one.png"));
    assert_eq!(
        runtime.stage().background_transform,
        crate::syntax::TransformState::identity()
    );
    assert!(matches!(
        runtime.continue_story().unwrap(),
        WaitState::Effect {
            effect: VisualEffect::Transform { ref alias, .. }
        } if alias == crate::syntax::BACKGROUND_ALIAS
    ));

    let snapshot: RuntimeSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&runtime.snapshot()).unwrap()).unwrap();
    let mut restored = Runtime::restore(program, snapshot).unwrap();
    assert_eq!(restored.continue_story().unwrap(), WaitState::Dialogue);
    assert!((restored.stage().background_transform.scale - 2.0).abs() < f32::EPSILON);
    assert!((restored.stage().background_transform.x - 100.0).abs() < f32::EPSILON);

    assert_eq!(restored.continue_story().unwrap(), WaitState::Dialogue);
    assert_eq!(restored.stage().background.as_deref(), Some("two.png"));
    assert_eq!(
        restored.stage().background_transform,
        crate::syntax::TransformState::identity()
    );
    restored.rollback().unwrap();
    assert_eq!(restored.stage().background.as_deref(), Some("one.png"));
    assert!((restored.stage().background_transform.scale - 2.0).abs() < f32::EPSILON);
    assert!((restored.stage().background_transform.x - 100.0).abs() < f32::EPSILON);
}
