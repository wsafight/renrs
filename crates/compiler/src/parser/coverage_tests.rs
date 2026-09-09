use super::*;
use crate::syntax::{Easing, Position, StatementKind, TransitionKind};

fn parse(source: &str) -> crate::syntax::Script {
    parse_script(source, "cover.rns").unwrap()
}

fn errors(source: &str) -> Vec<String> {
    parse_script(source, "cover.rns")
        .unwrap_err()
        .into_iter()
        .map(|item| item.message)
        .collect()
}

#[test]
fn parses_declarations_audio_display_and_transitions() {
    let script = parse(
        r##"config title "Coverage"
config id "org.cover.test"
define e = character "Eileen" color "#ff0000" image hero
default score = 0
image hero = "images/hero.png"
layer effects order 40
transform shy:
    xpos 1
    ypos 2
    xalign 0.2
    yalign 1.0
    scale 1.2
    rotate 5
    alpha 0.9
    anchor 0.5 1
    crop 0 0 10 10
transform clean:
    uncrop
label start:
    window hide
    scene hero
    show hero as hero at shy zorder 3 onlayer effects
    hide hero
    clear effects
    nvl clear
    window show
    play music "a.ogg" loop fadein 0.1 volume 0.5 if_changed
    queue music "b.ogg" fadein 0.2 volume 0.4
    play sound "c.wav" loop volume 0.3
    queue sound "d.wav" volume 0.2
    voice "e.wav"
    stop music fadeout 0.1
    stop sound fadeout 0.2
    stop voice
    move hero to left over 0.1
    transform hero xpos 4 ypos 5 xalign 0.1 yalign 0.2 scale 1.1 rotate 2 alpha 0.8 anchor 0.4 0.9 crop 1 1 8 8 over 0.2 ease out
    transform hero uncrop over 0.1 ease in
    transition dissolve 0.2
    transition wipe left 0.2
    transition wipe right 0.2
    transition punch h 0.2
    transition punch v 0.2
    pause 0
    timeline:
        transform hero x 1 over 0.05
        move hero to right over 0.05
        pause 0.05
        repeat 2
    return
"##,
    );
    assert_eq!(script.title, "Coverage");
    assert_eq!(script.project_id, "org.cover.test");
    assert!(script.transforms.contains_key("shy"));
    assert!(script.transforms.contains_key("clean"));
    assert!(matches!(
        script.labels["start"][0].kind,
        StatementKind::Window { visible: false }
    ));
    assert!(matches!(
        &script.labels["start"][2].kind,
        StatementKind::Show {
            position: Position::Center,
            at_transform: Some(name),
            ..
        } if name == "shy"
    ));
    assert!(
        script.labels["start"]
            .iter()
            .any(|item| { matches!(item.kind, StatementKind::StopVoice { .. }) })
    );
    assert!(script.labels["start"].iter().any(|item| {
        matches!(
            item.kind,
            StatementKind::Transition {
                kind: TransitionKind::Dissolve,
                ..
            }
        )
    }));
    assert!(script.labels["start"].iter().any(|item| {
        matches!(
            item.kind,
            StatementKind::Transform {
                easing: Easing::EaseIn,
                ..
            }
        )
    }));
}

#[test]
fn reports_duplicate_top_level_and_invalid_clauses() {
    let found = errors(
        "config title \"A\"\nconfig title \"B\"\nconfig id \"org.a\"\nconfig id \"org.b\"\ndefault score = 1\ndefault score = 2\nimage hero = \"a.png\"\nimage hero = \"b.png\"\nlayer fx order 1\nlayer fx order 2\n    indented\nlabel start:\n    return\n",
    );
    assert!(
        found.iter().any(|item| item.contains("more than once")
            || item.contains("declared")
            || item.contains("indented")),
        "{found:?}"
    );
}

#[test]
fn rejects_invalid_audio_window_repeat_and_transform_values() {
    assert!(errors("label start:\n    play video \"a.ogg\"\n")[0].contains("music"));
    assert!(errors("label start:\n    queue video \"a.ogg\"\n")[0].contains("music"));
    assert!(errors("label start:\n    stop lights\n")[0].contains("music"));
    assert!(errors("label start:\n    window maybe\n")[0].contains("show"));
    assert!(errors("label start:\n    repeat 0\n")[0].contains("1..16"));
    assert!(errors("label start:\n    repeat 99\n")[0].contains("1..16"));
    assert!(errors("label start:\n    pause -1\n")[0].contains("non-negative"));
    assert!(errors("label start:\n    pause nope\n")[0].contains("number"));
    assert!(errors("label start:\n    video \"a.mp4\"\n")[0].contains("over"));
    assert!(errors("label start:\n    transform hero xalign 2\n")[0].contains("xalign"));
    assert!(errors("label start:\n    transform hero scale 0\n")[0].contains("scale"));
    assert!(
        errors("transform left:\n    x 1\nlabel start:\n    return\n")[0].contains("cannot be")
    );
    assert!(errors("label start:\n    transition bounce 0.2\n")[0].contains("fade"));
    assert!(errors("label start:\n    timeline:\n        jump start\n")[0].contains("timeline"));
    assert!(errors("label start:\n    show \"a.png\" maybe\n")[0].contains("as"));
    assert!(errors("label start:\n    jump\n")[0].contains("label"));
    assert!(
        errors("label start:\n    hide\n")[0].contains("alias")
            || errors("label start:\n    hide\n")[0].contains("expected")
    );
    assert!(errors("label start:\n    set x =\n")[0].contains("expression"));
    assert!(
        errors("label start:\n    menu:\n        \"Only\":\n            return\n")[0]
            .contains("two options")
    );
    assert!(
        errors("label start:\n    transform hero ease nope\n")[0].contains("easing")
            || errors("label start:\n    transform hero ease nope\n")[0].contains("property")
    );
    assert!(errors("label start:\n    transition push up 0.2\n")[0].contains("left"));
    assert!(errors("label start:\n    move hero left over 0.1\n")[0].contains("`to`"));
}

#[test]
fn compiles_named_show_transform_and_unknown_transform() {
    use crate::compile;
    let program = compile(&parse(
        "transform shy:\n    xalign 0.2\nlabel start:\n    show \"hero.png\" as hero at shy\n    return\n",
    ))
    .unwrap();
    assert!(program.instructions.iter().any(|item| matches!(
        item.kind,
        crate::compiler::InstructionKind::Transform { .. }
    )));
    let error = compile(&parse(
        "label start:\n    show \"hero.png\" as hero at missing\n    return\n",
    ))
    .unwrap_err();
    assert!(error.to_string().contains("missing"));
}
