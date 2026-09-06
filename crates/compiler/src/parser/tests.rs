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
            "label start:\n    show \"hero.png\" as hero at left layer 7\n    move hero to right over 0.5\n    transition fade 0.25",
            "test.rns",
        )
        .unwrap();
    assert!(matches!(
        script.labels["start"][0].kind,
        StatementKind::Show { layer: 7, .. }
    ));
    assert!(matches!(
        script.labels["start"][1].kind,
        StatementKind::Move { seconds: 0.5, .. }
    ));
    assert!(matches!(
        script.labels["start"][2].kind,
        StatementKind::Transition {
            kind: TransitionKind::Fade,
            seconds: 0.25
        }
    ));
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
    assert_eq!(script.label_parameters["add"], ["value"]);
    assert_eq!(
        script.labels["start"][1].id.as_ref().unwrap().as_str(),
        "line.result"
    );
}
