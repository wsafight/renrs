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
fn reports_dynamic_jump_and_call_targets_with_stable_codes() {
    let converted = convert_script(
        "label start:\n    jump route_name\n    jump expression next_route\n    call expression destination\n    call helper if ready\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(converted.output.contains("jump route_name"));
    assert!(
        converted
            .output
            .contains("# TODO migration: jump expression next_route")
    );
    assert!(
        converted
            .output
            .contains("# TODO migration: call expression destination")
    );
    assert!(
        converted
            .output
            .contains("# TODO migration: call helper if ready")
    );

    let diagnostics = converted
        .issues
        .iter()
        .map(|issue| (issue.code.as_str(), issue.message.as_str()))
        .collect::<Vec<_>>();
    assert!(
        diagnostics.iter().any(|(code, message)| {
            *code == "jump_target_dynamic" && message.contains("expression next_route")
        }),
        "{diagnostics:?}"
    );
    assert!(diagnostics.iter().any(|(code, message)| {
        *code == "call_target_dynamic" && message.contains("expression destination")
    }));
    assert!(diagnostics.iter().any(|(code, message)| {
        *code == "call_clause_unsupported" && message.contains("call `helper if ready`")
    }));
}

#[test]
fn reports_unknown_identifier_statements_as_custom_with_source_context() {
    let converted = convert_script(
        "label start:\n    quest_marker 3\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(
        converted
            .output
            .contains("# TODO migration: quest_marker 3")
    );
    let issue = converted
        .issues
        .iter()
        .find(|issue| issue.code == "custom_statement_unsupported")
        .expect("unknown identifier statement should have a stable custom code");
    assert!(issue.message.contains("`quest_marker`"));
    assert!(issue.message.contains("quest_marker 3"));
}

#[test]
fn keeps_renpy_test_language_on_the_generic_baseline_code() {
    let converted = convert_script(
        "testsuite global:\n    pass\ntestcase sample:\n    pass\n",
        "testcases.rpy",
        &AssetCatalog::empty(),
    );
    assert_eq!(converted.issues.len(), 2, "{:?}", converted.issues);
    assert!(
        converted
            .issues
            .iter()
            .all(|issue| issue.code == "statement_unsupported")
    );
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

#[test]
fn inlines_finite_atl_repeat_as_recompilable_cycles() {
    let converted = convert_script(
        "transform pulse:\n    alpha 0\n    linear .1 alpha 1\n    repeat 2\nlabel start:\n    show hero at pulse\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(converted.output.contains(
        "show \"images/hero.png\" as hero at center\n    transform hero alpha 0\n    transform hero alpha 1 over 0.1 ease linear\n    transform hero alpha 0\n    transform hero alpha 1 over 0.1 ease linear"
    ));
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("finite ATL repeat should parse after migration");
    let program =
        crate::compile(&parsed).expect("finite ATL repeat should compile after migration");
    let mut runtime = crate::Runtime::new(program).expect("migrated program should run");
    for (index, expected_alpha) in [0.0, 1.0, 0.0, 1.0].into_iter().enumerate() {
        let waiting = if index == 0 {
            runtime.advance()
        } else {
            runtime.continue_story()
        }
        .expect("cycle should produce a transform effect");
        assert!(matches!(waiting, crate::WaitState::Effect { .. }));
        assert!((runtime.stage().sprites[0].transform.alpha - expected_alpha).abs() < f32::EPSILON);
    }
}

#[test]
fn migrates_static_image_expression_to_a_recompilable_show() {
    let converted = convert_script(
        "label start:\n    show expression \"images/hero.png\" as hero at left\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("show \"images/hero.png\" as hero at left")
    );
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("static image expression should parse after migration");
    crate::compile(&parsed).expect("static image expression should compile after migration");
}

#[test]
fn migrates_single_quoted_image_expression_to_a_recompilable_show() {
    let converted = convert_script(
        "label start:\n    show expression 'images/hero.png' as hero at right\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("show \"images/hero.png\" as hero at right")
    );
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("single-quoted image expression should parse after migration");
    crate::compile(&parsed).expect("single-quoted image expression should compile after migration");
}

#[test]
fn migrates_static_image_constructor_to_a_recompilable_show() {
    let converted = convert_script(
        "label start:\n    show expression Image(\"images/hero.png\") as hero at left\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("show \"images/hero.png\" as hero at left")
    );
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("static image constructor should parse after migration");
    crate::compile(&parsed).expect("static image constructor should compile after migration");
}

#[test]
fn migrates_bounded_static_transform_image_expression() {
    let converted = convert_script(
        "label start:\n    show expression Transform(\"images/hero.png\", zoom=1.2, rotate=15, xoffset=8) as hero\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(converted.output.contains(
        "show \"images/hero.png\" as hero at center\n    transform hero scale 1.2\n    transform hero rotate 15\n    transform hero x 8"
    ));
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("static Transform expression should parse after migration");
    crate::compile(&parsed).expect("static Transform expression should compile after migration");
}

#[test]
fn keeps_scene_transform_image_expressions_explicitly_unsupported() {
    let converted = convert_script(
        "label start:\n    scene expression Transform(\"images/room.png\", zoom=1.1)\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("room", "images/room.png"),
    );
    assert!(converted.issues.iter().any(|issue| {
        issue.kind == MigrationIssueKind::Unsupported
            && issue.code == "image_expression_transform_unsupported"
    }));
    assert!(
        converted.output.contains(
            "# TODO migration: scene expression Transform(\"images/room.png\", zoom=1.1)"
        )
    );
}

