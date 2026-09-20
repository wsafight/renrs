use super::*;

#[test]
fn preserves_alias_and_converts_attached_transition() {
    let catalog = AssetCatalog::with_image("eileen happy", "images/eileen_happy.png");
    let supported = convert_image_statement(
        "show",
        "eileen happy as hero at left",
        &catalog,
        &TransformCatalog::default(),
    );
    assert!(matches!(
        supported,
        LineConversion::One(ref value) if value == "show \"images/eileen_happy.png\" as hero at left"
    ));
    let attached = convert_image_statement(
        "show",
        "eileen happy with dissolve",
        &catalog,
        &TransformCatalog::default(),
    );
    assert!(matches!(
        attached,
        LineConversion::Assumed { ref value, ref message }
            if value == "show \"images/eileen_happy.png\" as eileen at center\ntransition fade 0.5"
                && message.contains("mapped Ren'Py `dissolve`")
    ));
}

#[test]
fn converts_standard_display_layer_and_static_zorder() {
    let catalog = AssetCatalog::with_image("eileen happy", "images/eileen_happy.png");
    let converted = convert_image_statement(
        "show",
        "eileen happy onlayer transient zorder 20",
        &catalog,
        &TransformCatalog::default(),
    );
    assert!(matches!(
        converted,
        LineConversion::One(ref value)
            if value == "show \"images/eileen_happy.png\" as eileen at center onlayer transient zorder 20"
    ));
    assert!(matches!(
        convert_image_statement(
            "show",
            "eileen happy onlayer custom",
            &catalog,
            &TransformCatalog::default(),
        ),
        LineConversion::Unsupported { ref message, .. } if message.contains("standard static")
    ));
}
