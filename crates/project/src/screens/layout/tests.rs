use super::*;
use crate::screens::Screens;

#[test]
fn rejects_overflow_unknown_actions_and_interactive_huds() {
    for root in [
        r#"{"type":"column","style":"unknown","children":[{"type":"text","text":"child"}]}"#,
        r#"{"type":"column","children":[{"size":200,"type":"text","text":"too tall"}]}"#,
        r#"{"type":"button","text":"bad","action":"execute_python"}"#,
        r#"{"type":"button","text":"New","action":"new_game"}"#,
    ] {
        let json = serde_json::json!({"hud": {"bounds": {"x":0,"y":0,"width":300,"height":100},
            "root": serde_json::from_str::<serde_json::Value>(root).unwrap()}});
        assert!(Screens::from_slice(&serde_json::to_vec(&json).unwrap()).is_err());
    }
}

#[test]
fn places_fixed_and_flexible_rows_without_overlap() {
    let screens = Screens::from_slice(br#"{"hud":{"bounds":{"x":0,"y":100,"width":600,"height":100},"root":{"type":"row","gap":20,"children":[{"size":100,"type":"text","text":"one"},{"type":"text","text":"two"}]}}}"#).unwrap();
    let placed = layout(screens.hud.as_ref().unwrap()).unwrap();
    assert!((placed[0].bounds.width - 100.0).abs() < f32::EPSILON);
    assert!((placed[1].bounds.x - 120.0).abs() < f32::EPSILON);
    assert!((placed[1].bounds.width - 480.0).abs() < f32::EPSILON);
}

#[test]
fn nested_viewports_preserve_clip_ancestry_and_container_style() {
    let screens = Screens::from_slice(br#"{"styles":{"body":{"font_size":24}},"hud":{"bounds":{"x":20,"y":40,"width":600,"height":300},"root":{"type":"viewport","id":"outer","content_height":1000,"style":"body","child":{"type":"stack","children":[{"bounds":{"x":10,"y":600,"width":400,"height":200},"type":"viewport","id":"inner","content_height":800,"child":{"type":"text","text":"nested"}}]}}}}"#).unwrap();
    let placed = layout(screens.hud.as_ref().unwrap()).unwrap();
    assert_eq!(placed[0].viewports.len(), 2);
    assert_eq!(placed[0].style.as_deref(), Some("body"));
    assert!((placed[0].bounds.y - 640.0).abs() < f32::EPSILON);
}
