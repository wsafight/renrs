use super::{BTreeMap, BinaryOp, Expr, RuntimeError, UnaryOp, Value, is_text_tag};

pub(super) fn evaluate(
    expression: &Expr,
    variables: &BTreeMap<String, Value>,
    line: usize,
) -> Result<Value, RuntimeError> {
    match expression {
        Expr::Invoke {
            function,
            arguments,
        } => {
            let values = arguments
                .iter()
                .map(|argument| evaluate(argument, variables, line))
                .collect::<Result<Vec<_>, _>>()?;
            super::builtins::invoke(*function, values, line)
        }
        Expr::Value(value) => Ok(value.clone()),
        Expr::Variable(name) => {
            variables
                .get(name)
                .cloned()
                .ok_or_else(|| RuntimeError::Execution {
                    line,
                    message: format!("variable `{name}` has not been assigned on this path"),
                })
        }
        Expr::Unary { op, value } => {
            let value = evaluate(value, variables, line)?;
            match (op, value) {
                (UnaryOp::Negate, Value::Integer(value)) => value
                    .checked_neg()
                    .map(Value::Integer)
                    .ok_or_else(|| execution(line, "integer overflow")),
                (UnaryOp::Not, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
                (UnaryOp::Negate, value) => Err(type_error(line, "unary `-`", "integer", &value)),
                (UnaryOp::Not, value) => Err(type_error(line, "`not`", "boolean", &value)),
            }
        }
        Expr::Binary { left, op, right } => {
            let left = evaluate(left, variables, line)?;
            if *op == BinaryOp::And && left == Value::Boolean(false) {
                return Ok(Value::Boolean(false));
            }
            if *op == BinaryOp::Or && left == Value::Boolean(true) {
                return Ok(Value::Boolean(true));
            }
            let right = evaluate(right, variables, line)?;
            binary(left, *op, right, line)
        }
    }
}

fn binary(left: Value, op: BinaryOp, right: Value, line: usize) -> Result<Value, RuntimeError> {
    match op {
        BinaryOp::Add => match (left, right) {
            (Value::Integer(left), Value::Integer(right)) => left
                .checked_add(right)
                .map(Value::Integer)
                .ok_or_else(|| execution(line, "integer overflow")),
            (Value::String(mut left), Value::String(right)) => {
                left.push_str(&right);
                Ok(Value::String(left))
            }
            (left, right) => Err(binary_type_error(line, "`+`", &left, &right)),
        },
        BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
            let (Value::Integer(left), Value::Integer(right)) = (&left, &right) else {
                return Err(binary_type_error(line, "arithmetic", &left, &right));
            };
            let result = match op {
                BinaryOp::Subtract => left.checked_sub(*right),
                BinaryOp::Multiply => left.checked_mul(*right),
                BinaryOp::Divide if *right == 0 => return Err(execution(line, "division by zero")),
                BinaryOp::Divide => left.checked_div(*right),
                _ => unreachable!(),
            };
            result
                .map(Value::Integer)
                .ok_or_else(|| execution(line, "integer overflow"))
        }
        BinaryOp::Equal => Ok(Value::Boolean(left == right)),
        BinaryOp::NotEqual => Ok(Value::Boolean(left != right)),
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            let ordering = match (&left, &right) {
                (Value::Integer(left), Value::Integer(right)) => left.cmp(right),
                (Value::String(left), Value::String(right)) => left.cmp(right),
                _ => return Err(binary_type_error(line, "comparison", &left, &right)),
            };
            let result = match op {
                BinaryOp::Less => ordering.is_lt(),
                BinaryOp::LessEqual => ordering.is_le(),
                BinaryOp::Greater => ordering.is_gt(),
                BinaryOp::GreaterEqual => ordering.is_ge(),
                _ => unreachable!(),
            };
            Ok(Value::Boolean(result))
        }
        BinaryOp::And | BinaryOp::Or => match (left, right) {
            (Value::Boolean(left), Value::Boolean(right)) => {
                Ok(Value::Boolean(if op == BinaryOp::And {
                    left && right
                } else {
                    left || right
                }))
            }
            (left, right) => Err(binary_type_error(line, "boolean operation", &left, &right)),
        },
    }
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

fn type_error(line: usize, operation: &str, expected: &str, found: &Value) -> RuntimeError {
    execution(
        line,
        format!(
            "{operation} expects {expected}, found {}",
            found.type_name()
        ),
    )
}

fn binary_type_error(line: usize, operation: &str, left: &Value, right: &Value) -> RuntimeError {
    execution(
        line,
        format!(
            "{operation} cannot combine {} and {}",
            left.type_name(),
            right.type_name()
        ),
    )
}
