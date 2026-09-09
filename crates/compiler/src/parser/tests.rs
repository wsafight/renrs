use super::*;
use crate::syntax::StatementKind;

const SCRIPT: &str = r##"
config title "Test Story"
define e = character "Eileen" color "#ef6a6a"

label start:
    scene "images/room.png"
    e "Hello # this is dialogue"
    set courage = 1 + 2 * 3
    menu:
        "Open the door":
            jump outside
        "Wait":
            e "Not yet."

label outside:
    if courage >= 2:
        "A bright field waits outside."
    else:
        "The door stays closed."
    return
"##;

#[test]
fn parses_complete_script() {
    let script = parse_script(SCRIPT, "script.rns").unwrap();
    assert_eq!(script.title, "Test Story");
    assert_eq!(script.characters["e"].name, "Eileen");
    assert_eq!(script.labels.len(), 2);
    assert!(matches!(
        script.labels["start"][3].kind,
        StatementKind::Menu { .. }
    ));
}

#[test]
fn parses_narrated_and_spoken_menu_prompts() {
    let script = parse_script(
        "define e = character \"Eileen\"\nlabel start:\n    menu \"Choose\":\n        \"A\":\n            return\n        \"B\":\n            return\nlabel spoken:\n    menu e \"Ready?\":\n        \"Yes\":\n            return\n        \"No\":\n            return",
        "menu.rns",
    )
    .unwrap();
    let StatementKind::Menu {
        prompt: Some(prompt),
        ..
    } = &script.labels["start"][0].kind
    else {
        panic!("expected a narrated menu prompt");
    };
    assert_eq!(prompt.speaker, None);
    assert_eq!(prompt.text, "Choose");
    let StatementKind::Menu {
        prompt: Some(prompt),
        ..
    } = &script.labels["spoken"][0].kind
    else {
        panic!("expected a spoken menu prompt");
    };
    assert_eq!(prompt.speaker.as_deref(), Some("e"));
    assert_eq!(prompt.text, "Ready?");
}

#[test]
fn reports_bad_indentation() {
    let errors = parse_script("label start:\n   \"bad\"", "bad.rns").unwrap_err();
    assert!(errors[0].message.contains("multiple of four"));
}

#[test]
fn requires_start_label() {
    let errors = parse_script("label other:\n    return", "bad.rns").unwrap_err();
    assert!(errors.iter().any(|error| error.message.contains("start")));
}

#[test]
fn parses_layers_tweens_and_transitions() {
    let script = parse_script(
            "layer effects order 50\nlabel start:\n    show \"hero.png\" as hero at left onlayer effects zorder 7\n    clear effects\n    move hero to right over 0.5\n    transition fade 0.25",
            "test.rns",
        )
        .unwrap();
    assert_eq!(script.display_layers["effects"].order, 50);
    assert!(matches!(&script.labels["start"][0].kind,
        StatementKind::Show { layer: 7, display_layer, .. } if display_layer == "effects"));
    assert!(matches!(
        script.labels["start"][1].kind,
        StatementKind::ClearLayer { .. }
    ));
    assert!(matches!(
        script.labels["start"][2].kind,
        StatementKind::Move { seconds: 0.5, .. }
    ));
    assert!(matches!(
        script.labels["start"][3].kind,
        StatementKind::Transition {
            kind: TransitionKind::Fade,
            seconds: 0.25
        }
    ));
}

#[test]
fn parses_static_audio_volume_and_rejects_values_outside_the_mixer_range() {
    let script = parse_script(
        "label start:\n    play music \"theme.ogg\" volume 0.4 loop fadein 0.5\n    queue music \"next.ogg\" fadein 0.2 volume 0.7\n    play sound \"click.wav\" volume 0.25",
        "audio.rns",
    )
    .unwrap();
    assert!(matches!(
        script.labels["start"][0].kind,
        StatementKind::PlayMusic {
            repeat: true,
            fade_in: 0.5,
            volume: 0.4,
            ..
        }
    ));
    assert!(matches!(
        script.labels["start"][1].kind,
        StatementKind::QueueMusic {
            repeat: false,
            fade_in: 0.2,
            volume: 0.7,
            ..
        }
    ));
    assert!(matches!(
        script.labels["start"][2].kind,
        StatementKind::PlaySound { volume: 0.25, .. }
    ));
    let errors = parse_script(
        "label start:\n    play sound \"click.wav\" volume 1.1",
        "audio.rns",
    )
    .unwrap_err();
    assert!(errors[0].message.contains("between 0 and 1"));
}

#[test]
fn rejects_redeclared_builtin_and_duplicate_show_clauses() {
    let built_in = parse_script(
        "layer master order 10\nlabel start:\n    return",
        "test.rns",
    )
    .unwrap_err();
    assert!(built_in[0].message.contains("built in"));

    let duplicate = parse_script(
        "label start:\n    show \"hero.png\" zorder 1 layer 2",
        "test.rns",
    )
    .unwrap_err();
    assert!(
        duplicate[0]
            .message
            .contains("expected `as`, `at`, `zorder`, or `onlayer`")
    );
}

