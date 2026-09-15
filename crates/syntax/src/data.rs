pub use velin_syntax::Builtin;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Value;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[test]
    fn names_and_argument_counts_match_the_data_builtins() {
        assert_eq!(Builtin::named("list"), Some(Builtin::List));
        assert_eq!(Builtin::named("contains"), Some(Builtin::Contains));
        assert_eq!(Builtin::named("random"), Some(Builtin::Random));
        assert_eq!(Builtin::named("unknown"), None);
        assert!(Builtin::List.accepts(0));
        assert!(Builtin::Record.accepts(2));
        assert!(!Builtin::Record.accepts(1));
        assert!(Builtin::Get.accepts(3));
        assert!(!Builtin::Len.accepts(2));
    }

    #[test]
    fn data_budget_rejects_wide_deep_and_heavy_values() {
        Value::Integer(1).validate_data().unwrap();
        let wide = Value::List(Arc::new(vec![Value::Integer(0); 4096]));
        assert!(wide.validate_data().is_err());
        let mut nested = Value::Integer(1);
        for _ in 0..17 {
            nested = Value::List(Arc::new(vec![nested]));
        }
        assert!(nested.validate_data().is_err());
        let heavy = Value::Record(Arc::new(BTreeMap::from([(
            "x".repeat(1024 * 1024 + 1),
            Value::Boolean(true),
        )])));
        assert_eq!(heavy.validate_data(), Err("data text exceeds 1 MiB"));
    }
}
