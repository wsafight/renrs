use super::*;

#[test]
fn converts_interpolation_and_reports_fallthrough() {
    let converted = convert_script(
        "label start:\n    $ ready = True\n    \"Hello [ready]\"\nlabel second:\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(converted.output.contains("set ready = true"));
    assert!(converted.output.contains("\"Hello {ready}\""));
    assert!(
        converted
            .issues
            .iter()
            .any(|item| item.message.contains("fallthrough"))
    );
}

#[test]
fn converts_static_label_call_and_return_arguments() {
    let converted = convert_script(
        "label start:\n    call add(2, amount=True)\n    return\nlabel add(current, amount=1):\n    return current + amount\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(converted.output.contains("call add(2, amount=true)"));
    assert!(converted.output.contains("label add(current, amount=1):"));
    assert!(converted.output.contains("return current + amount"));
}

#[test]
fn rejects_variadic_label_parameters() {
    let converted = convert_script(
        "label start(*args):\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(matches!(
        converted.issues.as_slice(),
        [issue] if issue.kind == MigrationIssueKind::Unsupported
    ));
}

#[test]
fn preserves_static_audio_volume_clauses() {
    let converted = convert_script(
        "label start:\n    play music \"theme.ogg\" volume 0.4\n    play sound \"click.wav\" volume 0.25\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("play music \"theme.ogg\" volume 0.4")
    );
    assert!(
        converted
            .output
            .contains("play sound \"click.wav\" volume 0.25")
    );
}

#[test]
fn inlines_supported_static_transform_at_show_site() {
    let converted = convert_script(
        "transform reveal:\n    xalign 0.0\n    yalign 1.0\n    alpha 0.0\n    linear .5 alpha 1.0\nlabel start:\n    show hero at reveal\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(converted.output.contains(
        "show \"images/hero.png\" as hero at left\n    transform hero alpha 0\n    transform hero alpha 1 over 0.5 ease linear"
    ));
}
