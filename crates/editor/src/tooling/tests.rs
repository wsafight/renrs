use super::*;
use crate::parse_script;

#[test]
fn formatter_preserves_comments_and_indentation() {
    let source = "\nlabel start:   \n    # Keep me   \n    \"Hello\"\t\n\n\n\n";
    assert_eq!(
        format_source(source),
        "label start:\n    # Keep me\n    \"Hello\"\n"
    );
}

#[test]
fn extracts_document_symbols() {
    let symbols =
        document_symbols("define e = character \"Eileen\"\nlabel start:\n    \"Hello\"\n");
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].kind, SymbolKind::Character);
    assert_eq!(symbols[1].name, "start");
}

#[test]
fn finds_cross_file_symbol_roles_without_reading_strings_or_comments() {
    let source = "define e = character \"Eileen\"\nimage hero = \"hero.png\"\ndefault score = 1\nlabel start(value):\n    show hero\n    e \"jump hidden\"\n    set score = score + value\n    call ending\n# jump ignored\nlabel ending:\n    return\n";
    let occurrences = symbol_occurrences(source);
    assert!(occurrences.iter().any(|item| {
        item.name == "ending"
            && item.kind == SymbolKind::Label
            && item.role == SymbolRole::Reference
    }));
    assert!(occurrences.iter().any(|item| {
        item.name == "hero" && item.kind == SymbolKind::Image && item.role == SymbolRole::Reference
    }));
    assert!(!occurrences.iter().any(|item| item.name == "hidden"));
    let score = source.lines().nth(6).unwrap().find("score").unwrap();
    assert_eq!(symbol_at(source, 6, score).unwrap().name, "score");
}

#[test]
fn graph_contains_nested_jump_and_call_edges() {
    let script = parse_script(
        "label start:\n    call intro\n    menu:\n        \"A\":\n            jump end\n        \"B\":\n            return\nlabel intro:\n    return\nlabel end:\n    return",
        "test.rns",
    )
    .unwrap();
    let graph = story_graph(&script);
    assert!(graph.contains("\"start\" -> \"intro\" [label=\"call\"]"));
    assert!(graph.contains("\"start\" -> \"end\" [label=\"jump\"]"));
}

#[test]
fn references_cover_anchors_arguments_conditions_and_unicode_interpolation() {
    let source = "default score = 1\ndefine e = character \"E\"\nlabel start(value):\n    @id \"line\" e \"你好 {score} {{literal}} {b}bold{/b}\"\n    call ending(score)\n    menu:\n        \"Go\" id \"choice\" if score > 0:\n            return value\n";
    let items = symbol_occurrences(source);
    assert_eq!(items.iter().filter(|item| item.name == "score").count(), 4);
    assert_eq!(items.iter().filter(|item| item.name == "e").count(), 2);
    assert!(
        items
            .iter()
            .any(|item| item.name == "value" && item.role == SymbolRole::Definition)
    );
    assert!(
        !items
            .iter()
            .any(|item| matches!(item.name.as_str(), "literal" | "bold" | "b"))
    );
    let line = source.lines().nth(3).unwrap();
    let column = line[..line.find("score").unwrap()].encode_utf16().count();
    assert_eq!(symbol_at(source, 3, column).unwrap().name, "score");
}
