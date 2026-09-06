use renrs_syntax::syntax::Value;
use rhai::{AST, Dynamic, Engine, Scope};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub struct Extensions {
    engine: Option<Arc<Engine>>,
    scripts: BTreeMap<String, AST>,
}

impl std::fmt::Debug for Extensions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Extensions")
            .field("names", &self.scripts.keys())
            .finish_non_exhaustive()
    }
}

impl Extensions {
    /// Compiles deterministic extension programs with bounded execution and data.
    /// # Errors
    /// Rejects invalid scripts, names and source budgets.
    pub fn new(sources: &BTreeMap<String, String>) -> Result<Self, String> {
        if sources.is_empty() {
            return Ok(Self {
                engine: None,
                scripts: BTreeMap::new(),
            });
        }
        if sources.len() > 64 || sources.values().map(String::len).sum::<usize>() > 1024 * 1024 {
            return Err("extensions exceed 64 programs or 1 MiB of source".to_owned());
        }
        let mut engine = Engine::new();
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(32);
        engine.set_max_expr_depths(32, 32);
        engine.set_max_array_size(4096);
        engine.set_max_map_size(4096);
        engine.set_max_string_size(1024 * 1024);
        for symbol in ["import", "export", "eval"] {
            engine.disable_symbol(symbol);
        }
        engine.on_print(|_| {});
        engine.on_debug(|_, _, _| {});
        let scripts = sources
            .iter()
            .map(|(name, source)| {
                if name.is_empty()
                    || !name.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-')
                    })
                {
                    return Err(format!("invalid extension name: {name}"));
                }
                engine
                    .compile(source)
                    .map(|ast| (name.clone(), ast))
                    .map_err(|error| format!("extension {name}: {error}"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            engine: Some(Arc::new(engine)),
            scripts,
        })
    }

    /// Invokes a fresh scope containing only immutable input; the result is story data.
    /// # Errors
    /// Reports missing extensions, execution limits and invalid result types.
    /// # Panics
    /// Panics if the extension engine has not been initialized.
    pub fn invoke(&self, name: &str, input: &Value) -> Result<Value, String> {
        input.validate_data().map_err(str::to_owned)?;
        let ast = self
            .scripts
            .get(name)
            .ok_or_else(|| format!("unknown extension: {name}"))?;
        let mut scope = Scope::new();
        scope.push_constant(
            "input",
            rhai::serde::to_dynamic(input).map_err(|error| error.to_string())?,
        );
        let output: Dynamic = self
            .engine
            .as_ref()
            .expect("registered extension engine")
            .eval_ast_with_scope(&mut scope, ast)
            .map_err(|error| format!("extension {name}: {error}"))?;
        let output: Value =
            rhai::serde::from_dynamic(&output).map_err(|error| error.to_string())?;
        output.validate_data().map_err(str::to_owned)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn computes_data_without_leaking_scope_between_invocations() {
        let extensions = Extensions::new(&BTreeMap::from([(
            "reward".into(),
            "let score = input; score * 2 + 1".into(),
        )]))
        .unwrap();
        for _ in 0..2 {
            assert_eq!(
                extensions.invoke("reward", &Value::Integer(21)).unwrap(),
                Value::Integer(43)
            );
        }
    }
    #[test]
    fn rejects_io_imports_unbounded_execution_and_invalid_result_types() {
        assert!(
            Extensions::new(&BTreeMap::from([(
                "bad".into(),
                "import \"secrets\" as secrets;".into()
            )]))
            .is_err()
        );
        let extensions = Extensions::new(&BTreeMap::from([
            ("loop".into(), "loop {}".into()),
            ("unit".into(), "()".into()),
        ]))
        .unwrap();
        assert!(extensions.invoke("loop", &Value::Integer(0)).is_err());
        assert!(extensions.invoke("unit", &Value::Integer(0)).is_err());
    }
}
