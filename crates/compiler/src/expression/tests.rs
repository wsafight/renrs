use super::*;

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