#[test]
fn parses_serializable_transform_properties_and_easing() {
    let script = parse_script(
            "label start:\n    show \"hero.png\" as hero\n    transform hero x 24 y -8 scale 0.8 rotate 12 alpha 0.7 anchor 0.5 1 crop 0 0 100 200 over 0.4 ease in_out\n",
            "test.rns",
        )
        .unwrap();
    let StatementKind::Transform {
        properties,
        seconds,
        easing,
        ..
    } = &script.labels["start"][1].kind
    else {
        panic!("expected transform");
    };
    assert_eq!(properties.x, Some(24.0));
    assert!((*seconds - 0.4).abs() < f32::EPSILON);
    assert_eq!(*easing, Easing::EaseInOut);
    assert!((properties.crop.flatten().unwrap().height - 200.0).abs() < f32::EPSILON);
}

#[test]
fn parses_project_declarations_calls_and_translation_ids() {
    let script = parse_script(
        r#"config title "Example Story"
config id "com.example.story"
default score = 1
image hero = "images/hero.png"
label start:
    call add(score)
    @id "line.result" alias "line.old" "Result: {_return}"
    show hero
label add(value):
    return value + 1
"#,
        "test.rns",
    )
    .unwrap();
    assert_eq!(script.project_id, "com.example.story");
    assert!(script.defaults.contains_key("score"));
    assert!(script.images.contains_key("hero"));
    assert_eq!(script.label_parameters["add"][0].name, "value");
    assert!(script.label_parameters["add"][0].default.is_none());
    assert_eq!(
        script.labels["start"][1].id.as_ref().unwrap().as_str(),
        "line.result"
    );
}

#[test]
fn parses_default_parameters_and_named_call_arguments() {
    let script = parse_script(
        r#"label start:
    call greet("Eileen", punctuation="?", excited=true)
label greet(name, greeting="Hello", punctuation="!", excited=false):
    return
"#,
        "test.rns",
    )
    .unwrap();
    let parameters = &script.label_parameters["greet"];
    assert_eq!(parameters.len(), 4);
    assert_eq!(parameters[0].name, "name");
    assert!(parameters[0].default.is_none());
    assert!(
        parameters[1..]
            .iter()
            .all(|parameter| parameter.default.is_some())
    );

    let StatementKind::Call { arguments, .. } = &script.labels["start"][0].kind else {
        panic!("expected call statement");
    };
    assert_eq!(arguments.len(), 3);
    assert_eq!(arguments[0].name, None);
    assert_eq!(arguments[1].name.as_deref(), Some("punctuation"));
    assert_eq!(arguments[2].name.as_deref(), Some("excited"));
}

#[test]
fn rejects_invalid_parameter_and_argument_ordering() {
    let parameter_errors = parse_script(
        "label start:\n    return\nlabel invalid(optional=1, required):\n    return",
        "test.rns",
    )
    .unwrap_err();
    assert!(
        parameter_errors[0]
            .message
            .contains("required parameters must precede")
    );

    let argument_errors = parse_script(
        "label start:\n    call target(second=2, 1)\nlabel target(first, second):\n    return",
        "test.rns",
    )
    .unwrap_err();
    assert!(
        argument_errors[0]
            .message
            .contains("positional arguments must precede")
    );

    let duplicate_parameter = parse_script(
        "label start:\n    return\nlabel invalid(value, value=1):\n    return",
        "test.rns",
    )
    .unwrap_err();
    assert!(
        duplicate_parameter[0]
            .message
            .contains("duplicate parameter")
    );

    let duplicate_argument = parse_script(
        "label start:\n    call target(value=1, value=2)\nlabel target(value):\n    return",
        "test.rns",
    )
    .unwrap_err();
    assert!(
        duplicate_argument[0]
            .message
            .contains("duplicate named argument")
    );
}

#[test]
fn parses_say_attributes_window_screens_and_named_transforms() {
    let script = parse_script(
        r##"define m = character "Mira" color "#ef8b72" image mira
image mira = "images/mira.png"
image mira_happy = "images/mira_happy.png"
transform shy:
    xalign 0.2
    yalign 1.0
label start:
    window hide
    show mira at shy
    window show
    m happy "Hello.{w} There.{nw}"
    show screen bag
    call screen examine
    hide screen bag
    play music "a.ogg" loop if_changed
    play sound "b.wav" loop volume 0.4
    stop sound
    stop voice
    transition push left 0.4
    timeline:
        transform mira x 8 over 0.05
        transform mira x -8 over 0.05
        repeat 2
    return
"##,
        "story.rns",
    )
    .unwrap();
    assert_eq!(script.characters["m"].image.as_deref(), Some("mira"));
    assert!(script.transforms.contains_key("shy"));
    assert!(matches!(
        script.labels["start"][0].kind,
        StatementKind::Window { visible: false }
    ));
    assert!(matches!(
        &script.labels["start"][3].kind,
        StatementKind::Dialogue { attributes, .. } if attributes == &["happy".to_owned()]
    ));
    assert!(matches!(
        script.labels["start"][4].kind,
        StatementKind::ShowScreen { .. }
    ));
    assert!(matches!(
        script.labels["start"][5].kind,
        StatementKind::CallScreen { .. }
    ));
}
