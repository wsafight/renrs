use super::{BTreeMap, Expr, RuntimeError, Value, is_text_tag};

pub(super) fn evaluate(
    expression: &Expr,
    variables: &BTreeMap<String, Value>,
    line: usize,
) -> Result<Value, RuntimeError> {
    velin_eval::evaluate(expression, variables, line)
        .map_err(|error| execution(error.line, error.message))
}

pub(super) fn interpolate(
    input: &str,
    variables: &BTreeMap<String, Value>,
    line: usize,
) -> Result<String, RuntimeError> {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                output.push('{');
                continue;
            }
            let mut name = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(current) => name.push(current),
                    None => return Err(execution(line, "unclosed `{` in dialogue interpolation")),
                }
            }
            let name = name.trim();
            if is_text_tag(name) {
                output.push('{');
                output.push_str(name);
                output.push('}');
                continue;
            }
            let value = variables.get(name).ok_or_else(|| {
                execution(line, format!("unknown interpolated variable `{name}`"))
            })?;
            match value {
                Value::Integer(value) => output.push_str(&value.to_string()),
                Value::Boolean(value) => output.push_str(if *value { "true" } else { "false" }),
                Value::String(value) => output.push_str(value),
                Value::List(_) | Value::Record(_) => output.push_str(
                    &serde_json::to_string(value)
                        .map_err(|error| execution(line, error.to_string()))?,
                ),
            }
        } else if ch == '}' && chars.peek() == Some(&'}') {
            chars.next();
            output.push('}');
        } else {
            output.push(ch);
        }
    }
    Ok(output)
}

pub(super) fn execution(line: usize, message: impl Into<String>) -> RuntimeError {
    RuntimeError::Execution {
        line,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{BinaryOp, UnaryOp};
    use std::sync::Arc;

    fn eval(expression: &Expr) -> Result<Value, RuntimeError> {
        evaluate(expression, &BTreeMap::new(), 1)
    }

    fn binary(left: Value, op: BinaryOp, right: Value) -> Expr {
        Expr::Binary {
            left: Box::new(Expr::Value(left)),
            op,
            right: Box::new(Expr::Value(right)),
        }
    }

    #[test]
    fn arithmetic_comparisons_and_string_concat_evaluate() {
        assert_eq!(
            eval(&binary(Value::Integer(1), BinaryOp::Add, Value::Integer(2))).unwrap(),
            Value::Integer(3)
        );
        assert_eq!(
            eval(&binary(
                Value::String("a".into()),
                BinaryOp::Add,
                Value::String("b".into())
            ))
            .unwrap(),
            Value::String("ab".into())
        );
        assert_eq!(
            eval(&binary(
                Value::Integer(3),
                BinaryOp::GreaterEqual,
                Value::Integer(3)
            ))
            .unwrap(),
            Value::Boolean(true)
        );
        assert!(
            eval(&binary(
                Value::Integer(1),
                BinaryOp::Divide,
                Value::Integer(0)
            ))
            .unwrap_err()
            .to_string()
            .contains("division by zero")
        );
        assert!(
            eval(&binary(
                Value::Integer(i64::MAX),
                BinaryOp::Add,
                Value::Integer(1)
            ))
            .unwrap_err()
            .to_string()
            .contains("overflow")
        );
    }

    #[test]
    fn boolean_ops_short_circuit_missing_variables() {
        let and = Expr::Binary {
            left: Box::new(Expr::Value(Value::Boolean(false))),
            op: BinaryOp::And,
            right: Box::new(Expr::Variable("missing".into())),
        };
        assert_eq!(eval(&and).unwrap(), Value::Boolean(false));
        let or = Expr::Binary {
            left: Box::new(Expr::Value(Value::Boolean(true))),
            op: BinaryOp::Or,
            right: Box::new(Expr::Variable("missing".into())),
        };
        assert_eq!(eval(&or).unwrap(), Value::Boolean(true));
        assert!(
            eval(&Expr::Unary {
                op: UnaryOp::Not,
                value: Box::new(Expr::Value(Value::Integer(1))),
            })
            .unwrap_err()
            .to_string()
            .contains("boolean")
        );
    }

    #[test]
    fn interpolates_values_literals_and_markup_tags() {
        let variables = BTreeMap::from([
            ("name".into(), Value::String("Mira".into())),
            ("score".into(), Value::Integer(2)),
            (
                "bag".into(),
                Value::List(Arc::new(vec![Value::String("key".into())])),
            ),
        ]);
        assert_eq!(
            interpolate("Hi {name}, {score}", &variables, 1).unwrap(),
            "Hi Mira, 2"
        );
        assert_eq!(
            interpolate("brace {{name}}", &variables, 1).unwrap(),
            "brace {name}"
        );
        assert_eq!(
            interpolate("{b}{name}{/b}", &variables, 1).unwrap(),
            "{b}Mira{/b}"
        );
        assert!(interpolate("{name", &variables, 1).is_err());
        assert!(interpolate("{missing}", &variables, 1).is_err());
        assert!(
            interpolate("bag {bag}", &variables, 1)
                .unwrap()
                .contains("key")
        );
    }
}
