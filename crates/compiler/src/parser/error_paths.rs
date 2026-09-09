use super::*;

fn errors(source: &str) -> Vec<String> {
    parse_script(source, "err.rns")
        .unwrap_err()
        .into_iter()
        .map(|item| item.message)
        .collect()
}

fn has(source: &str, needle: &str) {
    let found = errors(source);
    assert!(
        found.iter().any(|item| item.contains(needle)),
        "{found:?} does not contain {needle}"
    );
}

#[test]
fn reports_top_level_declaration_errors() {
    has("config\nlabel start:\n    return\n", "title");
    has(
        "config id \"Not Valid\"\nlabel start:\n    return\n",
        "reverse-DNS",
    );
    has(
        "config name \"X\"\nlabel start:\n    return\n",
        "config title",
    );
    has("default\nlabel start:\n    return\n", "variable");
    has("default score =\nlabel start:\n    return\n", "expression");
    has("image\nlabel start:\n    return\n", "image name");
    has("image hero\nlabel start:\n    return\n", "expected");
    has("layer\nlabel start:\n    return\n", "display layer");
    has("layer fx\nlabel start:\n    return\n", "order");
    has("layer fx order\nlabel start:\n    return\n", "integer");
    has("define\nlabel start:\n    return\n", "character id");
    has(
        "define e = npc \"E\"\nlabel start:\n    return\n",
        "character",
    );
    has("label:\n    return\n", "label name");
    has("label start:\n", "empty block");
    has(
        "label start:\n    return\nlabel start:\n    return\n",
        "more than once",
    );
    has(
        "transform shy:\n    x 1\ntransform shy:\n    y 1\nlabel start:\n    return\n",
        "transform `shy`",
    );
    has(
        "define e = character \"A\"\ndefine e = character \"B\"\nlabel start:\n    return\n",
        "character `e`",
    );
    has(
        "scene \"x.png\"\nlabel start:\n    return\n",
        "expected `config`",
    );
}

#[test]
fn reports_statement_and_transform_errors() {
    has("label start:\n    @foo \"Hi\"\n", "`id`");
    has(
        "label start:\n    transform hero over -1 x 1\n",
        "non-negative",
    );
    has(
        "label start:\n    transform camera xalign 0.5\n",
        "camera supports",
    );
    has("label start:\n    transform hero\n", "property");
    has("label start:\n    transform hero x inf\n", "finite");
    has("label start:\n    transform hero yalign 2\n", "yalign");
    has("label start:\n    transform hero alpha 2\n", "alpha");
    has("label start:\n    transform hero anchor 2 0\n", "anchors");
    has("label start:\n    transform hero crop -1 0 1 1\n", "crop");
    has(
        "label start:\n    play music \"a.ogg\" fadein -1\n",
        "fadein",
    );
    has("label start:\n    play music \"a.ogg\" volume\n", "volume");
    has(
        "label start:\n    play music \"a.ogg\" volume 2\n",
        "between 0 and 1",
    );
    has("label start:\n    stop\n", "music");
    has("label start:\n    clear\n", "display layer");
    has("label start:\n    scene\n", "image name");
    has("label start:\n    show\n", "image name");
    has("label start:\n    show \"a.png\" as\n", "alias");
    has("label start:\n    show \"a.png\" at\n", "position");
    has("label start:\n    show \"a.png\" zorder\n", "z-order");
    has(
        "label start:\n    show \"a.png\" onlayer\n",
        "display layer",
    );
    has("label start:\n    hide hero extra\n", "expected");
    has("label start:\n    video \"a.mp4\" over -1\n", "duration");
    has("label start:\n    transition wipe up 0.2\n", "left");
    has("label start:\n    transition punch z 0.2\n", "`h`");
    has("label start:\n    call\n", "label");
    has("label start:\n    window show extra\n", "expected");
    has(
        "transform shy:\nlabel start:\n    return\n",
        "at least one property",
    );
    has(
        "transform shy:\n    x 1\n        y 1\nlabel start:\n    return\n",
        "indentation",
    );
    has(
        "transform shy:\n    x 1\n    x 2\nlabel start:\n    return\n",
        "duplicate",
    );
}

#[test]
fn reports_dialogue_attribute_and_menu_errors() {
    has(
        "define e = character \"E\"\nlabel start:\n    e a b c d e f g h i \"Hi\"\n",
        "8 attributes",
    );
    has(
        "label start:\n    if true:\n        \"A\"\n    elif false:\n    return\n",
        "elif branch cannot be empty",
    );
    has(
        "label start:\n    if true:\n        \"A\"\n    else:\n    return\n",
        "else branch cannot be empty",
    );
    has("label start:\n    parallel:\n        \"no\"\n", "timeline");
    has("label start:\n    nvl\n", "nvl on");
}
