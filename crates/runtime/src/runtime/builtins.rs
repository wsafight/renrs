use super::{RuntimeError, Value, value::execution};
use crate::syntax::Builtin;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) fn invoke(
    function: Builtin,
    mut arguments: Vec<Value>,
    line: usize,
) -> Result<Value, RuntimeError> {
    if !function.accepts(arguments.len()) {
        return Err(execution(line, "invalid built-in argument count"));
    }
    let result = match function {
        Builtin::List => Value::List(Arc::new(arguments)),
        Builtin::Record => {
            let mut record = BTreeMap::new();
            let mut arguments = arguments.into_iter();
            while let Some(key) = arguments.next() {
                let Value::String(key) = key else {
                    return Err(execution(line, "record keys must be strings"));
                };
                if record.insert(key, arguments.next().unwrap()).is_some() {
                    return Err(execution(line, "duplicate record key"));
                }
            }
            Value::Record(Arc::new(record))
        }
        Builtin::Len => Value::Integer(
            i64::try_from(match &arguments[0] {
                Value::List(values) => values.len(),
                Value::Record(values) => values.len(),
                Value::String(value) => value.chars().count(),
                _ => return Err(execution(line, "len expects a list, record or string")),
            })
            .map_err(|_| execution(line, "length overflow"))?,
        ),
        Builtin::Get => {
            let value = match (&arguments[0], &arguments[1]) {
                (Value::List(values), Value::Integer(index)) => usize::try_from(*index)
                    .ok()
                    .and_then(|index| values.get(index)),
                (Value::Record(values), Value::String(key)) => values.get(key),
                _ => {
                    return Err(execution(
                        line,
                        "get expects a list and integer index, or a record and string key",
                    ));
                }
            };
            value
                .or(arguments.get(2))
                .cloned()
                .ok_or_else(|| execution(line, "missing key or list index"))?
        }
        Builtin::Contains => Value::Boolean(match (&arguments[0], &arguments[1]) {
            (Value::List(values), value) => values.contains(value),
            (Value::Record(values), Value::String(key)) => values.contains_key(key),
            (Value::String(text), Value::String(part)) => text.contains(part),
            _ => return Err(execution(line, "contains expects a list, record or string")),
        }),
        Builtin::Push => {
            let value = arguments.pop().unwrap();
            let Value::List(mut values) = arguments.pop().unwrap() else {
                return Err(execution(line, "push expects a list"));
            };
            Arc::make_mut(&mut values).push(value);
            Value::List(values)
        }
        Builtin::Put | Builtin::Remove => edit(function, arguments, line)?,
    };
    result
        .validate_data()
        .map_err(|error| execution(line, error))?;
    Ok(result)
}

fn edit(function: Builtin, mut arguments: Vec<Value>, line: usize) -> Result<Value, RuntimeError> {
    let replacement = (function == Builtin::Put).then(|| arguments.pop().unwrap());
    let key = arguments.pop().unwrap();
    match (arguments.pop().unwrap(), key) {
        (Value::List(mut values), Value::Integer(index)) => {
            let index = usize::try_from(index)
                .ok()
                .filter(|index| *index < values.len())
                .ok_or_else(|| execution(line, "list index out of bounds"))?;
            if let Some(value) = replacement {
                Arc::make_mut(&mut values)[index] = value;
            } else {
                Arc::make_mut(&mut values).remove(index);
            }
            Ok(Value::List(values))
        }
        (Value::Record(mut values), Value::String(key)) => {
            if let Some(value) = replacement {
                Arc::make_mut(&mut values).insert(key, value);
            } else if Arc::make_mut(&mut values).remove(&key).is_none() {
                return Err(execution(line, "missing record key"));
            }
            Ok(Value::Record(values))
        }
        _ => Err(execution(
            line,
            "put/remove expects a list and integer index, or a record and string key",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Runtime, compile, parse_script};
    #[test]
    fn inventory_is_independent_and_survives_save_and_rollback() {
        let program = compile(&parse_script("default bag = list(\"key\")\ndefault copy = bag\ndefault quest = record(\"done\", false)\nlabel start:\n    \"Ready\"\n    set bag = push(bag, \"map\")\n    set quest = put(quest, \"done\", contains(bag, \"map\"))\n    \"Collected\"\n    set bag = remove(bag, 0)\n    \"Used\"\n", "inventory.rns").unwrap()).unwrap();
        let mut runtime = Runtime::new(program.clone()).unwrap();
        runtime.advance().unwrap();
        runtime.continue_story().unwrap();
        assert_eq!(
            runtime.variables()["copy"],
            Value::List(Arc::new(vec![Value::String("key".into())]))
        );
        let saved = serde_json::to_string(&runtime.snapshot()).unwrap();
        let mut restored =
            Runtime::restore(program, serde_json::from_str(&saved).unwrap()).unwrap();
        assert_eq!(restored.variables()["bag"], runtime.variables()["bag"]);
        restored.continue_story().unwrap();
        restored.rollback().unwrap();
        assert_eq!(restored.variables()["bag"], runtime.variables()["bag"]);
        assert_eq!(
            restored.variables()["quest"],
            Value::Record(Arc::new(BTreeMap::from([(
                "done".into(),
                Value::Boolean(true)
            )])))
        );
    }
    #[test]
    fn invalid_calls_and_collection_growth_are_bounded() {
        assert!(parse_script("label start:\n    set x = exec(\"command\")\n", "bad.rns").is_err());
        assert!(
            invoke(
                Builtin::Get,
                vec![Value::List(Arc::new(vec![])), Value::Integer(-1)],
                1
            )
            .is_err()
        );
        let list = Value::List(Arc::new(vec![Value::Integer(0); 4095]));
        assert!(invoke(Builtin::Push, vec![list, Value::Integer(1)], 1).is_err());
        assert!(
            invoke(
                Builtin::Record,
                vec![Value::Integer(1), Value::Boolean(true)],
                1
            )
            .is_err()
        );
    }
}