#[test]
fn migrates_static_at_image_expression_through_a_named_transform() {
    let converted = convert_script(
        "transform focus:\n    zoom 1.1\nlabel start:\n    show expression At(\"images/hero.png\", focus) as hero\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("show \"images/hero.png\" as hero at center\n    transform hero scale 1.1")
    );
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("static At expression should parse after migration");
    crate::compile(&parsed).expect("static At expression should compile after migration");
}

#[test]
fn keeps_dynamic_image_expressions_as_structured_diagnostics() {
    let converted = convert_script(
        "label start:\n    show expression image_path as hero\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(
        converted
            .output
            .contains("# TODO migration: show expression image_path as hero")
    );
    assert!(converted.issues.iter().any(|issue| {
        issue.kind == MigrationIssueKind::Unsupported
            && issue.code == "image_expression_dynamic"
            && issue.message.contains("single- or double-quoted static")
    }));
}

#[test]
fn reports_static_image_expression_boundaries_with_stable_codes() {
    let converted = convert_script(
        "label start:\n    show expression Image(\"images/missing.png\") as hero\n    show expression \"images/hero.png\"\n    show expression Image(\"images/hero.png\", xalign=0.5) as hero\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    let codes = converted
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"image_expression_resource_missing"));
    assert!(codes.contains(&"image_expression_alias_required"));
    assert!(codes.contains(&"image_expression_constructor_unsupported"));
}

#[test]
fn reports_unbounded_atl_repeat_without_guessing() {
    let converted = convert_script(
        "transform loop:\n    alpha 0\n    repeat\nlabel start:\n    show hero at loop\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(
        converted
            .output
            .contains("# TODO migration: transform loop:")
    );
    assert!(converted.issues.iter().any(|issue| {
        issue.kind == MigrationIssueKind::Unsupported
            && issue.code == "atl_repeat_unsupported"
            && issue.message.contains("unbounded ATL")
    }));
}

#[test]
fn specializes_static_parameterized_atl_calls_for_show_and_camera() {
    let converted = convert_script(
        "transform move_by(distance):\n    xoffset distance\nlabel start:\n    show hero at move_by(20)\n    camera master at move_by(20)\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("show \"images/hero.png\" as hero at center\n    transform hero x 20")
    );
    assert!(converted.output.contains("transform camera x 20"));
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("specialized parameterized ATL should parse after migration");
    crate::compile(&parsed).expect("specialized parameterized ATL should compile after migration");
}

#[test]
fn reports_dynamic_parameterized_atl_calls_without_guessing() {
    let converted = convert_script(
        "transform move_by(distance):\n    xoffset distance\nlabel start:\n    show hero at move_by(distance)\n    camera at move_by(1, 2)\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("hero", "images/hero.png"),
    );
    let diagnostics = converted
        .issues
        .iter()
        .filter(|issue| issue.code == "atl_parameters_unsupported")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("# TODO migration: show hero at move_by(distance)")
    );
    assert!(
        converted
            .output
            .contains("# TODO migration: camera at move_by(1, 2)")
    );
}

#[test]
fn keeps_parameterized_scene_calls_explicitly_unsupported() {
    let converted = convert_script(
        "transform move_by(distance):\n    xoffset distance\nlabel start:\n    scene room at move_by(20)\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("room", "images/room.png"),
    );
    assert!(converted.issues.iter().any(|issue| {
        issue.kind == MigrationIssueKind::Unsupported && issue.code == "atl_parameters_unsupported"
    }));
    assert!(
        converted
            .output
            .contains("# TODO migration: scene room at move_by(20)")
    );
}

#[test]
fn inlines_static_master_camera_and_directional_easing() {
    let converted = convert_script(
        "transform focus:\n    zoom 1.1\n    easein .4 xoffset -20\n    pause .2\nlabel start:\n    camera master at focus\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(matches!(
        converted.issues.as_slice(),
        [issue] if issue.kind == MigrationIssueKind::Assumption
            && issue.message.contains("blocking RenRS pause")
    ));
    assert!(converted.output.contains(
        "transform camera scale 1.1\n    transform camera x -20 over 0.4 ease in\n    pause 0.2"
    ));
}

#[test]
fn migrates_attached_static_transitions_into_recompilable_script() {
    let converted = convert_script(
        "label start:\n    scene room with dissolve\n    show room as narrator with pushleft\n    return\n",
        "script.rpy",
        &AssetCatalog::with_image("room", "images/room.png"),
    );
    assert_eq!(converted.issues.len(), 1, "{:?}", converted.issues);
    assert_eq!(converted.issues[0].code, "transition_duration_assumed");
    assert!(converted.output.contains(
        "scene \"images/room.png\"\n    transition fade 0.5\n    show \"images/room.png\" as narrator at center\n    transition push left 0.5"
    ));
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("attached transitions should parse after migration");
    crate::compile(&parsed).expect("attached transitions should compile after migration");
}

#[test]
fn reports_non_master_layer_cameras_without_guessing() {
    let converted = convert_script(
        "transform focus:\n    zoom 1.1\nlabel start:\n    camera screens at focus\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(converted.issues.is_empty(), "{:?}", converted.issues);
    assert!(
        converted
            .output
            .contains("transform camera onlayer screens scale 1.1")
    );
    let parsed = crate::parse_script(&converted.output, "migrated.rns")
        .expect("standard layer camera should parse after migration");
    crate::compile(&parsed).expect("standard layer camera should compile after migration");
}

#[test]
fn keeps_custom_layer_cameras_explicitly_unsupported() {
    let converted = convert_script(
        "transform focus:\n    zoom 1.1\nlabel start:\n    camera custom at focus\n    return\n",
        "script.rpy",
        &AssetCatalog::empty(),
    );
    assert!(matches!(
        converted.issues.as_slice(),
        [issue] if issue.kind == MigrationIssueKind::Unsupported
            && issue.code == "display_statement_unsupported"
    ));
}
