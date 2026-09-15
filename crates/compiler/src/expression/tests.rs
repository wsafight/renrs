use super::*;
use crate::syntax::{BinaryOp, UnaryOp, Value};

fn parse(source: &str) -> Expr {
    parse_expression(source, "test.rns", 1, 1).unwrap()
}

#[test]
fn respects_operator_precedence() {
    let parsed = parse("1 + 2 * 3 == 7 and not false");
    assert!(matches!(
        parsed,
        Expr::Binary {
            op: BinaryOp::And,
            ..
        }
    ));
}

#[test]
fn rejects_assignment() {
    let error = parse_expression("score = 2", "test.rns", 4, 9).unwrap_err();
    assert_eq!(error.line, 4);
    assert!(error.message.contains("=="));
}

#[test]
fn parses_unary_calls_strings_and_comparisons() {
    assert!(matches!(
        parse("-score"),
        Expr::Unary {
            op: UnaryOp::Negate,
            ..
        }
    ));
    assert!(matches!(
        parse(r#"list("a", 1)"#),
        Expr::Invoke {
            function: Builtin::List,
            ..
        }
    ));
    assert!(matches!(
        parse(r#""hello""#),
        Expr::Value(Value::String(text)) if text == "hello"
    ));
    assert!(matches!(
        parse("count >= 2"),
        Expr::Binary {
            op: BinaryOp::GreaterEqual,
            ..
        }
    ));
}

#[test]
fn rejects_unexpected_tokens_and_excessive_nesting() {
    let extra = parse_expression("1 2", "test.rns", 1, 1).unwrap_err();
    assert!(extra.message.contains("unexpected token"));
    let unexpected = parse_expression("1 $ 2", "test.rns", 1, 1).unwrap_err();
    assert!(unexpected.message.contains("unexpected character"));
    let nested = format!("{}1{}", "(".repeat(33), ")".repeat(33));
    let error = parse_expression(&nested, "test.rns", 1, 1).unwrap_err();
    assert!(error.message.contains("nesting exceeds 32"));
}

#[test]
fn preserves_renrs_strings_and_expression_serialization() {
    assert_eq!(
        parse(r#""[slot] and ]]""#),
        Expr::Value(Value::String("[slot] and ]]".into()))
    );
    assert_eq!(
        serde_json::to_value(parse("1 + 2")).unwrap(),
        serde_json::json!({
            "Binary": {
                "left": {"Value": 1},
                "op": "Add",
                "right": {"Value": 2}
            }
        })
    );
}

#[test]
fn bracket_compatibility_preserves_limits_and_diagnostic_columns() {
    let source = format!("\"{}\"", "[".repeat(velin_parse::MAX_EXPRESSION_BYTES - 2));
    assert!(matches!(
        parse(&source),
        Expr::Value(Value::String(text))
            if text.len() == velin_parse::MAX_EXPRESSION_BYTES - 2
    ));

    let error = parse_expression(r#""[slot]" + @"#, "test.rns", 2, 5).unwrap_err();
    assert_eq!(error.line, 2);
    assert_eq!(error.column, 16);

    assert_eq!(
        parse("\"\u{1}[slot]\u{2}\""),
        Expr::Value(Value::String("\u{1}[slot]\u{2}".into()))
    );
}

#[test]
fn applies_velin_checks_without_enabling_random_story_expressions() {
    let invalid = parse_expression(r#""text" + 1"#, "test.rns", 2, 5).unwrap_err();
    assert!(invalid.message.contains("cannot combine"));
    let condition = parse_condition("1", "test.rns", 3, 9).unwrap_err();
    assert!(condition.message.contains("condition expects boolean"));
    assert!(parse_condition("ready", "test.rns", 3, 9).is_ok());
    for source in ["random(1, 6)", "chance(50)"] {
        let error = parse_expression(source, "test.rns", 4, 5).unwrap_err();
        assert!(error.message.contains("not available in RenRS"));
    }
}
